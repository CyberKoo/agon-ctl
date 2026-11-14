use super::cf::{CfDictionary, CfRef, OwnedCf};
use super::io_kit::IoObject;
use agon_osd_macos_sys::core_display as bindings;

pub(crate) fn display_info(display: u32) -> Option<CfDictionary> {
    // SAFETY: the online display ID is valid and the returned dictionary follows the create rule.
    let raw = unsafe { bindings::CoreDisplay_DisplayCreateInfoDictionary(display) };
    // SAFETY: `raw` is the owned result of CoreDisplay_DisplayCreateInfoDictionary.
    unsafe { CfDictionary::from_created(raw) }
}

pub(crate) struct AvService {
    _owned: OwnedCf,
    raw: CfRef,
}

impl AvService {
    pub(crate) fn new(service: &IoObject) -> Option<Self> {
        // SAFETY: the IOService is alive and the result follows the create rule.
        let raw =
            unsafe { bindings::IOAVServiceCreateWithService(std::ptr::null(), service.raw()) };
        // SAFETY: `raw` is the owned result of IOAVServiceCreateWithService.
        let owned = unsafe { OwnedCf::from_created(raw)? };
        Some(Self { _owned: owned, raw })
    }

    pub(crate) fn write_i2c(
        &self,
        chip_address: u32,
        data_address: u32,
        data: &[u8],
    ) -> Result<(), i32> {
        let length = u32::try_from(data.len()).map_err(|_| -1)?;
        // SAFETY: the service is alive and `data` exposes `length` readable bytes.
        let status = unsafe {
            bindings::IOAVServiceWriteI2C(
                self.raw,
                chip_address,
                data_address,
                data.as_ptr().cast(),
                length,
            )
        };
        if status == 0 { Ok(()) } else { Err(status) }
    }

    pub(crate) fn read_i2c(
        &self,
        chip_address: u32,
        offset: u32,
        output: &mut [u8],
    ) -> Result<(), i32> {
        let length = u32::try_from(output.len()).map_err(|_| -1)?;
        // SAFETY: the service is alive and `output` exposes `length` writable bytes.
        let status = unsafe {
            bindings::IOAVServiceReadI2C(
                self.raw,
                chip_address,
                offset,
                output.as_mut_ptr().cast(),
                length,
            )
        };
        if status == 0 { Ok(()) } else { Err(status) }
    }
}
