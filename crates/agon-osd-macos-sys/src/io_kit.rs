use super::cf::CfRef;
use std::ffi::c_char;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    pub fn IORegistryGetRootEntry(main_port: u32) -> u32;
    pub fn IORegistryEntryCreateIterator(
        entry: u32,
        plane: *const c_char,
        options: u32,
        iterator: *mut u32,
    ) -> i32;
    pub fn IOIteratorNext(iterator: u32) -> u32;
    pub fn IOObjectRelease(object: u32) -> i32;
    pub fn IORegistryEntryGetName(entry: u32, name: *mut c_char) -> i32;
    pub fn IORegistryEntryGetPath(entry: u32, plane: *const c_char, path: *mut c_char) -> i32;
    pub fn IORegistryEntryCreateCFProperty(
        entry: u32,
        key: CfRef,
        allocator: CfRef,
        options: u32,
    ) -> CfRef;
    pub fn IORegistryEntryGetParentEntry(entry: u32, plane: *const c_char, parent: *mut u32)
    -> i32;
}
