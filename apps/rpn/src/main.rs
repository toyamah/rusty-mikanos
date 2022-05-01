#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;
use shared_lib::atol;
use shared_lib::rust_official::cchar::c_char;
use shared_lib::rust_official::cstr::CStr;

extern "C" {
    fn SyscallLogString(level: i64, s: *const c_char) -> CallResult;
    fn SyscallPutString(fd: i32, buf: usize, count: usize) -> CallResult;
}

#[repr(C)]
struct CallResult {
    value: u64,
    error: i32,
}

fn print(s: &str) {
    unsafe { SyscallPutString(1, s.as_ptr() as usize, s.as_bytes().len()) };
}

fn info(s: &str) {
    unsafe {
        SyscallLogString(3, s.as_ptr() as *const c_char);
    }
}

#[no_mangle]
pub extern "C" fn main(argc: i32, argv: *const *const c_char) -> i32 {
    let mut stack = Stack::new();
    // let plus = unsafe { CStr::from_bytes_with_nul_unchecked(b"+") };
    // let minus = unsafe { CStr::from_bytes_with_nul_unchecked(b"-") };

    for i in 1..argc {
        let ptr = unsafe { *argv.add(i as usize) };
        let c_str = unsafe { CStr::from_ptr(ptr) };
        let bytes = c_str.to_bytes();

        if bytes == b"+" {
            let b = stack.pop().unwrap();
            let a = stack.pop().unwrap();
            stack.push(a + b);
            info("+\0");
        } else if bytes == b"-" {
            let b = stack.pop().unwrap();
            let a = stack.pop().unwrap();
            stack.push(a - b);
            info("-\0");
        } else {
            let a = unsafe { atol(ptr) };
            stack.push(a);
            info("#\0");
        }

        // if unsafe { strcmp(ptr, plus.as_ptr()) } == 0 {
        //     let b = stack.pop().unwrap();
        //     let a = stack.pop().unwrap();
        //     stack.push(a + b);
        // } else if unsafe { strcmp(ptr, minus.as_ptr()) } == 0 {
        //     let b = stack.pop().unwrap();
        //     let a = stack.pop().unwrap();
        //     stack.push(a - b);
        // } else {
        //     let a = unsafe { atol(ptr) };
        //     stack.push(a);
        // }
    }

    print("test1");
    info("\nhello, this is rpn\n\0");
    print("test2");

    loop {}
    // stack.pop().unwrap_or(0) as i32
}

struct Stack {
    s: [i64; 100],
    ptr: usize,
}

impl Stack {
    fn new() -> Stack {
        Self {
            s: [0; 100],
            ptr: 0,
        }
    }

    fn pop(&mut self) -> Option<i64> {
        if self.ptr == 0 {
            None
        } else {
            self.ptr -= 1;
            let value = self.s[self.ptr];
            Some(value)
        }
    }

    fn push(&mut self, value: i64) {
        self.s[self.ptr] = value;
        self.ptr += 1;
    }
}

// unsafe fn strcmp(a_ptr: *const c_char, b_ptr: *const c_char) -> i32 {
//     let a_str = CStr::from_ptr(a_ptr);
//     let a_bytes = a_str.to_bytes();
//     let b_str = CStr::from_ptr(b_ptr);
//     let b_bytes = b_str.to_bytes();
//
//     let len = cmp::min(a_bytes.len(), b_bytes.len());
//
//     for i in 0..len {
//         let a = a_bytes[i];
//         let b = b_bytes[i];
//         if a != b {
//             return (a - b) as i32;
//         }
//     }
//
//     return (a_bytes[len - 1] - b_bytes[len - 1]) as i32;
// }
//
// unsafe fn atol(s: *const c_char) -> i64 {
//     let c_str = CStr::from_ptr(s);
//     let bytes = c_str.to_bytes();
//
//     let mut v = 0;
//     for &c in bytes {
//         v = v * 10 + i64::from(c - b'0');
//     }
//     v as i64
// }

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}

macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ({
        $crate::io::_print($crate::format_args_nl!($($arg)*));
    })
}
