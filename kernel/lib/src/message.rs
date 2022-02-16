#[derive(Copy, Clone, Debug)]
pub struct Message {
    pub m_type: MessageType,
}

impl Message {
    pub const fn new(m_type: MessageType) -> Message {
        Message { m_type }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum MessageType {
    InterruptXhci,
    InterruptLAPICTimer,
}
