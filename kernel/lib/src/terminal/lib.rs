use crate::asm::global::{call_app, get_cr3, set_cr3};
use crate::elf::Elf64Ehdr;
use crate::error::{Code, Error};
use crate::fat::global::{boot_volume_image, create_file, find_file};
use crate::fat::{Attribute, DirectoryEntry, FatFileDescriptor, END_OF_CLUSTER_CHAIN};
use crate::graphics::global::frame_buffer_config;
use crate::graphics::{
    draw_text_box_with_colors, PixelColor, PixelWriter, Rectangle, Vector2D, COLOR_BLACK,
};
use crate::io::{FileDescriptor, STD_ERR, STD_IN, STD_OUT};
use crate::layer::global::layer_manager;
use crate::layer::LayerID;
use crate::libc::{memcpy, strcpy, strlen};
use crate::memory_manager::global::MEMORY_MANAGER;
use crate::message::MessageType::Layer;
use crate::message::{LayerMessage, LayerOperation, Message, MessageType, WindowActiveMode};
use crate::paging::global::{copy_page_maps, free_page_map, reset_cr3};
use crate::paging::{LinearAddress4Level, PageMapEntry};
use crate::pci::devices;
use crate::rust_official::c_str::CString;
use crate::rust_official::cchar::c_char;
use crate::sync::Mutex;
use crate::sync::MutexGuard;
use crate::task::global::{main_task_id, task_manager};
use crate::task::{Task, TaskID};
use crate::terminal::file_descriptor::{
    PipeDescriptor, TerminalDescriptor, TerminalFileDescriptor,
};
use crate::terminal::history::{CommandHistory, Direction};
use crate::terminal::terminal_writer::{TerminalWriter, TERMINAL_WRITERS};
use crate::timer::global::{current_tick, do_with_timer_manager};
use crate::timer::{Timer, TIMER_FREQ};
use crate::window::{TITLED_WINDOW_BOTTOM_RIGHT_MARGIN, TITLED_WINDOW_TOP_LEFT_MARGIN};
use crate::{make_error, str_trimming_nul_unchecked, Window};
use alloc::boxed::Box;
use alloc::collections::{BTreeMap, VecDeque};
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::{format, vec};
use core::arch::asm;
use core::cell::{RefCell, RefMut};
use core::ffi::c_void;
use core::fmt::Write;
use core::mem;
use core::ops::DerefMut;
use shared::PixelFormat;

static APP_LOADS: Mutex<BTreeMap<usize, AppLoadInfo>> = Mutex::new(BTreeMap::new());
fn insert_app_load(
    app_loads: &mut BTreeMap<usize, AppLoadInfo>,
    e: &DirectoryEntry,
    app_load: AppLoadInfo,
) {
    app_loads.insert(e as *const _ as usize, app_load);
}

