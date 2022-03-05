#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Message {
    pub m_type: MessageType,
}

impl Message {
    pub const fn new(m_type: MessageType) -> Message {
        Self { m_type }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MessageType {
    InterruptXhci,
    TimerTimeout {
        timeout: u64,
        value: i32,
    },
    KeyPush {
        modifier: u8,
        keycode: u8,
        ascii: char,
    },
}
