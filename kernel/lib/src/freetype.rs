#![allow(non_camel_case_types)]

use crate::memory_manager::global::memory_manager;
use crate::memory_manager::{FrameID, BYTES_PER_FRAME};
use core::ffi::c_void;
use core::ptr::null_mut;

pub type FT_Library = *mut FT_LibraryRec;
pub type FT_LibraryRec = c_void;
// pub type FT_Face = *mut FT_FaceRec;

pub type FT_Memory = *mut FT_MemoryRec;

pub type FT_Alloc_Func = extern "C" fn(FT_Memory, i64) -> *mut c_void;
pub type FT_Free_Func = extern "C" fn(FT_Memory, *mut c_void);
pub type FT_Realloc_Func = extern "C" fn(FT_Memory, i64, i64, *mut c_void) -> *mut c_void;

#[repr(C)]
#[derive(Debug, Hash, PartialEq, Eq)]
#[allow(missing_copy_implementations)]
pub struct FT_MemoryRec {
    pub user: *mut c_void,
    pub alloc: FT_Alloc_Func,
    pub free: FT_Free_Func,
    pub realloc: FT_Realloc_Func,
}

impl FT_MemoryRec {
    pub fn new() -> FT_MemoryRec {
        Self {
            user: 0 as *mut c_void,
            alloc: alloc_library,
            free: free_library,
            realloc: realloc_library,
        }
    }
}

// #[no_mangle]
// pub extern "C" fn sbrk(incr: isize) -> *mut c_void {
//     unsafe {
//         if program_break as isize == 0
//             || program_break as isize + incr >= program_break_end as isize
//         {
//             let i: isize = -1;
//             return i as *mut c_void;
//         }
//
//         let prev_break = program_break;
//         program_break = (program_break as isize + incr) as *mut c_void;
//         prev_break
//     }
// }

extern "C" fn alloc_library(_memory: FT_Memory, size: i64) -> *mut c_void {
    // let mut a = memory_manager().allocate(size as usize / BYTES_PER_FRAME);
    // a.unwrap().frame() as *mut c_void
    unsafe { malloc(size as usize) }
}

extern "C" fn free_library(_memory: FT_Memory, block: *mut c_void) {
    // let start_frame = FrameID::new(block as usize / BYTES_PER_FRAME);
    // memory_manager().free(start_frame, ).unwrap()
    unsafe { free(block) }
}

extern "C" fn realloc_library(
    _memory: FT_Memory,
    _cur_size: i64,
    new_size: i64,
    block: *mut c_void,
) -> *mut c_void {
    // panic!("a");
    unsafe { realloc(block, new_size as usize) }
    // unsafe { libc::realloc(block, new_size as size_t) }
}

/// A value of 0 is always interpreted as a successful operation.
/// https://freetype.org/freetype2/docs/reference/ft2-basic_types.html#ft_error
pub type FT_Error = i32;

pub const FT_LOAD_RENDER: i32 = 0x1 << 2;
pub const FT_LOAD_TARGET_MONO: i32 = FT_RENDER_MODE_MONO << 16;

/// https://freetype.org/freetype2/docs/reference/ft2-base_interface.html#ft_render_mode
pub const FT_RENDER_MODE_MONO: i32 = 2;

// #[repr(C)]
// #[derive(Debug, Hash, PartialEq, Eq)]
// #[allow(missing_copy_implementations)]
// pub struct FT_FaceRec {
//     pub num_faces: i64,
//     pub face_index: i64,
//
//     pub face_flags: i64,
//     pub style_flags: i64,
//
//     pub num_glyphs: i64,
//
//     pub family_name: *mut u8,
//     pub style_name: *mut u8,
//
//     pub num_fixed_sizes: i32,
//     pub available_sizes: *mut FT_Bitmap_Size,
//
//     pub num_charmaps: i32,
//     pub charmaps: *mut FT_CharMap,
//
//     pub generic: FT_Generic,
//
//     pub bbox: FT_BBox,
//
//     pub units_per_EM: u16,
//     pub ascender: i16,
//     pub descender: i16,
//     pub height: i16,
//
//     pub max_advance_width: i16,
//     pub max_advance_height: i16,
//
//     pub underline_position: i16,
//     pub underline_thickness: i16,
//
//     pub glyph: *mut c_void,
//     pub size: *mut c_void,
//     pub charmap: *mut c_void,
//
//     pub driver: *mut c_void,
//     pub memory: FT_Memory,
//     pub stream: *mut c_void,
//
//     pub sizes_list: FT_ListRec,
//
//     pub autohint: FT_Generic,
//     pub extensions: *mut c_void,
//
//     pub internal: *mut c_void,
// }
//
// #[repr(C)]
// #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
// pub struct FT_ListRec {
//     pub head: *mut c_void,
//     pub tail: *mut c_void,
// }
//
// #[repr(C)]
// #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
// pub struct FT_Bitmap_Size {
//     pub height: i16,
//     pub width: i16,
//
//     pub size: i64,
//
//     pub x_ppem: i64,
//     pub y_ppem: i64,
// }
//
// #[repr(C)]
// #[derive(Debug, Hash, PartialEq, Eq)]
// #[allow(missing_copy_implementations)]
// pub struct FT_Generic {
//     pub data: *mut c_void,
//     pub finalizer: *mut c_void,
// }
//
/// https://freetype.org/freetype2/docs/reference/ft2-base_interface.html
extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);

    pub fn FT_Init_FreeType(alibrary: *mut FT_Library) -> FT_Error;

    // pub fn FT_New_Memory_Face(
    //     library: FT_Library,
    //     file_base: *const u8,
    //     file_size: i64,
    //     face_index: i64,
    //     aface: *mut FT_Face,
    // ) -> FT_Error;

    pub fn FT_New_Library(memory: FT_Memory, alibrary: *mut FT_Library) -> FT_Error;

    // pub fn FT_Set_Pixel_Sizes(face: FT_Face, pixel_width: u32, pixel_height: u32) -> FT_Error;

    // pub fn FT_Get_Char_Index(face: FT_Face, charcode: u64) -> u32;

    // pub fn FT_Load_Glyph(face: FT_Face, glyph_index: u32, load_flags: i32) -> FT_Error;

    // pub fn FT_Done_Face(face: FT_Face) -> FT_Error;

}
