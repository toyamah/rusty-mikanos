#[derive(Copy, Clone)]
pub struct Message {
    pub m_type: MessageType,
    pub arg: Arg,
}

impl Message {
    pub const fn new(m_type: MessageType, arg: Arg) -> Message {
        Self { m_type, arg }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum MessageType {
    InterruptXhci,
    TimerTimeout,
    KeyPush,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union Arg {
    pub timer: TimerMessage,
    pub keyboard: Keyboard,
    none: (),
}

impl Arg {
    pub const NONE: Arg = Arg { none: () };
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TimerMessage {
    pub timeout: u64,
    pub value: i32,
}

impl TimerMessage {
    pub fn new(timeout: u64, value: i32) -> TimerMessage {
        Self { timeout, value }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Keyboard {
    pub keycode: u8,
    pub ascii: char,
}

impl Keyboard {
    pub fn new(keycode: u8, ascii: char) -> Keyboard {
        Self { keycode, ascii }
    }
}
