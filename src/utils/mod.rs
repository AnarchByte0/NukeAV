pub mod importer;
pub mod exporter;

pub use exporter::UIBuilder;

use crate::prUTF16Char;
use std::os::raw::c_char;

/// Copy a Rust string slice into a null-terminated raw C char buffer
pub fn copy_c_str(src: &str, dst: &mut [c_char]) {
    if dst.is_empty() {
        return;
    }
    let bytes = src.as_bytes();
    let max_len = dst.len() - 1;
    let len = bytes.len().min(max_len);
    for i in 0..len {
        let b = bytes[i];
        if b == 0 {
            dst[i] = 0;
            return;
        }
        dst[i] = b as c_char;
    }
    dst[len] = 0;
}

/// Convert a null-terminated UTF-16 pointer into a Rust String
/// # Safety
/// The caller must ensure that `ptr` points to a valid null-terminated UTF-16 string or is null.
pub unsafe fn get_utf16_string(ptr: *const prUTF16Char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    let slice = std::slice::from_raw_parts(ptr, len);
    String::from_utf16_lossy(slice)
}

/// Copy a Rust string slice into a fixed-size Adobe UTF-16 buffer
/// # Safety
/// The caller must ensure that `out` points to an allocated buffer of at least `max_len` UTF-16 characters or is null.
pub unsafe fn str_to_utf16(s: &str, out: *mut prUTF16Char, max_len: usize) {
    if out.is_null() || max_len == 0 {
        return;
    }
    let mut i = 0;
    for c in s.encode_utf16() {
        if i >= max_len - 1 {
            break;
        }
        *out.add(i) = c;
        i += 1;
    }
    *out.add(i) = 0; // null-terminator
}

/// Allocate a persistent UTF-16 string on the heap and return a raw pointer
pub fn leak_utf16(s: &str) -> *const prUTF16Char {
    let mut vec: Vec<prUTF16Char> = s.encode_utf16().collect();
    vec.push(0); // null-terminator
    Box::leak(vec.into_boxed_slice()).as_ptr()
}
