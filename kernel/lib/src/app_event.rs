#[repr(C)]
pub struct AppEvent {
    type_: AppEventType,
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub enum AppEventType {
    Quit,
    Empty,
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
        }
    }
}
