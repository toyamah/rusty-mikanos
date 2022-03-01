use crate::message::{Arg, Keyboard, Message, MessageType};
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

const L_CONTROL_BIT_MASK: u8 = 0b00000001;
const L_SHIFT_BIT_MASK: u8 = 0b00000010;
const L_ALT_BIT_MASK: u8 = 0b00000100;
const L_GUIBIT_MASK: u8 = 0b00001000;
const R_CONTROL_BIT_MASK: u8 = 0b00010000;
const R_SHIFT_BIT_MASK: u8 = 0b00100000;
const R_ALT_BIT_MASK: u8 = 0b01000000;
const R_GUIBIT_MASK: u8 = 0b10000000;

pub fn on_input(modifier: u8, keycode: u8, task_manager: &mut TaskManager) {
    let shift_inputted = (modifier & (L_SHIFT_BIT_MASK | R_SHIFT_BIT_MASK)) != 0;
    let ascii = if shift_inputted {
        KEYCODE_MAP_SHIFT[keycode as usize]
    } else {
        KEYCODE_MAP[keycode as usize]
    };
    task_manager
        .send_message(
            task_manager.main_task().id(),
            Message::new(
                MessageType::KeyPush,
                Arg {
                    keyboard: Keyboard::new(modifier, keycode, ascii),
                },
            ),
        )
        .unwrap();
}
