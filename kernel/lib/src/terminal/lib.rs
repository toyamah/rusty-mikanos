use crate::asm::global::{call_app, get_cr3, set_cr3};
use crate::elf::Elf64Ehdr;
use crate::error::{Code, Error};
use crate::fat::global::{boot_volume_image, create_file, find_file};
use crate::fat::{Attribute, DirectoryEntry, FatFileDescriptor, END_OF_CLUSTER_CHAIN};
use crate::font::{convert_utf8_to_u32, count_utf8_size, write_ascii, write_string, write_unicode};
use crate::graphics::global::frame_buffer_config;
use crate::graphics::{
    draw_text_box_with_colors, fill_rectangle, PixelColor, PixelWriter, Rectangle, Vector2D,
    COLOR_BLACK, COLOR_WHITE,
};
use crate::io::{FileDescriptor, STD_ERR, STD_OUT};
use crate::layer::global::{active_layer, layer_manager, layer_task_map, screen_frame_buffer};
use crate::layer::{LayerID, LayerManager};
use crate::libc::{memcpy, strcpy};
use crate::memory_manager::global::memory_manager;
use crate::message::{LayerMessage, LayerOperation, Message, MessageType, WindowActiveMode};
use crate::paging::global::{copy_page_maps, free_page_map, reset_cr3};
use crate::paging::{LinearAddress4Level, PageMapEntry};
use crate::pci::devices;
use crate::rust_official::c_str::CString;
use crate::rust_official::cchar::c_char;
use crate::rust_official::strlen;
use crate::task::global::task_manager;
use crate::task::{Task, TaskID};
use crate::terminal::file_descriptor::TerminalFileDescriptor;
use crate::terminal::history::{CommandHistory, Direction};
use crate::timer::global::timer_manager;
use crate::timer::{Timer, TIMER_FREQ};
use crate::window::{TITLED_WINDOW_BOTTOM_RIGHT_MARGIN, TITLED_WINDOW_TOP_LEFT_MARGIN};
use crate::{make_error, str_trimming_nul_unchecked, Window};
use alloc::collections::{BTreeMap, VecDeque};
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::{format, vec};
use core::arch::asm;
use core::cell::{RefCell, RefMut};
use core::ffi::c_void;
use core::fmt::Write;
use core::ops::{Deref, DerefMut};
use core::{fmt, mem};
use shared::PixelFormat;

static mut TERMINALS: BTreeMap<TaskID, Terminal> = BTreeMap::new();
pub(crate) fn get_terminal_mut_by(task_id: TaskID) -> Option<&'static mut Terminal> {
    unsafe { TERMINALS.get_mut(&task_id) }
}

static mut APP_LOADS: BTreeMap<usize, AppLoadInfo> = BTreeMap::new();
pub(super) fn get_app_load_ref(e: &DirectoryEntry) -> Option<&'static AppLoadInfo> {
    unsafe { APP_LOADS.get(&(e as *const _ as usize)) }
}
pub(super) fn get_app_load_mut(e: &DirectoryEntry) -> Option<&'static mut AppLoadInfo> {
    unsafe { APP_LOADS.get_mut(&(e as *const _ as usize)) }
}
pub(super) fn insert_app_load(e: &DirectoryEntry, app_load: AppLoadInfo) {
    unsafe { APP_LOADS.insert(e as *const _ as usize, app_load) };
}

