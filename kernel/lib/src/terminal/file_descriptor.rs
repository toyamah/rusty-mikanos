use crate::io::FileDescriptor;
use crate::keyboard::{is_control_key_inputted, KEY_D};
use crate::message::MessageType;
use crate::str_trimming_nul;
use crate::task::global::task_manager;
use crate::task::TaskID;
use crate::terminal::lib::get_terminal_mut_by;
use alloc::rc::Rc;
use alloc::string::String;
use core::arch::asm;
use core::cell::RefCell;
use core::fmt::Write;

pub(super) struct TerminalDescriptor {
    pub(super) command_line: String,
    pub(super) exit_after_command: bool,
    pub(super) show_window: bool,
    pub(super) files: [Rc<RefCell<FileDescriptor>>; 3],
}

pub(crate) struct TerminalFileDescriptor {
    terminal_id: TaskID,
}

impl TerminalFileDescriptor {
    pub fn new(terminal_id: TaskID) -> Self {
        Self { terminal_id }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        loop {
            unsafe { asm!("cli") };
            let task = task_manager()
                .get_task_mut(get_terminal_mut_by(self.terminal_id).unwrap().task_id());
            if task.is_none() {
                return 0;
            }
            let task = task.unwrap();
            let message = match task.receive_message() {
                None => {
                    task_manager().sleep(task.id()).unwrap();
                    continue;
                }
                Some(m) => m,
            };
            unsafe { asm!("sti") };
            let arg = if let MessageType::KeyPush(arg) = message.m_type {
                arg
            } else {
                continue;
            };
            if !arg.press {
                continue;
            }

            let terminal = get_terminal_mut_by(self.terminal_id).unwrap();
            if is_control_key_inputted(arg.modifier) && arg.ascii.is_ascii() {
                write!(terminal, "^{}", arg.ascii.to_ascii_uppercase()).unwrap();
                if arg.keycode == KEY_D {
                    return 0; // EOT
                }
                continue;
            }

            let mut bytes = [0_u8; 4];
            let str = arg.ascii.encode_utf8(&mut bytes);
            terminal.print(str);
            buf[..4].copy_from_slice(&bytes);
            return bytes.iter().filter(|&&x| x != 0).count();
        }
    }

    pub fn write(&mut self, buf: &[u8]) -> usize {
        match str_trimming_nul(buf) {
            Ok(str) => {
                let terminal = get_terminal_mut_by(self.terminal_id).unwrap();
                terminal.print(str);
                str.as_bytes().len()
            }
            Err(_) => 0,
        }
    }

    pub fn load(&mut self, _buf: &mut [u8], _offset: usize) -> usize {
        0
    }

    pub fn size(&self) -> usize {
        0
    }
}

pub(crate) struct PipeDescriptor {
    task_id: TaskID,
    data: [u8; 16],
    len: usize,
    closed: bool,
}

impl PipeDescriptor {
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        todo!()
    }

    pub fn write(&mut self, buf: &[u8]) -> usize {
        todo!()
    }

    pub fn load(&mut self, _buf: &mut [u8], _offset: usize) -> usize {
        0
    }

    pub fn size(&self) -> usize {
        0
    }
}
