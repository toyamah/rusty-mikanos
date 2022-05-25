use crate::app_event::{AppEvent, AppEventArg, AppEventType, KeyPush, TimerTimeout};
use crate::asm::global::{write_msr, SyscallEntry};
use crate::fat::global::{boot_volume_image, find_file};
use crate::fat::FatFileDescriptor;
use crate::font::write_string;
use crate::graphics::global::frame_buffer_config;
use crate::graphics::{fill_rectangle, PixelColor, PixelWriter, Rectangle, Vector2D};
use crate::io::{FileDescriptor, STD_IN};
use crate::keyboard::{KEY_Q, L_CONTROL_BIT_MASK, R_CONTROL_BIT_MASK};
use crate::layer::global::{active_layer, layer_manager, layer_task_map, screen_frame_buffer};
use crate::layer::LayerID;
use crate::message::MessageType;
use crate::msr::{IA32_EFFR, IA32_FMASK, IA32_LSTAR, IA32_STAR};
use crate::rust_official::c_str::CStr;
use crate::rust_official::cchar::c_char;
use crate::task::global::task_manager;
use crate::terminal::global::get_terminal_mut_by;
use crate::timer::global::timer_manager;
use crate::timer::{Timer, TIMER_FREQ};
use crate::Window;
use core::arch::asm;
use core::{mem, slice};
use log::{debug, log, Level};

type SyscallFuncType = fn(u64, u64, u64, u64, u64, u64) -> SyscallResult;

// Execute command `errno -l` to see each error number
const EPERM: i32 = 1; // Operation not permitted (POSIX.1-2001).
const ENOENT: i32 = 2; // No such file or directory
const E2BIG: i32 = 7; // Argument list too long
const EBADF: i32 = 9; // Bad file descriptor
const EFAULT: i32 = 14; // Bad address
const EINVAL: i32 = 22; // Invalid argument

const O_RDONLY: i32 = 0x0000; /* open for reading only */
const O_WRONLY: i32 = 0x0001; /* open for writing only */
const O_RDWR: i32 = 0x0002; /* open for reading and writing */
const O_ACCMODE: i32 = 0x0003; /* mask for above modes */

#[repr(C)]
struct SyscallResult {
    value: u64,
    error: i32,
}

impl SyscallResult {
    fn new(value: u64, error: i32) -> Self {
        Self { value, error }
    }

    fn err(value: u64, error: i32) -> Self {
        Self { value, error }
    }

    fn ok(value: u64) -> Self {
        Self { value, error: 0 }
    }

    fn is_err(&self) -> bool {
        self.error != 0
    }
}

fn log_string(log_level: u64, s: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64) -> SyscallResult {
    let log_level = match log_level {
        1 => Level::Error,
        2 => Level::Warn,
        3 => Level::Info,
        4 => Level::Debug,
        5 => Level::Trace,
        _ => return SyscallResult::err(0, EPERM),
    };

    let c_str = unsafe { c_str_from(s) };
    let len = c_str.to_bytes().len();
    if len > 1024 {
        return SyscallResult::err(0, E2BIG);
    }
    let str = str_from(c_str.to_bytes());
    log!(log_level, "{}", str);
    SyscallResult::ok(len as u64)
}

fn put_string(fd: u64, buf: u64, count: u64, _a4: u64, _a5: u64, _a6: u64) -> SyscallResult {
    if count > 1024 {
        return SyscallResult::err(0, E2BIG);
    }

    let c_str = unsafe { c_str_from(buf) };
    let str = str_from(&c_str.to_bytes()[..count as usize]);

    if fd == 1 {
        let task_id = task_manager().current_task().id();
        let terminal = get_terminal_mut_by(task_id).expect("failed to get terminal");
        terminal.print(str);
        return SyscallResult::ok(count);
    }

    SyscallResult::err(0, EBADF)
}

fn exit(status: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64) -> SyscallResult {
    unsafe { asm!("cli") };
    let task = task_manager().current_task();
    unsafe { asm!("sti") };
    SyscallResult::new(*task.os_stack_pointer(), status as i32)
}

