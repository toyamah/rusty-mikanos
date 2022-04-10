// Note: Code in this file was borrowed and modified from the official Rust std library
// https://github.com/rust-lang/rust/blob/master/library/std/src/ffi/c_str.rs

// Note: These lines were modified from the original.
use crate::rust_official::cchar::c_char;
use crate::strlen;
use core::ptr::slice_from_raw_parts;

/// Representation of a borrowed C string.
///
/// This type represents a borrowed reference to a nul-terminated
/// array of bytes. It can be constructed safely from a <code>&[[u8]]</code>
/// slice, or unsafely from a raw `*const c_char`. It can then be
/// converted to a Rust <code>&[str]</code> by performing UTF-8 validation, or
/// into an owned [`CString`].
///
/// `&CStr` is to [`CString`] as <code>&[str]</code> is to [`String`]: the former
/// in each pair are borrowed references; the latter are owned
/// strings.
///
/// Note that this structure is **not** `repr(C)` and is not recommended to be
/// placed in the signatures of FFI functions. Instead, safe wrappers of FFI
/// functions may leverage the unsafe [`CStr::from_ptr`] constructor to provide
/// a safe interface to other consumers.
///
/// # Examples
///
/// Inspecting a foreign C string:
///
/// ```ignore (extern-declaration)
/// use std::ffi::CStr;
/// use std::os::raw::c_char;
///
/// extern "C" { fn my_string() -> *const c_char; }
///
/// unsafe {
///     let slice = CStr::from_ptr(my_string());
///     println!("string buffer size without nul terminator: {}", slice.to_bytes().len());
/// }
/// ```
///
/// Passing a Rust-originating C string:
///
/// ```ignore (extern-declaration)
/// use std::ffi::{CString, CStr};
/// use std::os::raw::c_char;
///
/// fn work(data: &CStr) {
///     extern "C" { fn work_with(data: *const c_char); }
///
///     unsafe { work_with(data.as_ptr()) }
/// }
///
/// let s = CString::new("data data data data").expect("CString::new failed");
/// work(&s);
/// ```
///
/// Converting a foreign C string into a Rust [`String`]:
///
/// ```ignore (extern-declaration)
/// use std::ffi::CStr;
/// use std::os::raw::c_char;
///
/// extern "C" { fn my_string() -> *const c_char; }
///
/// fn my_string_safe() -> String {
///     unsafe {
///         CStr::from_ptr(my_string()).to_string_lossy().into_owned()
///     }
/// }
///
/// println!("string: {}", my_string_safe());
/// ```
///
/// [str]: prim@str "str"
#[derive(Hash)]
// Note: These lines were modified from the original.
// #[cfg_attr(not(test), rustc_diagnostic_item = "CStr")]
// #[stable(feature = "rust1", since = "1.0.0")]
// FIXME:
// `fn from` in `impl From<&CStr> for Box<CStr>` current implementation relies
// on `CStr` being layout-compatible with `[u8]`.
// When attribute privacy is implemented, `CStr` should be annotated as `#[repr(transparent)]`.
// Anyway, `CStr` representation and layout are considered implementation detail, are
// not documented and must not be relied upon.
pub struct CStr {
    // FIXME: this should not be represented with a DST slice but rather with
    //        just a raw `c_char` along with some form of marker to make
    //        this an unsized type. Essentially `sizeof(&CStr)` should be the
    //        same as `sizeof(&c_char)` but `CStr` should be an unsized type.
    inner: [c_char],
}