pub fn task_terminal(task_id: u64, data: usize) {
    let td = {
        let ptr = data as *mut usize as *mut TerminalDescriptor;
        if ptr.is_null() {
            None
        } else {
            let b = unsafe { Box::from_raw(data as *mut TerminalDescriptor) };
            Some(*b)
        }
    };
    let term_desc = td.as_ref();
    let command = term_desc.map(|td| td.command_line.as_str()).unwrap_or("");
    let show_window = term_desc.is_none();

    let task_id = TaskID::new(task_id);
    let mut terminal = create_terminal(task_id, term_desc, show_window);

    if !show_window {
        for c in command.chars() {
            terminal.input_key(0, 0, c);
        }
        terminal.input_key(0, 0, '\n');
    }

    if let Some(fd) = term_desc {
        if fd.exit_after_command {
            unsafe { asm!("cli") };
            task_manager().finish(terminal.last_exit_code);
            unsafe { asm!("sti") };
        }
    }

    let add_blink_timer = |t: u64| {
        do_with_timer_manager(|fm| fm.add_timer(Timer::new(t + TIMER_FREQ / 2, 1, task_id)))
    };
    add_blink_timer(current_tick());
    let mut active_mode = WindowActiveMode::Deactivate;

    loop {
        unsafe { asm!("cli") };
        let current_task_id = task_manager().current_task().id();
        let msg = task_manager()
            .get_task_mut(current_task_id)
            .unwrap()
            .receive_message();
        if msg.is_none() {
            task_manager().sleep(current_task_id).unwrap();
            unsafe { asm!("sti") };
            continue;
        };
        unsafe { asm!("sti") };

        let msg = msg.unwrap();
        match msg.m_type {
            MessageType::TimerTimeout { timeout, value: _ } => {
                add_blink_timer(timeout);
                if show_window && active_mode == WindowActiveMode::Activate {
                    let area = terminal.blink_cursor();

                    let msg = Message::new(Layer(LayerMessage {
                        layer_id: terminal.layer_id,
                        op: LayerOperation::DrawArea(area),
                        src_task_id: task_id,
                    }));
                    unsafe { asm!("cli") };
                    task_manager().send_message(main_task_id(), msg).unwrap();
                    unsafe { asm!("sti") };
                }
            }
            MessageType::KeyPush(arg) => {
                if !arg.press {
                    continue;
                }
                let area = terminal.input_key(arg.modifier, arg.keycode, arg.ascii);
                if show_window {
                    let msg = Message::new(Layer(LayerMessage {
                        layer_id: terminal.layer_id,
                        op: LayerOperation::DrawArea(area),
                        src_task_id: task_id,
                    }));
                    unsafe { asm!("cli") };
                    task_manager().send_message(main_task_id(), msg).unwrap();
                    unsafe { asm!("sti") };
                }
            }
            MessageType::WindowActive(mode) => active_mode = mode,
            MessageType::WindowClose(message) => {
                let _ = layer_manager().lock().close_layer(message.layer_id);
                unsafe { asm!("cli") };
                task_manager().finish(terminal.last_exit_code);
            }
            _ => {}
        }
    }
}

fn create_terminal(
    task_id: TaskID,
    term_desc: Option<&TerminalDescriptor>,
    show_window: bool,
) -> Terminal {
    let mut terminal = Terminal::new(task_id, term_desc);
    terminal.initialize(show_window, frame_buffer_config().pixel_format);

    if show_window {
        let mut lm = layer_manager().lock();
        lm.move_(terminal.layer_id, Vector2D::new(100, 200));
        lm.register_layer_task_relation(terminal.layer_id, task_id);
        lm.activate_layer(Some(terminal.layer_id));
    }
    terminal
}

#[derive(Clone)]
pub(crate) struct AppLoadInfo {
    vaddr_end: u64,
    entry: u64,
    pml4: *const PageMapEntry,
}

unsafe impl Send for AppLoadInfo {}

impl AppLoadInfo {
    fn new(vaddr_end: u64, entry: u64, pml4: *mut PageMapEntry) -> Self {
        Self {
            vaddr_end,
            entry,
            pml4,
        }
    }
}

pub(super) const ROWS: usize = 15;
pub(super) const COLUMNS: usize = 60;
pub(super) const LINE_MAX: usize = 128;

pub(crate) struct Terminal {
    task_id: TaskID,
    layer_id: LayerID,
    line_buf: String,
    command_history: CommandHistory,
    files: [Rc<RefCell<FileDescriptor>>; STD_ERR + 1],
    last_exit_code: i32,
}

impl Terminal {
    fn new(task_id: TaskID, terminal_desc: Option<&TerminalDescriptor>) -> Terminal {
        let files = if let Some(td) = terminal_desc {
            td.files.clone()
        } else {
            [
                Rc::new(RefCell::new(FileDescriptor::Terminal(
                    TerminalFileDescriptor::new(task_id),
                ))),
                Rc::new(RefCell::new(FileDescriptor::Terminal(
                    TerminalFileDescriptor::new(task_id),
                ))),
                Rc::new(RefCell::new(FileDescriptor::Terminal(
                    TerminalFileDescriptor::new(task_id),
                ))),
            ]
        };

        Self {
            task_id,
            layer_id: LayerID::MAX,
            line_buf: String::with_capacity(LINE_MAX),
            command_history: CommandHistory::new(),
            files,
            last_exit_code: 0,
        }
    }

    pub(crate) fn layer_id(&self) -> LayerID {
        self.layer_id
    }

    pub(crate) fn task_id(&self) -> TaskID {
        self.task_id
    }