fn open_window(w: u64, h: u64, x: u64, y: u64, title: u64, _a6: u64) -> SyscallResult {
    let c_str = unsafe { c_str_from(title) };
    let str = str_from(c_str.to_bytes());
    let window = Window::new_with_title(
        w as usize,
        h as usize,
        frame_buffer_config().pixel_format,
        str,
    );

    unsafe { asm!("cli") };
    let layer_id = layer_manager()
        .new_layer(window)
        .set_draggable(true)
        .move_(Vector2D::new(x as i32, y as i32))
        .id();
    active_layer().activate(
        Some(layer_id),
        layer_manager(),
        screen_frame_buffer(),
        task_manager(),
        layer_task_map(),
    );

    let task_id = task_manager().current_task().id();
    layer_task_map().insert(layer_id, task_id);
    unsafe { asm!("sti") };

    SyscallResult::ok(layer_id.value() as u64)
}

fn win_write_string(
    layer_id_flags: u64,
    x: u64,
    y: u64,
    color: u64,
    text: u64,
    _a6: u64,
) -> SyscallResult {
    let color = PixelColor::from(color as u32);
    let c_str = unsafe { c_str_from(text) };
    let str = str_from(c_str.to_bytes());

    do_win_func(layer_id_flags, |window| {
        write_string(
            &mut window.normal_window_writer(),
            x as i32,
            y as i32,
            str,
            &color,
        );
        SyscallResult::ok(0)
    })
}

fn win_fill_rectangle(
    layer_id_flags: u64,
    x: u64,
    y: u64,
    w: u64,
    h: u64,
    color: u64,
) -> SyscallResult {
    let color = PixelColor::from(color as u32);
    let pos = Vector2D::new(x as i32, y as i32);
    let size = Vector2D::new(w as i32, h as i32);
    do_win_func(layer_id_flags, |window| {
        fill_rectangle(&mut window.normal_window_writer(), &pos, &size, &color);
        SyscallResult::ok(0)
    })
}

fn get_current_tick(_a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64) -> SyscallResult {
    SyscallResult::new(timer_manager().current_tick(), TIMER_FREQ as i32)
}

fn win_redraw(
    layer_id_flags: u64,
    _a2: u64,
    _a3: u64,
    _a4: u64,
    _a5: u64,
    _a6: u64,
) -> SyscallResult {
    do_win_func(layer_id_flags, |_| SyscallResult::ok(0))
}

fn win_draw_line(
    layer_id_flags: u64,
    x0: u64,
    y0: u64,
    x1: u64,
    y1: u64,
    color: u64,
) -> SyscallResult {
    let mut x0 = x0 as i32;
    let mut y0 = y0 as i32;
    let mut x1 = x1 as i32;
    let mut y1 = y1 as i32;
    let color = PixelColor::from(color as u32);

    do_win_func(layer_id_flags, |window| {
        let dx = x1 - x0 + (x1 - x0).signum();
        let dy = y1 - y0 + (y1 - y0).signum();

        if dx == 0 && dy == 0 {
            window.normal_window_writer().write(x0, y0, &color);
            return SyscallResult::ok(0);
        }

        if dx.abs() >= dy.abs() {
            if dx < 0 {
                mem::swap(&mut x0, &mut x1);
                mem::swap(&mut y0, &mut y1);
            }

            let roundish = if y1 >= y0 { libm::floor } else { libm::ceil };
            let m = dy as f64 / dx as f64;
            for x in x0..=x1 {
                let y = roundish(m * (x - x0) as f64 + y0 as f64);
                window
                    .normal_window_writer()
                    .write(x as i32, y as i32, &color);
            }
        } else {
            if dy < 0 {
                mem::swap(&mut x0, &mut x1);
                mem::swap(&mut y0, &mut y1);
            }

            let roundish = if x1 >= x0 { libm::floor } else { libm::ceil };
            let m = dx as f64 / dy as f64;
            for y in y0..=y1 {
                let x = roundish(m * (y - y0) as f64 + x0 as f64);
                window
                    .normal_window_writer()
                    .write(x as i32, y as i32, &color);
            }
        }

        SyscallResult::ok(0)
    })
}