// Note: I picked out some needed functions from `impl CStr` block of the original.
impl CStr {
    /// Wraps a raw C string with a safe C string wrapper.
    ///
    /// This function will wrap the provided `ptr` with a `CStr` wrapper, which
    /// allows inspection and interoperation of non-owned C strings. The total
    /// size of the raw C string must be smaller than `isize::MAX` **bytes**
    /// in memory due to calling the `slice::from_raw_parts` function.
    /// This method is unsafe for a number of reasons:
    ///
    /// * There is no guarantee to the validity of `ptr`.
    /// * The returned lifetime is not guaranteed to be the actual lifetime of
    ///   `ptr`.
    /// * There is no guarantee that the memory pointed to by `ptr` contains a
    ///   valid nul terminator byte at the end of the string.
    /// * It is not guaranteed that the memory pointed by `ptr` won't change
    ///   before the `CStr` has been destroyed.
    ///
    /// > **Note**: This operation is intended to be a 0-cost cast but it is
    /// > currently implemented with an up-front calculation of the length of
    /// > the string. This is not guaranteed to always be the case.
    ///
    /// # Examples
    ///
    /// ```ignore (extern-declaration)
    /// # fn main() {
    /// use std::ffi::CStr;
    /// use std::os::raw::c_char;
    ///
    /// extern "C" {
    ///     fn my_string() -> *const c_char;
    /// }
    ///
    /// unsafe {
    ///     let slice = CStr::from_ptr(my_string());
    ///     println!("string returned: {}", slice.to_str().unwrap());
    /// }
    /// # }
    /// ```
    #[inline]
    #[must_use]
    // Note: This line was modified from the original.
    // #[stable(feature = "rust1", since = "1.0.0")]
    pub unsafe fn from_ptr<'a>(ptr: *const c_char) -> &'a CStr {
        // SAFETY: The caller has provided a pointer that points to a valid C
        // string with a NUL terminator of size less than `isize::MAX`, whose
        // content remain valid and doesn't change for the lifetime of the
        // returned `CStr`.
        //
        // Thus computing the length is fine (a NUL byte exists), the call to
        // from_raw_parts is safe because we know the length is at most `isize::MAX`, meaning
        // the call to `from_bytes_with_nul_unchecked` is correct.
        //
        // The cast from c_char to u8 is ok because a c_char is always one byte.
        unsafe {
            // Note: These lines were modified from the original.
            // let len: usize = sys::strlen(ptr);
            // let len = 1;
            let len = strlen(ptr);
            let ptr = ptr as *const u8;
            let bytes = unsafe { &*slice_from_raw_parts(ptr, len as usize + 1) };
            CStr::from_bytes_with_nul_unchecked(bytes)
        }
    }

    /// Unsafely creates a C string wrapper from a byte slice.
    ///
    /// This function will cast the provided `bytes` to a `CStr` wrapper without
    /// performing any sanity checks. The provided slice **must** be nul-terminated
    /// and not contain any interior nul bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::ffi::{CStr, CString};
    ///
    /// unsafe {
    ///     let cstring = CString::new("hello").expect("CString::new failed");
    ///     let cstr = CStr::from_bytes_with_nul_unchecked(cstring.to_bytes_with_nul());
    ///     assert_eq!(cstr, &*cstring);
    /// }
    /// ```
    #[inline]
    #[must_use]
    // Note: These lines were modified from the original.
    // #[stable(feature = "cstr_from_bytes", since = "1.10.0")]
    // #[rustc_const_stable(feature = "const_cstr_unchecked", since = "1.59.0")]
    pub const unsafe fn from_bytes_with_nul_unchecked(bytes: &[u8]) -> &CStr {
        // SAFETY: Casting to CStr is safe because its internal representation
        // is a [u8] too (safe only inside std).
        // Dereferencing the obtained pointer is safe because it comes from a
        // reference. Making a reference is then safe because its lifetime
        // is bound by the lifetime of the given `bytes`.
        unsafe { &*(bytes as *const [u8] as *const CStr) }
    }

    /// Converts this C string to a byte slice.
    ///
    /// The returned slice will **not** contain the trailing nul terminator that this C
    /// string has.
    ///
    /// > **Note**: This method is currently implemented as a constant-time
    /// > cast, but it is planned to alter its definition in the future to
    /// > perform the length calculation whenever this method is called.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::ffi::CStr;
    ///
    /// let cstr = CStr::from_bytes_with_nul(b"foo\0").expect("CStr::from_bytes_with_nul failed");
    /// assert_eq!(cstr.to_bytes(), b"foo");
    /// ```
    #[inline]
    // Note: These lines were modified from the original.
    // #[must_use = "this returns the result of the operation, \
    //               without modifying the original"]
    // #[stable(feature = "rust1", since = "1.0.0")]
    pub fn to_bytes(&self) -> &[u8] {
        let bytes = self.to_bytes_with_nul();
        // SAFETY: to_bytes_with_nul returns slice with length at least 1
        unsafe { bytes.get_unchecked(..bytes.len() - 1) }
    }

    /// Converts this C string to a byte slice containing the trailing 0 byte.
    ///
    /// This function is the equivalent of [`CStr::to_bytes`] except that it
    /// will retain the trailing nul terminator instead of chopping it off.
    ///
    /// > **Note**: This method is currently implemented as a 0-cost cast, but
    /// > it is planned to alter its definition in the future to perform the
    /// > length calculation whenever this method is called.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::ffi::CStr;
    ///
    /// let cstr = CStr::from_bytes_with_nul(b"foo\0").expect("CStr::from_bytes_with_nul failed");
    /// assert_eq!(cstr.to_bytes_with_nul(), b"foo\0");
    /// ```
    #[inline]
    // Note: These lines were modified from the original.
    // #[must_use = "this returns the result of the operation, \
    //               without modifying the original"]
    // #[stable(feature = "rust1", since = "1.0.0")]
    pub fn to_bytes_with_nul(&self) -> &[u8] {
        unsafe { &*(&self.inner as *const [c_char] as *const [u8]) }
    }
}