pub fn task_terminal(task_id: u64, command: usize) {
    let command = {
        let ptr = command as *const usize as *const c_char;
        if ptr.is_null() {
            "".to_string()
        } else {
            // use CString to free memory
            let c_string = unsafe { CString::from_raw(ptr as *mut c_char) };
            String::from_utf8(c_string.into_bytes()).unwrap()
        }
    };
    let show_window = command.is_empty();

    unsafe { asm!("cli") };
    let task_id = TaskID::new(task_id);
    let current_task_id = task_manager().current_task().id();
    {
        // Initialize Terminal
        let mut terminal = Terminal::new(task_id);
        terminal.initialize(
            show_window,
            layer_manager(),
            frame_buffer_config().pixel_format,
        );
        if show_window {
            layer_manager().move_(
                terminal.layer_id,
                Vector2D::new(100, 200),
                screen_frame_buffer(),
            );
            layer_task_map().insert(terminal.layer_id, task_id);
            active_layer().activate(
                Some(terminal.layer_id),
                layer_manager(),
                screen_frame_buffer(),
                task_manager(),
                layer_task_map(),
            );
        }
        unsafe { TERMINALS.insert(task_id, terminal) };
    }
    unsafe { asm!("sti") };

    let terminal = || unsafe { TERMINALS.get_mut(&task_id).expect("no such terminal") };

    if !show_window {
        for c in command.chars() {
            terminal().input_key(0, 0, c);
        }
        terminal().input_key(0, 0, '\n');
    }

    let add_blink_timer =
        |t: u64| timer_manager().add_timer(Timer::new(t + TIMER_FREQ / 2, 1, task_id));
    add_blink_timer(timer_manager().current_tick());
    let mut active_mode = WindowActiveMode::Deactivate;

    loop {
        unsafe { asm!("cli") };
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
                    let area = terminal().blink_cursor();

                    let msg = Message::new(MessageType::Layer(LayerMessage {
                        layer_id: terminal().layer_id,
                        op: LayerOperation::DrawArea(area),
                        src_task_id: task_id,
                    }));
                    unsafe { asm!("cli") };
                    task_manager()
                        .send_message(task_manager().main_task().id(), msg)
                        .unwrap();
                    unsafe { asm!("sti") };
                }
            }
            MessageType::KeyPush(arg) => {
                if !arg.press {
                    continue;
                }
                let area = terminal().input_key(arg.modifier, arg.keycode, arg.ascii);
                if show_window {
                    let msg = Message::new(MessageType::Layer(LayerMessage {
                        layer_id: terminal().layer_id,
                        op: LayerOperation::DrawArea(area),
                        src_task_id: task_id,
                    }));
                    unsafe { asm!("cli") };
                    task_manager()
                        .send_message(task_manager().main_task().id(), msg)
                        .unwrap();
                    unsafe { asm!("sti") };
                }
            }
            MessageType::WindowActive(mode) => active_mode = mode,
            _ => {}
        }
    }
}

pub(crate) fn terminal_window(terminal_layer_id: LayerID) -> &'static mut Window {
    layer_manager()
        .get_layer_mut(terminal_layer_id)
        .expect("couldn't find terminal window")
        .get_window_mut()
}

#[derive(Clone)]
pub(crate) struct AppLoadInfo {
    vaddr_end: u64,
    entry: u64,
    pml4: *const PageMapEntry,
}

impl AppLoadInfo {
    fn new(vaddr_end: u64, entry: u64, pml4: *const PageMapEntry) -> Self {
        Self {
            vaddr_end,
            entry,
            pml4,
        }
    }
}

const ROWS: usize = 15;
const COLUMNS: usize = 60;
const LINE_MAX: usize = 128;

pub(crate) struct Terminal {
    task_id: TaskID,
    layer_id: LayerID,
    cursor: Vector2D<i32>,
    is_cursor_visible: bool,
    line_buf: String,
    command_history: CommandHistory,
    files: [Rc<RefCell<FileDescriptor>>; STD_ERR + 1],
    last_exit_code: i32,
}

