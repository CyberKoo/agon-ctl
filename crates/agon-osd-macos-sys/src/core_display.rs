use super::cf::CfRef;
use std::ffi::c_void;

#[link(name = "CoreDisplay", kind = "framework")]
unsafe extern "C" {
    pub fn CoreDisplay_DisplayCreateInfoDictionary(display: u32) -> CfRef;
    pub fn IOAVServiceCreateWithService(allocator: CfRef, service: u32) -> CfRef;
    pub fn IOAVServiceWriteI2C(
        service: CfRef,
        chip_address: u32,
        data_address: u32,
        input_buffer: *const c_void,
        input_buffer_size: u32,
    ) -> i32;
    pub fn IOAVServiceReadI2C(
        service: CfRef,
        chip_address: u32,
        offset: u32,
        output_buffer: *mut c_void,
        output_buffer_size: u32,
    ) -> i32;
}
