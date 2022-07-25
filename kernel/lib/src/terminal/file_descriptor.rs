use crate::io::FileDescriptor;
use crate::keyboard::{is_control_key_inputted, KEY_D};
use crate::libc::{memcpy, memmove};
use crate::message::{Message, MessageType, PipeMessage};
use crate::str_trimming_nul;
use crate::sync::{Mutex, MutexGuard};
use crate::task::global::task_manager;
use crate::task::TaskID;
use crate::terminal::terminal_writer::{TerminalWriter, TERMINAL_WRITERS};
use alloc::string::String;
use alloc::sync::Arc;
use core::arch::asm;
use core::ffi::c_void;
use core::fmt::Write;
use core::{cmp, mem};

pub(super) struct TerminalDescriptor {
    pub(super) command_line: String,
    pub(super) exit_after_command: bool,
    pub(super) show_window: bool,
    pub(super) files: [Arc<Mutex<FileDescriptor>>; 3],
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
            let task = task_manager().get_task_mut(self.terminal_id);
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

            if is_control_key_inputted(arg.modifier) && arg.ascii.is_ascii() {
                write!(
                    self.terminal_writer(),
                    "^{}",
                    arg.ascii.to_ascii_uppercase()
                )
                .unwrap();
                if arg.keycode == KEY_D {
                    return 0; // EOT
                }
                continue;
            }

            let mut bytes = [0_u8; 4];
            let str = arg.ascii.encode_utf8(&mut bytes);
            {
                let mut w = self.terminal_writer();
                w.print(str);
                w.redraw();
            }
            let size = cmp::min(buf.len(), bytes.len());
            buf[..size].copy_from_slice(&bytes[..size]);
            return bytes.iter().filter(|&&x| x != 0).count();
        }
    }

    pub fn write(&mut self, buf: &[u8]) -> usize {
        match str_trimming_nul(buf) {
            Ok(str) => {
                let mut writer = self.terminal_writer();
                writer.print(str);
                writer.redraw();
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

    fn terminal_writer(&self) -> MutexGuard<TerminalWriter> {
        unsafe { TERMINAL_WRITERS.get(self.terminal_id) }.lock()
    }
}

pub(crate) struct PipeDescriptor {
    pub(super) task_id: TaskID,
    data: [u8; 16],
    len: usize,
    closed: bool,
    write_only: bool,
}

impl PipeDescriptor {
    pub fn new(task_id: TaskID) -> Self {
        Self {
            task_id,
            data: [0; 16],
            len: 0,
            closed: false,
            write_only: false,
        }
    }

    pub fn copy_for_write(&self) -> Self {
        Self {
            task_id: self.task_id,
            data: [0; 16],
            len: 0,
            closed: self.closed,
            write_only: true,
        }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        if self.write_only {
            panic!("this descriptor is not allowed to read");
        }

        if self.len > 0 {
            let copy_bytes = cmp::min(self.len, buf.len());
            buf[..copy_bytes].copy_from_slice(&self.data[..copy_bytes]);
            self.len -= copy_bytes;
            unsafe {
                memmove(
                    self.data.as_mut_ptr() as *mut c_void,
                    self.data.as_ptr().add(copy_bytes) as *const c_void,
                    self.len,
                )
            };
            return copy_bytes;
        }

        if self.closed {
            return 0;
        }

        let pipe_message = loop {
            unsafe { asm!("cli") };
            let message = match task_manager()
                .get_task_mut(self.task_id)
                .and_then(|t| t.receive_message())
            {
                None => {
                    task_manager().sleep(self.task_id).unwrap();
                    continue;
                }
                Some(m) => m,
            };
            unsafe { asm!("sti") };

            let pipe_message = match message.m_type {
                MessageType::Pipe(p) => p,
                _ => continue,
            };
            break pipe_message;
        };

        if pipe_message.len == 0 {
            return 0;
        }

        let copy_bytes = cmp::min(pipe_message.len, buf.len());

        buf[..copy_bytes].copy_from_slice(&pipe_message.data[..copy_bytes]);
        self.len = pipe_message.len - copy_bytes;
        if self.len != 0 {
            self.data[..self.len].copy_from_slice(&pipe_message.data[copy_bytes..self.len]);
        }
        copy_bytes
    }

    pub fn write(&mut self, buf: &[u8]) -> usize {
        let mut sent_bytes = 0;
        while sent_bytes < buf.len() {
            let mut data = [0_u8; 16];
            let len = cmp::min(buf.len() - sent_bytes, mem::size_of_val(&data));
            unsafe {
                memcpy(
                    data.as_mut_ptr() as *mut c_void,
                    buf.as_ptr().add(sent_bytes) as *const c_void,
                    len,
                );
            }
            sent_bytes += len;

            let message = Message::new(MessageType::Pipe(PipeMessage { data, len }));
            unsafe { asm!("cli") };
            let _ = task_manager().send_message(self.task_id, message);
            unsafe { asm!("sti") };
        }
        buf.len()
    }

    pub fn load(&mut self, _buf: &mut [u8], _offset: usize) -> usize {
        0
    }

    pub fn size(&self) -> usize {
        0
    }

    pub fn finish_write(&mut self) {
        let message = Message::new(MessageType::Pipe(PipeMessage {
            data: [0; 16],
            len: 0,
        }));
        unsafe { asm!("cli") };
        let _ = task_manager().send_message(self.task_id, message);
        unsafe { asm!("sti") };
    }
}