/// Some functions depend on global functions although Terminal is not in a global module.
impl Terminal {
    fn new(task_id: TaskID) -> Terminal {
        let files = [
            Rc::new(RefCell::new(FileDescriptor::Terminal(
                TerminalFileDescriptor::new(task_id),
            ))),
            Rc::new(RefCell::new(FileDescriptor::Terminal(
                TerminalFileDescriptor::new(task_id),
            ))),
            Rc::new(RefCell::new(FileDescriptor::Terminal(
                TerminalFileDescriptor::new(task_id),
            ))),
        ];

        Self {
            task_id,
            layer_id: LayerID::MAX,
            cursor: Vector2D::new(0, 0),
            is_cursor_visible: false,
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

    fn initialize(
        &mut self,
        show_window: bool,
        layout_manager: &mut LayerManager,
        pixel_format: PixelFormat,
    ) {
        if show_window {
            let mut window = Window::new_with_title(
                COLUMNS * 8 + 8 + Window::TITLED_WINDOW_MARGIN.x as usize,
                ROWS * 16 + 8 + Window::TITLED_WINDOW_MARGIN.y as usize,
                pixel_format,
                "MikanTerm",
            );

            let inner_size = window.inner_size();
            draw_terminal(&mut window, Vector2D::new(0, 0), inner_size);
            self.layer_id = layout_manager.new_layer(window).set_draggable(true).id();
            self.print(">");
        }
    }

    fn blink_cursor(&mut self) -> Rectangle<i32> {
        self.is_cursor_visible = !self.is_cursor_visible;
        self.draw_cursor(self.is_cursor_visible);
        Rectangle::new(self.calc_cursor_pos(), Vector2D::new(7, 15))
    }

    fn draw_cursor(&mut self, visible: bool) {
        if let Some(window) = self.window_mut() {
            let color = if visible { &COLOR_WHITE } else { &COLOR_BLACK };
            fill_rectangle(
                &mut window.normal_window_writer(),
                &self.calc_cursor_pos(),
                &Vector2D::new(7, 15),
                color,
            );
        }
    }

    fn input_key(&mut self, _modifier: u8, keycode: u8, ascii: char) -> Rectangle<i32> {
        self.draw_cursor(false);

        let mut draw_area = Rectangle::new(self.calc_cursor_pos(), Vector2D::new(8 * 2, 16));

        match ascii {
            '\n' => {
                self.command_history.push(self.line_buf.to_string());

                self.cursor.x = 0;
                if self.cursor.y < ROWS as i32 - 1 {
                    self.cursor.y += 1;
                } else {
                    self.scroll1();
                }

                self.execute_line();
                self.print(">");
                draw_area.pos = TITLED_WINDOW_TOP_LEFT_MARGIN;
                draw_area.size = self.window_mut().map(|w| w.inner_size()).unwrap_or(
                    Vector2D::new(0, 0)
                        - TITLED_WINDOW_TOP_LEFT_MARGIN
                        - TITLED_WINDOW_BOTTOM_RIGHT_MARGIN,
                )
            }
            '\x08' => {
                if self.line_buf.pop().is_some() {
                    self.cursor.x -= 1;
                    if let Some(window) = self.window_mut() {
                        fill_rectangle(
                            &mut window.normal_window_writer(),
                            &self.calc_cursor_pos(),
                            &Vector2D::new(8, 16),
                            &COLOR_BLACK,
                        );
                    }
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
                if self.cursor.x < COLUMNS as i32 - 1 && self.line_buf.len() < LINE_MAX {
                    self.line_buf.push(ascii);
                    let pos = self.calc_cursor_pos();
                    if let Some(window) = self.window_mut() {
                        write_ascii(
                            &mut window.normal_window_writer(),
                            pos.x,
                            pos.y,
                            ascii,
                            &COLOR_WHITE,
                        );
                    }
                    self.cursor.x += 1;
                }
            }
        }

        self.draw_cursor(true);
        draw_area
    }

    fn calc_cursor_pos(&self) -> Vector2D<i32> {
        TITLED_WINDOW_TOP_LEFT_MARGIN + Vector2D::new(4 + 8 * self.cursor.x, 4 + 16 * self.cursor.y)
    }

    fn scroll1(&mut self) {
        if let Some(window) = self.window_mut() {
            let move_src = Rectangle::new(
                TITLED_WINDOW_TOP_LEFT_MARGIN + Vector2D::new(4, 4 + 16),
                Vector2D::new(8 * COLUMNS as i32, 16 * (ROWS as i32 - 1)),
            );
            window.move_(
                TITLED_WINDOW_TOP_LEFT_MARGIN + Vector2D::new(4, 4),
                &move_src,
            );
            fill_rectangle(
                window,
                &Vector2D::new(4, 4 + 16 * self.cursor.y),
                &Vector2D::new(8 * COLUMNS as i32, 16),
                &COLOR_BLACK,
            );
        }
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
                if let Some(window) = self.window_mut() {
                    fill_rectangle(
                        window,
                        &Vector2D::new(4, 4),
                        &Vector2D::new(8 * COLUMNS as i32, 16 * ROWS as i32),
                        &COLOR_BLACK,
                    );
                }
                self.cursor = Vector2D::new(0, 0);
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
            "noterm" => {
                if let Some(&first_arg) = argv.get(1) {
                    let c = CString::_new(first_arg.as_bytes().to_vec()).unwrap();
                    let task_id = task_manager()
                        .new_task()
                        .init_context(task_terminal, c.into_raw() as u64, get_cr3)
                        .id();
                    task_manager().wake_up(task_id).unwrap();
                }
                0
            }
            "memstat" => self.execute_memstat(),
            _ => {
                let root_cluster = boot_volume_image().get_root_cluster();
                if let (Some(file_entry), post_slash) = find_file(command, root_cluster as u64) {
                    if !file_entry.is_directory() && post_slash {
                        let name_bytes = file_entry.formatted_name();
                        let name = str_trimming_nul_unchecked(&name_bytes);
                        writeln!(self.stderr(), "{} is not a directory", name).unwrap();
                        1
                    } else {
                        match self.execute_file(file_entry, argv.as_slice()) {
                            Ok(ec) => ec,
                            Err((ec, err)) => {
                                let _ = writeln!(self.stderr(), "failed to exec file: {}", err);
                                -ec
                            }
                        }
                    }
                } else {
                    writeln!(self.stderr(), "no such command: {}", command).unwrap();
                    1
                }
            }
        };

        self.last_exit_code = exit_code;
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
        PageMapEntry::setup_page_maps(args_frame_addr, 1, true, get_cr3(), memory_manager())
            .map_err(|e| (0, e))?;
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
        PageMapEntry::setup_page_maps(
            stack_frame_addr,
            stack_size / 4096,
            true,
            get_cr3(),
            memory_manager(),
        )
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

        PageMapEntry::clean_page_maps(
            LinearAddress4Level::new(0xffff_8000_0000_0000),
            get_cr3(),
            memory_manager(),
        )
        .map_err(|e| (ret, e))?;

        free_pml4(task).map(|_| ret).map_err(|e| (ret, e))
    }

    fn load_app(
        &mut self,
        file_entry: &DirectoryEntry,
        task: &mut Task,
    ) -> Result<AppLoadInfo, Error> {
        let temp_pml4 = setup_pml4(task)?;
        if let Some(mut app_load) = get_app_load_mut(file_entry).cloned() {
            copy_page_maps(temp_pml4, app_load.pml4, 4, 256)?;
            app_load.pml4 = temp_pml4;
            return Ok(app_load);
        }

        let mut file_buf: Vec<u8> = vec![0; file_entry.file_size() as usize];
        file_entry.load_file(file_buf.as_mut_slice(), boot_volume_image());

        let elf_header = unsafe { Elf64Ehdr::from_mut(&mut file_buf) }.unwrap();
        if !elf_header.is_elf() {
            return Err(make_error!(Code::InvalidFile));
        }

        let elf_last_addr = elf_header.load_elf(get_cr3(), memory_manager())?;
        let mut app_load = AppLoadInfo::new(elf_last_addr, elf_header.e_entry as u64, temp_pml4);
        insert_app_load(file_entry, app_load.clone());

        app_load.pml4 = setup_pml4(task)?;
        copy_page_maps(app_load.pml4 as *mut _, temp_pml4, 4, 256)?;
        Ok(app_load)
    }

    pub(crate) fn print(&mut self, s: &str) {
        let prev_cursor = self.calc_cursor_pos();
        self.draw_cursor(false);

        for char in s.chars() {
            self.print_char(char);
        }

        self.draw_cursor(true);
        let current_cursor = self.calc_cursor_pos();

        let draw_pos = Vector2D::new(TITLED_WINDOW_TOP_LEFT_MARGIN.x, prev_cursor.y);
        let draw_size = Vector2D::new(
            self.window_mut()
                .map(|w| w.inner_size().x)
                .unwrap_or(-TITLED_WINDOW_TOP_LEFT_MARGIN.x - TITLED_WINDOW_BOTTOM_RIGHT_MARGIN.x),
            current_cursor.y - prev_cursor.y + 16,
        );
        let msg = Message::new(MessageType::Layer(LayerMessage {
            layer_id: self.layer_id,
            op: LayerOperation::DrawArea(Rectangle::new(draw_pos, draw_size)),
            src_task_id: self.task_id,
        }));

        unsafe { asm!("cli") };
        task_manager()
            .send_message(task_manager().main_task().id(), msg)
            .unwrap();
        unsafe { asm!("sti") };
    }

    fn print_char(&mut self, c: char) {
        let window = match self.window_mut() {
            None => return,
            Some(w) => w,
        };

        if c == '\n' {
            self.new_line();
            return;
        }

        let columns = COLUMNS as i32;
        if c.is_ascii() {
            if self.cursor.x == columns {
                self.new_line();
            }
            let pos = self.calc_cursor_pos();
            write_unicode(
                &mut window.normal_window_writer(),
                pos.x,
                pos.y,
                c,
                &COLOR_WHITE,
            )
            .unwrap_or_default();
            self.cursor.x += 1;
        } else {
            if self.cursor.x == columns - 1 {
                self.new_line();
            }
            let pos = self.calc_cursor_pos();
            write_unicode(
                &mut window.normal_window_writer(),
                pos.x,
                pos.y,
                c,
                &COLOR_WHITE,
            )
            .unwrap_or_default();
            self.cursor.x += 2;
        }
    }

    fn new_line(&mut self) {
        self.cursor.x = 0;
        if self.cursor.y < ROWS as i32 - 1 {
            self.cursor.y += 1;
        } else {
            self.scroll1()
        }
    }

    fn history_up_down(&mut self, direction: Direction) -> Rectangle<i32> {
        self.cursor.x = 1;
        let first_pos = self.calc_cursor_pos();
        let draw_area = Rectangle::new(first_pos, Vector2D::new(8 * (COLUMNS as i32 - 1), 16));
        if let Some(window) = self.window_mut() {
            fill_rectangle(
                &mut window.normal_window_writer(),
                &draw_area.pos,
                &draw_area.size,
                &COLOR_BLACK,
            );
        }

        self.line_buf = match direction {
            Direction::Up => self.command_history.up().to_string(),
            Direction::Down => self.command_history.down().to_string(),
        };

        if let Some(window) = self.window_mut() {
            write_string(
                &mut window.normal_window_writer(),
                first_pos.x,
                first_pos.y,
                self.line_buf.as_str(),
                &COLOR_WHITE,
            );
        }
        self.cursor.x = self.line_buf.len() as i32 + 1;

        draw_area
    }

    fn window_mut(&self) -> Option<&'static mut Window> {
        layer_manager()
            .get_layer_mut(self.layer_id)
            .map(|l| l.get_window_mut())
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
        let first_arg = argv.get(1).unwrap_or(&"").deref();

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

        let mut fd = FatFileDescriptor::new(file_entry);
        let mut u8buf = [0; 4];
        self.draw_cursor(false);
        loop {
            if fd.read(&mut u8buf[0..1], bpb) != 1 {
                break;
            }

            let u8_remain = count_utf8_size(u8buf[0]) - 1;
            if u8_remain > 0 && fd.read(&mut u8buf[1..1 + u8_remain], bpb) != u8_remain {
                break;
            }
            let char = char::from_u32(convert_utf8_to_u32(&u8buf)).unwrap_or('□');
            write!(self.stdout(), "{}", char).unwrap();
        }
        self.draw_cursor(true);
        0
    }

    fn execute_memstat(&mut self) -> i32 {
        let p_stat = memory_manager().stat();
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

    fn stdout(&mut self) -> RefMut<'_, FileDescriptor> {
        self.files[STD_OUT].borrow_mut()
    }

    fn stderr(&mut self) -> RefMut<'_, FileDescriptor> {
        self.files[STD_ERR].borrow_mut()
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

impl Write for Terminal {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.print(s);
        Ok(())
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
    let parsed = s.trim().split_whitespace().collect::<VecDeque<_>>();
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

fn new_c_chars_vec(strs: &[&str]) -> Vec<*const c_char> {
    strs.iter()
        .map(|&s| CString::_new(s.as_bytes().to_vec()).unwrap())
        .map(|c| c.into_raw() as *const c_char)
        .collect::<Vec<_>>()
}

pub fn setup_pml4(current_task: &mut Task) -> Result<*mut PageMapEntry, Error> {
    let pml4 = PageMapEntry::new_page_map(memory_manager())?;

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