    fn initialize(&mut self, show_window: bool, pixel_format: PixelFormat) {
        let window = if show_window {
            let mut window = Window::new_with_title(
                COLUMNS * 8 + 8 + Window::TITLED_WINDOW_MARGIN.x as usize,
                ROWS * 16 + 8 + Window::TITLED_WINDOW_MARGIN.y as usize,
                pixel_format,
                "MikanTerm",
            );
            let inner_size = window.inner_size();
            draw_terminal(&mut window, Vector2D::new(0, 0), inner_size);

            let window = Arc::new(Mutex::new(window));
            self.layer_id = layer_manager()
                .lock()
                .new_layer(Arc::clone(&window))
                .set_draggable(true)
                .id();
            Some(window)
        } else {
            None
        };

        unsafe {
            TERMINAL_WRITERS.register(
                self.task_id,
                TerminalWriter::new(self.layer_id, self.task_id, window),
            );
        }

        if show_window {
            self.print(">")
        }
    }

    fn blink_cursor(&mut self) -> Rectangle<i32> {
        self.writer().blink_cursor()
    }

    fn draw_cursor(&mut self, visible: bool) {
        self.writer().draw_cursor(visible)
    }

    fn input_key(&mut self, _modifier: u8, keycode: u8, ascii: char) -> Rectangle<i32> {
        self.draw_cursor(false);

        let mut draw_area = Rectangle::new(self.calc_cursor_pos(), Vector2D::new(8 * 2, 16));

        match ascii {
            '\n' => {
                self.command_history.push(self.line_buf.to_string());

                self.writer().new_line();
                self.execute_line();
                self.print(">");
                draw_area.pos = TITLED_WINDOW_TOP_LEFT_MARGIN;
                draw_area.size = self.writer().window_inner_size().unwrap_or(
                    Vector2D::new(0, 0)
                        - TITLED_WINDOW_TOP_LEFT_MARGIN
                        - TITLED_WINDOW_BOTTOM_RIGHT_MARGIN,
                )
            }
            '\x08' => {
                if self.line_buf.pop().is_some() {
                    self.writer().back_space();
                    draw_area.pos = self.calc_cursor_pos();
                }
            }
            '\x00' => {
                if keycode == 0x51 {
                    draw_area = self.history_up_down(Direction::Down);
                } else if keycode == 0x52 {
                    draw_area = self.history_up_down(Direction::Up);
                }
            }
            _ => {
                if self.writer().can_write_on_this_line() && self.line_buf.len() < LINE_MAX {
                    self.line_buf.push(ascii);
                    self.writer().input_ascii(ascii);
                }
            }
        }

        self.draw_cursor(true);
        draw_area
    }

    fn calc_cursor_pos(&self) -> Vector2D<i32> {
        self.writer().calc_cursor_pos()
    }

