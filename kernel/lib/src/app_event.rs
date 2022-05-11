// This file is also referenced by the app crate as a symbolic link

#[repr(C)]
pub struct AppEvent {
    pub type_: AppEventType,
    pub arg: AppEventArg,
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub enum AppEventType {
    Quit,
    Empty,
    MouseMove,
    MouseButton,
    TimerTimeout,
    KeyPush,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union AppEventArg {
    pub mouse_move: MouseMove,
    pub mouse_button: MouseButton,
    pub timer_timeout: TimerTimeout,
    pub key_push: KeyPush,
    pub empty: (),
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct MouseMove {
    pub x: i32,
    pub y: i32,
    pub dx: i32,
    pub dy: i32,
    pub buttons: u8,
}

pub const BUTTON_PRESSED: i32 = 1;
pub const BUTTON_RELEASED: i32 = 0;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct MouseButton {
    pub x: i32,
    pub y: i32,
    pub press: i32,
    pub button: i32,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct TimerTimeout {
    pub timeout: u64,
    pub value: i32,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct KeyPush {
    pub modifier: u8,
    pub keycode: u8,
    pub ascii: char,
    pub press: bool,
}

impl Default for AppEvent {
    fn default() -> Self {
        AppEvent {
            type_: AppEventType::Empty,
            arg: AppEventArg { empty: () },
        }
    }
}

impl MouseButton {
    pub fn is_pressed(&self) -> bool {
        self.press == BUTTON_PRESSED
    }
}
