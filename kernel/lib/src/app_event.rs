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
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union AppEventArg {
    pub mouse_move: MouseMove,
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

impl Default for AppEvent {
    fn default() -> Self {
        AppEvent {
            type_: AppEventType::Empty,
            arg: AppEventArg { empty: () },
        }
    }
}