    fn execute_line(&mut self) {
        let line_buf = mem::take(&mut self.line_buf);
        let mut argv = if let Some(argv) = parse_command(line_buf.as_str()) {
            argv
        } else {
            return;
        };
        let original_stdout = Rc::clone(&self.files[STD_OUT]);

        // handles redirect
        if let Some(redirect_dest_index) = find_redirect_dest(&argv) {
            match extract_redirect(&argv, redirect_dest_index) {
                Ok(redirect_dest_file) => {
                    self.files[STD_OUT] = Rc::new(RefCell::new(FileDescriptor::Fat(
                        FatFileDescriptor::new(redirect_dest_file),
                    )))
                }
                Err(e) => {
                    writeln!(self.stderr(), "{}", e).unwrap_or_default();
                    return;
                }
            }
            argv = argv[..redirect_dest_index - 1].to_vec();
        }

        let pipe_fd_for_write = if let Some(pipe_dest_index) = find_pipe_dest(&argv) {
            let sub_command = join_to_string(' ', &argv[pipe_dest_index..]);
            argv = argv[..pipe_dest_index - 1].to_vec();

            let sub_task = task_manager().new_task();
            let pipe_fd = PipeDescriptor::new(sub_task.id());
            let pipe_fd_for_write = pipe_fd.copy_for_write();

            let term_desc = TerminalDescriptor {
                command_line: sub_command,
                exit_after_command: true,
                show_window: false,
                files: [
                    Rc::new(RefCell::new(FileDescriptor::Pipe(pipe_fd))),
                    self.files[STD_OUT].clone(),
                    self.files[STD_ERR].clone(),
                ],
            };
            let b = Box::new(term_desc);
            sub_task.init_context(task_terminal, Box::into_raw(b) as u64, get_cr3);
            task_manager().wake_up(sub_task.id()).unwrap();
            layer_manager()
                .lock()
                .register_layer_task_relation(self.layer_id, sub_task.id());

            self.files[STD_OUT] = Rc::new(RefCell::new(FileDescriptor::Pipe(
                pipe_fd_for_write.copy_for_write(),
            )));
            Some(pipe_fd_for_write)
        } else {
            None
        };

        let command = match argv.first() {
            None => return, // if enters a line that starts with '>' such as '> foo'
            Some(&c) => c,
        };
        let exit_code = match command {
            "echo" => {
                if let Some(&arg) = argv.get(1) {
                    let _ = if arg == "$?" {
                        let last = self.last_exit_code;
                        write!(self.stdout(), "{}", last)
                    } else {
                        write!(self.stdout(), "{}", arg)
                    };
                }
                let _ = writeln!(self.stdout());
                0
            }
            "clear" => {
                self.writer().clear();
                0
            }
            "lspci" => {
                for device in devices() {
                    writeln!(self.stdout(), "{}", device).unwrap();
                }
                0
            }
            "ls" => self.execute_ls(&argv),
            "cat" => self.execute_cat(&argv),
            "noterm" => self.exec_noterm(&argv),
            "memstat" => self.execute_memstat(),
            _ => {
                let root_cluster = boot_volume_image().get_root_cluster();
                if let Some(file_entry) = find_command(command, root_cluster as u64) {
                    match self.execute_file(file_entry, argv.as_slice()) {
                        Ok(ec) => ec,
                        Err((ec, err)) => {
                            let _ = writeln!(self.stderr(), "failed to exec file: {}", err);
                            -ec
                        }
                    }
                } else {
                    writeln!(self.stderr(), "no such command: {}", command).unwrap();
                    1
                }
            }
        };

        if let Some(mut fd) = pipe_fd_for_write {
            fd.finish_write();
            unsafe { asm!("cli") };
            let ec = task_manager().wait_finish(fd.task_id);
            unsafe { asm!("sti") };
            layer_manager()
                .lock()
                .register_layer_task_relation(self.layer_id, self.task_id);
            self.last_exit_code = ec;
        } else {
            self.last_exit_code = exit_code;
        }

        self.files[STD_OUT] = original_stdout;
    }

    fn execute_file(
        &mut self,
        file_entry: &DirectoryEntry,
        args: &[&str],
    ) -> Result<i32, (i32, Error)> {
        unsafe { asm!("cli") };
        let task = task_manager().current_task_mut();
        unsafe { asm!("sti") };

        let app_load = self.load_app(file_entry, task).map_err(|e| (0, e))?;

        let args_frame_addr = LinearAddress4Level::new(0xffff_ffff_ffff_f000);
        PageMapEntry::setup_page_maps(args_frame_addr, 1, true, get_cr3()).map_err(|e| (0, e))?;
        let argv = args_frame_addr.value() as *mut u64 as *mut *mut c_char;
        let argv_len = 32; // argv = 8x32 = 256 bytes
        let p_p_cchar_size = mem::size_of::<*const *const c_char>();
        let argbuf =
            (args_frame_addr.value() as usize + p_p_cchar_size * argv_len) as *const c_char;
        let argbuf_len = 4096 - p_p_cchar_size * argv_len;

        let c_chars_vec = new_c_chars_vec(args);
        let argc =
            make_argv(&c_chars_vec, argv, argv_len, argbuf, argbuf_len).map_err(|e| (0, e))?;

        let stack_size = Task::DEFAULT_STACK_BYTES;
        let stack_frame_addr = LinearAddress4Level::new(0xffff_ffff_ffff_f000 - stack_size as u64);
        PageMapEntry::setup_page_maps(stack_frame_addr, stack_size / 4096, true, get_cr3())
            .map_err(|e| (0, e))?;

        // register standard in/out and error file descriptors
        for file_rc in &self.files {
            task.register_file_descriptor_rc(Rc::clone(file_rc));
        }

        let elf_next_page = (app_load.vaddr_end + 4095) & 0xffff_ffff_ffff_f000;
        task.dpaging_begin = elf_next_page;
        task.dpaging_end = elf_next_page;
        task.file_map_end = stack_frame_addr.value();

        let ret = call_app(
            argc as i32,
            argv as *const *const c_char,
            3 << 3 | 3,
            app_load.entry,
            stack_frame_addr.value() + stack_size as u64 - 8,
            task.os_stack_pointer() as *const _,
        );

        task.clear_files();
        task.clear_file_mappings();
        // retake pointers to free memory
        for c_arg in c_chars_vec {
            let _ = unsafe { CString::from_raw(c_arg as *mut c_char) };
        }

        PageMapEntry::clean_page_maps(LinearAddress4Level::new(0xffff_8000_0000_0000), get_cr3())
            .map_err(|e| (ret, e))?;

        free_pml4(task).map(|_| ret).map_err(|e| (ret, e))
    }

