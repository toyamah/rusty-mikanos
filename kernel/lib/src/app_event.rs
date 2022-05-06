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
union AppEventArg {
    pub mouse_move: MouseMove,
    pub empty: (),
}

#[derive(Copy, Clone)]
#[repr(C)]
struct MouseMove {
    pub x: i32,
    pub y: i32,
    pub dx: i32,
    pub dy: i32,
    pub buttons: u8,
}

impl AppEvent {
    pub fn type_(&self) -> AppEventType {
        self.type_
    }

    pub fn set_type(&mut self, t: AppEventType) {
        self.type_ = t;
    }
}

impl Default for AppEvent {
    fn default() -> Self {
        AppEvent {
            type_: AppEventType::Empty,
            arg: AppEventArg { empty: () },
        }
    }
}
