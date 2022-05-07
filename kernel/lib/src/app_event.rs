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
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union AppEventArg {
    pub mouse_move: MouseMove,
    pub mouse_button: MouseButton,
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