    fn load_app(
        &mut self,
        file_entry: &DirectoryEntry,
        task: &mut Task,
    ) -> Result<AppLoadInfo, Error> {
        // hold the lock until copy is completed
        let mut loads = APP_LOADS.lock();

        if let Some(original) = loads.get_mut(&(file_entry as *const _ as usize)) {
            let temp_pml4 = setup_pml4(task)?;
            copy_page_maps(temp_pml4, original.pml4, 4, 256)?;
            let cloned = AppLoadInfo::new(original.vaddr_end, original.entry, temp_pml4);
            return Ok(cloned);
        }

        let temp_pml4 = setup_pml4(task)?;

        let mut file_buf: Vec<u8> = vec![0; file_entry.file_size() as usize];
        file_entry.load_file(file_buf.as_mut_slice(), boot_volume_image());

        let elf_header = unsafe { Elf64Ehdr::from_mut(&mut file_buf) }.unwrap();
        if !elf_header.is_elf() {
            return Err(make_error!(Code::InvalidFile));
        }

        let elf_last_addr = elf_header.load_elf(get_cr3())?;
        let mut app_load = AppLoadInfo::new(elf_last_addr, elf_header.e_entry as u64, temp_pml4);
        insert_app_load(&mut *loads, file_entry, app_load.clone());

        app_load.pml4 = setup_pml4(task)?;
        copy_page_maps(app_load.pml4 as *mut _, temp_pml4, 4, 256)?;
        Ok(app_load)
    }

    pub(crate) fn print(&mut self, s: &str) {
        self.writer().print(s)
    }

    fn print_char(&mut self, c: char) {
        self.writer().print_char(c)
    }

    pub(super) fn redraw(&mut self) {
        self.writer().redraw()
    }

    fn history_up_down(&mut self, direction: Direction) -> Rectangle<i32> {
        self.line_buf = match direction {
            Direction::Up => self.command_history.up().to_string(),
            Direction::Down => self.command_history.down().to_string(),
        };

        self.writer().history_up_down(self.line_buf.as_str())
    }

    fn execute_ls(&mut self, argv: &[&str]) -> i32 {
        let root_cluster = boot_volume_image().get_root_cluster();

        let first_arg = argv.get(1);
        if first_arg.is_none() {
            list_all_entries(self.stdout(), root_cluster);
            return 0;
        }

        let &first_arg = first_arg.unwrap();
        let (dir, post_slash) = find_file(first_arg, root_cluster.into());
        if dir.is_none() {
            writeln!(self.stderr(), "No such file or directory: {}", first_arg).unwrap();
            return 1;
        }

        let dir = dir.unwrap();
        if dir.is_directory() {
            list_all_entries(self.stdout(), dir.first_cluster());
            return 1;
        }

        let name_bytes = dir.formatted_name();
        let name = str_trimming_nul_unchecked(&name_bytes);

        if post_slash {
            writeln!(self.stderr(), "{} is not a directory", name).unwrap();
            1
        } else {
            writeln!(self.stdout(), "{}", name).unwrap();
            0
        }
    }

