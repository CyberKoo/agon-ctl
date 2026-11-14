use std::ffi::{c_char, c_void};

pub type CfRef = *const c_void;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    pub fn CFRelease(value: CfRef);
    pub fn CFGetTypeID(value: CfRef) -> usize;
    pub fn CFStringGetTypeID() -> usize;
    pub fn CFDictionaryGetTypeID() -> usize;
    pub fn CFStringCreateWithCString(
        allocator: CfRef,
        value: *const c_char,
        encoding: u32,
    ) -> CfRef;
    pub fn CFStringGetLength(value: CfRef) -> i64;
    pub fn CFStringGetMaximumSizeForEncoding(length: i64, encoding: u32) -> i64;
    pub fn CFStringGetCString(
        value: CfRef,
        buffer: *mut c_char,
        buffer_size: i64,
        encoding: u32,
    ) -> u8;
    pub fn CFDictionaryGetValue(dictionary: CfRef, key: CfRef) -> CfRef;
}
