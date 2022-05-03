use crate::syscall::SyscallWinFillRectangle;
use crate::{ByteBuffer, SyscallError, SyscallOpenWindow, SyscallWinWriteString};

#[derive(Copy, Clone)]
struct LayerID(u32);

const TITLE_OFFSET: (i32, i32) = (8, 28);

pub struct Window {
    layer_id: LayerID,
}

impl Window {
    fn new(layer_id: LayerID) -> Self {
        Self { layer_id }
    }

    pub fn open(wh: (i32, i32), xy: (i32, i32), title: &str) -> Result<Window, SyscallError> {
        let w = wh.0 + TITLE_OFFSET.0;
        let h = wh.1 + TITLE_OFFSET.1;
        let mut buf = ByteBuffer::new();
        buf.write_str_with_nul(title);

        let result = unsafe { SyscallOpenWindow(w, h, xy.0, xy.1, buf.as_ptr_c_char()) };
        result.to_result().map(|v| Window::new(LayerID(v as u32)))
    }

    pub fn write_string(&mut self, xy: (i32, i32), color: u32, text: &str) {
        let mut buf = ByteBuffer::new();
        buf.write_str_with_nul(text);
        unsafe {
            SyscallWinWriteString(self.layer_id.0, xy.0, xy.1, color, buf.as_ptr_c_char());
        }
    }

    pub fn fill_rectangle(&mut self, xy: (i32, i32), wh: (i32, i32), color: u32) {
        unsafe {
            SyscallWinFillRectangle(self.layer_id.0, xy.0, xy.1, wh.0, wh.1, color);
        }
    }
}