    fn execute_cat(&mut self, argv: &[&str]) -> i32 {
        let bpb = boot_volume_image();
        let fd = if let Some(first_arg) = argv.get(1) {
            let (file_entry, post_slash) = find_file(first_arg, bpb.get_root_cluster() as u64);
            if file_entry.is_none() {
                writeln!(self.stderr(), "no such file: {}", first_arg).unwrap();
                return 1;
            }

            let file_entry = file_entry.unwrap();
            if !file_entry.is_directory() && post_slash {
                let name_bytes = file_entry.formatted_name();
                let name = str_trimming_nul_unchecked(&name_bytes);
                writeln!(self.stderr(), "{} is not a directory", name).unwrap();
                return 1;
            }

            Rc::new(RefCell::new(FileDescriptor::Fat(FatFileDescriptor::new(
                file_entry,
            ))))
        } else {
            Rc::clone(&self.files[STD_IN])
        };

        let mut u8buf = [0; 1024];
        self.draw_cursor(false);
        loop {
            if fd.borrow_mut().read_delim(b'\n', &mut u8buf) == 0 {
                break;
            }
            let str = str_trimming_nul_unchecked(&u8buf);
            write!(self.stdout(), "{}", str).unwrap();
        }
        self.draw_cursor(true);
        0
    }

    fn execute_memstat(&mut self) -> i32 {
        let p_stat = MEMORY_MANAGER.lock().stat();
        writeln!(
            self.stdout(),
            "Phys used : {} frames ({} MiB)\nPhys total: {} frames ({} MiB)",
            p_stat.allocated_frames,
            p_stat.calc_allocated_size_in_mb(),
            p_stat.total_frames,
            p_stat.calc_total_size_in_mb(),
        )
        .unwrap();
        1
    }

    fn exec_noterm(&mut self, first_arg: &[&str]) -> i32 {
        let first_arg = match first_arg.get(1) {
            None => return 0,
            Some(&f) => f,
        };

        let term_dec = TerminalDescriptor {
            command_line: first_arg.to_string(),
            exit_after_command: true,
            show_window: false,
            files: self.files.clone(),
        };
        let b = Box::new(term_dec);
        let task_id = task_manager()
            .new_task()
            .init_context(task_terminal, Box::into_raw(b) as u64, get_cr3)
            .id();
        task_manager().wake_up(task_id).unwrap();
        0
    }

    fn stdout(&mut self) -> RefMut<'_, FileDescriptor> {
        self.files[STD_OUT].borrow_mut()
    }

    fn stderr(&mut self) -> RefMut<'_, FileDescriptor> {
        self.files[STD_ERR].borrow_mut()
    }

    fn writer(&self) -> MutexGuard<TerminalWriter> {
        unsafe { TERMINAL_WRITERS.get(self.task_id).lock() }
    }
}

fn list_all_entries<T: DerefMut<Target = FileDescriptor>>(mut fd: T, dir_cluster: u32) {
    let mut dir_cluster = dir_cluster as u64;

    while dir_cluster != END_OF_CLUSTER_CHAIN {
        let dirs = boot_volume_image().get_sector_by_cluster::<DirectoryEntry>(dir_cluster);
        for dir in dirs {
            if dir.is_free_and_no_more_allocated_after_this() {
                break;
            }
            if dir.is_free() || dir.attr() == Some(Attribute::LongName) {
                continue;
            }

            let name = dir.formatted_name();
            writeln!(fd, "{}", str_trimming_nul_unchecked(&name)).unwrap();
        }
        dir_cluster = boot_volume_image().next_cluster(dir_cluster);
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        unsafe { TERMINAL_WRITERS.remove(self.task_id) };
    }
}

fn draw_terminal<W: PixelWriter>(w: &mut W, pos: Vector2D<i32>, size: Vector2D<i32>) {
    draw_text_box_with_colors(
        w,
        pos,
        size,
        &COLOR_BLACK,
        &PixelColor::from(0xc6c6c6),
        &PixelColor::from(0x848484),
    );
}

fn parse_command(s: &str) -> Option<Vec<&str>> {
    let parsed = s.split_whitespace().collect::<VecDeque<_>>();
    if parsed.is_empty() {
        return None;
    }

    Some(Vec::from(parsed))
}

