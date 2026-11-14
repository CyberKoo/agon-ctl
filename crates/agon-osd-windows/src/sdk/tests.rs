use crate::Library;
use std::rc::Rc;

pub(super) fn library() -> Rc<Library> {
    // No vendor SDK or hardware is used: all SDK entry points are test functions.
    Rc::new(Library::load("SDK test", &["kernel32.dll"]).unwrap())
}

pub(super) fn edid() -> [u8; 128] {
    let mut data = [0; 128];
    data[..8].copy_from_slice(&[0, 255, 255, 255, 255, 255, 255, 0]);
    data[8..16].copy_from_slice(&[0x12, 0x34, 0x56, 0x78, 1, 2, 3, 4]);
    data[126] = 1;
    data[127] = 0_u8.wrapping_sub(data[..127].iter().fold(0_u8, |a, b| a.wrapping_add(*b)));
    data
}