fn close_window(
    layer_id_flags: u64,
    _a2: u64,
    _a3: u64,
    _a4: u64,
    _a5: u64,
    _a6: u64,
) -> SyscallResult {
    let layer_id = LayerID::new((layer_id_flags & 0xffffffff) as u32);
    let layer = match layer_manager().get_layer_mut(layer_id) {
        None => return SyscallResult::err(0, EBADF),
        Some(l) => l,
    };

    let layer_pos = layer.position();
    let win_size = layer.get_window_ref().size().to_i32_vec2d();

    unsafe { asm!("cli") };
    active_layer().activate(
        None,
        layer_manager(),
        screen_frame_buffer(),
        task_manager(),
        layer_task_map(),
    );
    layer_manager().remove_layer(layer_id);
    layer_manager().draw_on(Rectangle::new(layer_pos, win_size), screen_frame_buffer());
    unsafe { asm!("sti") };

    SyscallResult::ok(0)
}

fn read_event(app_events: u64, len: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64) -> SyscallResult {
    if app_events < 0x8000_0000_0000_0000 {
        return SyscallResult::err(0, EFAULT);
    }
    let app_events = app_events as *mut u64 as *mut AppEvent;
    let len = len as usize;

    unsafe { asm!("cli") };
    let task = task_manager().current_task_mut();
    unsafe { asm!("sti") };

    let mut i = 0;
    while i < len {
        unsafe { asm!("cli") };
        let msg = task.receive_message();
        if msg.is_none() && i == 0 {
            task_manager()
                .sleep(task.id())
                .expect("could not sleep a task");
            continue;
        }
        unsafe { asm!("sti") };

        let msg = if let Some(m) = msg {
            m
        } else {
            break;
        };

        match msg.m_type {
            MessageType::KeyPush {
                modifier,
                keycode,
                ascii,
                press,
            } => {
                let event = unsafe { app_events.add(i).as_mut() }
                    .expect("failed to convert to AppEvent Ref");
                let is_control_inputted = modifier & (L_CONTROL_BIT_MASK | R_CONTROL_BIT_MASK) != 0;
                if keycode == KEY_Q && is_control_inputted {
                    event.type_ = AppEventType::Quit;
                    i += 1;
                } else {
                    event.type_ = AppEventType::KeyPush;
                    event.arg = AppEventArg {
                        key_push: KeyPush {
                            modifier,
                            keycode,
                            ascii,
                            press,
                        },
                    };
                    i += 1;
                }
            }
            MessageType::MouseMove(arg) => {
                let event = unsafe { app_events.add(i).as_mut() }
                    .expect("failed to convert to AppEvent Ref");
                event.type_ = AppEventType::MouseMove;
                event.arg = AppEventArg {
                    mouse_move: arg.into(),
                };
                i += 1;
            }
            MessageType::MouseButton(arg) => {
                let event = unsafe { app_events.add(i).as_mut() }
                    .expect("failed to convert to AppEvent Ref");
                event.type_ = AppEventType::MouseButton;
                event.arg = AppEventArg {
                    mouse_button: arg.into(),
                };
                i += 1;
            }
            MessageType::TimerTimeout { timeout, value } => {
                let event = unsafe { app_events.add(i).as_mut() }
                    .expect("failed to convert to AppEvent Ref");
                let is_created_by_app = value < 0;
                if is_created_by_app {
                    event.type_ = AppEventType::TimerTimeout;
                    event.arg = AppEventArg {
                        timer_timeout: TimerTimeout {
                            timeout,
                            value: -value,
                        },
                    };
                    i += 1;
                }
            }
            _ => debug!("uncaught event type: {:?}", msg.m_type),
        }
    }

    SyscallResult::ok(i as u64)
}