fn make_argv(
    c_chars_slice: &[*const c_char],
    argv: *mut *mut c_char,
    argv_len: usize,
    argbuf: *const c_char,
    argbuf_len: usize,
) -> Result<usize, Error> {
    let mut argc = 0;
    let mut argbuf_index = 0;

    let mut push_to_argv = |s: *const c_char| {
        if argc >= argv_len || argbuf_index >= argbuf_len {
            Err(make_error!(Code::Full))
        } else {
            let dst = unsafe { argbuf.add(argbuf_index) } as *mut c_char;
            unsafe { *argv.add(argc) = dst }
            argc += 1;
            unsafe { strcpy(dst, s) };
            argbuf_index += unsafe { strlen(s) } + 1;
            Ok(())
        }
    };

    for &c_chars in c_chars_slice {
        push_to_argv(c_chars)?;
    }

    Ok(argc)
}

fn find_redirect_dest(argv: &[&str]) -> Option<usize> {
    match argv.iter().position(|&x| x == ">") {
        None => None,
        Some(i) => argv.get(i + 1).map(|_| i + 1),
    }
}

fn extract_redirect<'a>(
    argv: &'a [&str],
    redirect_dest_index: usize,
) -> Result<&'a DirectoryEntry, String> {
    let redirect_dest = argv[redirect_dest_index];
    let (file, post_slash) =
        find_file(redirect_dest, boot_volume_image().get_root_cluster() as u64);
    if let Some(file) = file {
        if file.is_directory() || post_slash {
            Err(format!("cannot redirect to a directory: {}", redirect_dest))
        } else {
            Ok(file)
        }
    } else {
        create_file(redirect_dest).map_err(|e| format!("failed to create a redirect file: {}", e))
    }
}

fn find_pipe_dest(argv: &[&str]) -> Option<usize> {
    match argv.iter().position(|&s| s == "|") {
        None => None,
        Some(i) => argv.get(i + 1).map(|_| i + 1),
    }
}

fn join_to_string(separator: char, strs: &[&str]) -> String {
    strs.iter().fold("".to_string(), |mut acc, &s| {
        acc.push_str(s);
        acc.push(separator);
        acc
    })
}

fn new_c_chars_vec(strs: &[&str]) -> Vec<*const c_char> {
    strs.iter()
        .map(|&s| CString::_new(s.as_bytes().to_vec()).unwrap())
        .map(|c| c.into_raw() as *const c_char)
        .collect::<Vec<_>>()
}

pub fn setup_pml4(current_task: &mut Task) -> Result<*mut PageMapEntry, Error> {
    let pml4 = PageMapEntry::new_page_map()?;

    let current_pml4 = get_cr3() as *const u64 as *const PageMapEntry;
    unsafe {
        memcpy(
            pml4 as *mut c_void,
            current_pml4 as *const c_void,
            256 * mem::size_of::<u64>(),
        );
    }

    let cr3 = pml4 as usize as u64;
    set_cr3(cr3);
    current_task.set_cr3(cr3);
    Ok(pml4)
}

pub fn free_pml4(current_task: &mut Task) -> Result<(), Error> {
    let cr3 = current_task.get_cr3();
    current_task.set_cr3(0);
    reset_cr3();
    free_page_map(cr3 as *mut u64 as *mut PageMapEntry)
}

fn find_command(command: &str, dir_cluster: u64) -> Option<&DirectoryEntry> {
    if let (Some(file_entry), post_slash) = find_file(command, dir_cluster) {
        if file_entry.is_directory() && post_slash {
            return None;
        }
        return Some(file_entry);
    }

    let root_cluster = boot_volume_image().get_root_cluster() as u64;
    if dir_cluster != root_cluster || command.contains('/') {
        return None;
    }

    let apps_entry = match find_file("apps", root_cluster).0 {
        None => return None,
        Some(apps_entry) => {
            if !apps_entry.is_directory() {
                return None;
            }
            apps_entry
        }
    };

    find_command(command, apps_entry.first_cluster() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn parse_command_empty() {
        assert_eq!(parse_command(""), None);
    }

    #[test]
    fn parse_command_no_args() {
        assert_eq!(parse_command("echo"), Some(vec!["echo"]));
    }

    #[test]
    fn parse_command_one_arg() {
        assert_eq!(parse_command("echo a\\aa"), Some(vec!["echo", "a\\aa"]));
    }

    #[test]
    fn parse_command_args() {
        assert_eq!(
            parse_command("ls -l | sort"),
            Some(vec!["ls", "-l", "|", "sort"])
        );
    }
}
