use crate::message::{KeyPushMessage, Message, MessageType};
use crate::task::global::main_task_id;
use crate::task::TaskManager;

const KEYCODE_MAP: [char; 256] = [
    '\0', '\0', '\0', '\0', 'a', 'b', 'c', 'd', // 0
    'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', // 8
    'm', 'n', 'o', 'p', 'q', 'r', 's', 't', // 16
    'u', 'v', 'w', 'x', 'y', 'z', '1', '2', // 24
    '3', '4', '5', '6', '7', '8', '9', '0', // 32
    '\n', '\x08', '\x08', '\t', ' ', '-', '=', '[', // 40
    ']', '\\', '#', ';', '\'', '`', ',', '.', // 48
    '/', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 56
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 64
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 72
    '\0', '\0', '\0', '\0', '/', '*', '-', '+', // 80
    '\n', '1', '2', '3', '4', '5', '6', '7', // 88
    '8', '9', '0', '.', '\\', '\0', '\0', '=', // 96
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 104
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 112
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 120
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 128
    '\0', '\\', '\0', '\0', '\0', '\0', '\0', '\0', // 136
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 144
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 152
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 160
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 168
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 176
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 184
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 192
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 200
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 208
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 216
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 224
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 232
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 240
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 248
];

const KEYCODE_MAP_SHIFT: [char; 256] = [
    '\0', '\0', '\0', '\0', 'A', 'B', 'C', 'D', // 0
    'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', // 8
    'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', // 16
    'U', 'V', 'W', 'X', 'Y', 'Z', '!', '@', // 24
    '#', '$', '%', '^', '&', '*', '(', ')', // 32
    '\n', '\x08', '\x08', '\t', ' ', '_', '+', '{', // 40
    '}', '|', '~', ':', '"', '~', '<', '>', // 48
    '?', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 56
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 64
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 72
    '\0', '\0', '\0', '\0', '/', '*', '-', '+', // 80
    '\n', '1', '2', '3', '4', '5', '6', '7', // 88
    '8', '9', '0', '.', '\\', '\0', '\0', '=', // 96
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 104
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 112
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 120
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 128
    '\0', '|', '\0', '\0', '\0', '\0', '\0', '\0', // 136
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 144
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 152
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 160
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 168
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 176
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 184
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 192
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 200
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 208
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 216
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 224
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 232
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 240
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 248
];

pub const KEY_D: u8 = 7;
pub const KEY_Q: u8 = 20;
pub const KEY_F2: u8 = 59;
pub const L_CONTROL_BIT_MASK: u8 = 0b00000001;
pub const L_SHIFT_BIT_MASK: u8 = 0b00000010;
pub const L_ALT_BIT_MASK: u8 = 0b00000100;
pub const L_GUIBIT_MASK: u8 = 0b00001000;
pub const R_CONTROL_BIT_MASK: u8 = 0b00010000;
pub const R_SHIFT_BIT_MASK: u8 = 0b00100000;
pub const R_ALT_BIT_MASK: u8 = 0b01000000;
pub const R_GUIBIT_MASK: u8 = 0b10000000;

pub fn on_input(modifier: u8, keycode: u8, press: bool, task_manager: &mut TaskManager) {
    let ascii = if is_shift_key_inputted(modifier) {
        KEYCODE_MAP_SHIFT[keycode as usize]
    } else {
        KEYCODE_MAP[keycode as usize]
    };
    task_manager
        .send_message(
            main_task_id(),
            Message::new(MessageType::KeyPush(KeyPushMessage {
                modifier,
                keycode,
                ascii,
                press,
            })),
        )
        .unwrap();
}

pub(crate) fn is_shift_key_inputted(modifier: u8) -> bool {
    (modifier & (L_SHIFT_BIT_MASK | R_SHIFT_BIT_MASK)) != 0
}

pub(crate) fn is_control_key_inputted(modifier: u8) -> bool {
    (modifier & (L_CONTROL_BIT_MASK | R_CONTROL_BIT_MASK)) != 0
}