fn create_timer(
    mode: u64,
    timer_value: u64,
    timeout_ms: u64,
    _a4: u64,
    _a5: u64,
    _a6: u64,
) -> SyscallResult {
    let mode = mode as u32;
    let timer_value = timer_value as i32;
    if timer_value <= 0 {
        return SyscallResult::err(0, EINVAL);
    }

    unsafe { asm!("cli") };
    let task_id = task_manager().current_task().id();
    unsafe { asm!("sti") };

    let is_relative = mode & 1 == 1;
    let timeout = if is_relative {
        timeout_ms * TIMER_FREQ / 1000 + timer_manager().current_tick()
    } else {
        timeout_ms * TIMER_FREQ / 1000
    };

    unsafe { asm!("cli") };
    timer_manager().add_timer(Timer::new(timeout, -timer_value, task_id));
    unsafe { asm!("sti") };
    SyscallResult::new(timeout * 1000 / TIMER_FREQ, 0)
}

fn open_file(path: u64, flag: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64) -> SyscallResult {
    let path = unsafe { c_str_from(path) }.to_str().unwrap();
    let flags = flag as i32;
    unsafe { asm!("cli") };
    let task = task_manager().current_task_mut();
    unsafe { asm!("sti") };

    if path == STD_IN {
        return SyscallResult::ok(0);
    }

    if (flags & O_ACCMODE) == O_WRONLY {
        return SyscallResult::err(0, EINVAL);
    }

    let (dir, post_slash) = find_file(path, boot_volume_image().get_root_cluster() as u64);
    if dir.is_none() {
        return SyscallResult::err(0, ENOENT);
    }
    let dir = dir.unwrap();
    if !dir.is_directory() && post_slash {
        return SyscallResult::err(0, ENOENT);
    }

    let fd = task.register_file_descriptor(FileDescriptor::Fat(FatFileDescriptor::new(dir)));
    SyscallResult::ok(fd as u64)
}

fn read_file(fd: u64, buf: u64, count: u64, _a4: u64, _a5: u64, _a6: u64) -> SyscallResult {
    let fd = fd as i32;
    let buf = buf as *mut u64 as *mut u8;
    let count = count as usize;
    unsafe { asm!("cli") };
    let task = task_manager().current_task_mut();
    unsafe { asm!("sti") };

    if fd < 0 || task.files_len() <= fd as usize {
        return SyscallResult::err(0, EBADF);
    }
    let fd = fd as usize;

    if let Some(descriptor) = task.get_file_mut(fd) {
        let buf = unsafe { slice::from_raw_parts_mut(buf, count) };
        let size = descriptor.read(buf);
        SyscallResult::ok(size as u64)
    } else {
        SyscallResult::err(0, EBADF)
    }
}

#[no_mangle]
static mut syscall_table: [SyscallFuncType; 14] = [
    log_string,
    put_string,
    exit,
    open_window,
    win_write_string,
    win_fill_rectangle,
    get_current_tick,
    win_redraw,
    win_draw_line,
    close_window,
    read_event,
    create_timer,
    open_file,
    read_file,
];

pub fn initialize_syscall() {
    write_msr(IA32_EFFR, 0x0501);
    write_msr(IA32_LSTAR, SyscallEntry as usize as u64);
    write_msr(IA32_STAR, 8 << 32 | (16 | 3) << 48);
    write_msr(IA32_FMASK, 0);
}

unsafe fn c_str_from<'a>(p: u64) -> &'a CStr {
    CStr::from_ptr(p as *const u64 as *const c_char)
}

fn str_from(bytes: &[u8]) -> &str {
    core::str::from_utf8(bytes).expect("could not convert to str")
}

fn do_win_func<F>(layer_id_flags: u64, f: F) -> SyscallResult
where
    F: FnOnce(&mut Window) -> SyscallResult,
{
    let layer_flags = layer_id_flags >> 32;
    let layer_id = LayerID::new((layer_id_flags & 0xffffffff) as u32);

    unsafe { asm!("cli") };
    let layer = layer_manager().get_layer_mut(layer_id);
    unsafe { asm!("sti") };

    let res = match layer {
        None => SyscallResult::err(0, EBADF),
        Some(l) => f(l.get_window_mut()),
    };
    if res.is_err() {
        return res;
    }

    if (layer_flags & 1) == 0 {
        unsafe { asm!("cli") };
        layer_manager().draw_layer_of(layer_id, screen_frame_buffer());
        unsafe { asm!("sti") };
    }

    res
}
