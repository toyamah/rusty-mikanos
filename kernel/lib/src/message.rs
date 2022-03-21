use crate::graphics::{Rectangle, Vector2D};
use crate::layer::LayerID;
use crate::task::TaskID;

#[derive(Debug, PartialEq)]
pub struct Message {
    pub m_type: MessageType,
}

impl Message {
    pub const fn new(m_type: MessageType) -> Message {
        Self { m_type }
    }

    pub fn is_layer_finished(&self) -> bool {
        match self.m_type {
            MessageType::InterruptXhci => false,
            MessageType::TimerTimeout { .. } => false,
            MessageType::KeyPush { .. } => false,
            MessageType::Layer(_) => false,
            MessageType::LayerFinish => true,
        }
    }
}

#[derive(Debug, PartialEq)]
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
    Layer(LayerMessage),
    LayerFinish,
}

#[derive(Debug, PartialEq)]
pub struct LayerMessage {
    pub layer_id: LayerID,
    pub op: LayerOperation,
    pub src_task_id: TaskID,
}

#[derive(Debug, PartialEq)]
pub enum LayerOperation {
    Move { pos: Vector2D<i32> },
    MoveRelative { pos: Vector2D<i32> },
    Draw,
    DrawArea(Rectangle<i32>),
}
