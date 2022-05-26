use crate::app_event;
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
}

#[derive(Debug, PartialEq)]
pub enum MessageType {
    InterruptXhci,
    TimerTimeout { timeout: u64, value: i32 },
    KeyPush(KeyPushMessage),
    Layer(LayerMessage),
    LayerFinish,
    MouseMove(MouseMoveMessage),
    MouseButton(MouseButtonMessage),
    WindowActive(WindowActiveMode),
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct KeyPushMessage {
    pub modifier: u8,
    pub keycode: u8,
    pub ascii: char,
    pub press: bool,
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

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C)]
pub struct MouseMoveMessage {
    pub x: i32,
    pub y: i32,
    pub dx: i32,
    pub dy: i32,
    pub buttons: u8,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C)]
pub struct MouseButtonMessage {
    pub x: i32,
    pub y: i32,
    pub press: i32,
    pub button: i32,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum WindowActiveMode {
    Activate,
    Deactivate,
}

// This trait is defined here because the app crate also uses app_event::MouseMove.
impl From<KeyPushMessage> for app_event::KeyPush {
    fn from(m: KeyPushMessage) -> Self {
        Self {
            modifier: m.modifier,
            keycode: m.keycode,
            ascii: m.ascii,
            press: m.press,
        }
    }
}

impl From<MouseMoveMessage> for app_event::MouseMove {
    fn from(m: MouseMoveMessage) -> Self {
        Self {
            x: m.x,
            y: m.y,
            dx: m.dx,
            dy: m.dy,
            buttons: m.buttons,
        }
    }
}

impl From<MouseButtonMessage> for app_event::MouseButton {
    fn from(m: MouseButtonMessage) -> Self {
        Self {
            x: m.x,
            y: m.y,
            press: m.press,
            button: m.button,
        }
    }
}
