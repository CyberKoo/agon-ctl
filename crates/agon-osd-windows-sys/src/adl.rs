use std::ffi::{c_char, c_void};

pub type AdlContextHandle = *mut c_void;
pub type AdlMainMallocCallback = unsafe extern "system" fn(i32) -> *mut c_void;

pub const ADL_OK: i32 = 0;
pub const ADL_DISPLAY_DISPLAYINFO_DISPLAYCONNECTED: i32 = 0x0000_0001;

// ADLDisplayID in GPUOpen-LibrariesAndSDKs/display-library/include/adl_structures.h.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct AdlDisplayId {
    pub display_logical_index: i32,
    pub display_physical_index: i32,
    pub display_logical_adapter_index: i32,
    pub display_physical_adapter_index: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AdlDisplayInfo {
    pub display_id: AdlDisplayId,
    pub display_controller_index: i32,
    pub display_name: [c_char; 256],
    pub display_manufacturer_name: [c_char; 256],
    pub display_type: i32,
    pub display_output_type: i32,
    pub display_connector: i32,
    pub display_info_mask: i32,
    pub display_info_value: i32,
}

impl Default for AdlDisplayInfo {
    fn default() -> Self {
        Self {
            display_id: AdlDisplayId::default(),
            display_controller_index: 0,
            display_name: [0; 256],
            display_manufacturer_name: [0; 256],
            display_type: 0,
            display_output_type: 0,
            display_connector: 0,
            display_info_mask: 0,
            display_info_value: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AdlDisplayEdidData {
    pub size: i32,
    pub flag: i32,
    pub edid_size: i32,
    pub block_index: i32,
    pub edid_data: [c_char; 256],
    pub reserved: [i32; 4],
}

impl Default for AdlDisplayEdidData {
    fn default() -> Self {
        Self {
            size: std::mem::size_of::<Self>() as i32,
            flag: 0,
            edid_size: 0,
            block_index: 0,
            edid_data: [0; 256],
            reserved: [0; 4],
        }
    }
}

pub type Adl2MainControlCreate =
    unsafe extern "system" fn(AdlMainMallocCallback, i32, *mut AdlContextHandle) -> i32;
pub type Adl2MainControlDestroy = unsafe extern "system" fn(AdlContextHandle) -> i32;
pub type Adl2AdapterNumberOfAdaptersGet =
    unsafe extern "system" fn(AdlContextHandle, *mut i32) -> i32;
pub type Adl2DisplayDisplayInfoGet = unsafe extern "system" fn(
    AdlContextHandle,
    i32,
    *mut i32,
    *mut *mut AdlDisplayInfo,
    i32,
) -> i32;
pub type Adl2DisplayEdidDataGet =
    unsafe extern "system" fn(AdlContextHandle, i32, i32, *mut AdlDisplayEdidData) -> i32;
pub type Adl2DisplayDdcBlockAccessGet = unsafe extern "system" fn(
    AdlContextHandle,
    i32,
    i32,
    i32,
    i32,
    i32,
    *mut c_char,
    *mut i32,
    *mut c_char,
) -> i32;

const _: () = assert!(std::mem::size_of::<AdlDisplayId>() == 16);
const _: () = {
    assert!(std::mem::offset_of!(AdlDisplayId, display_logical_index) == 0);
    assert!(std::mem::offset_of!(AdlDisplayId, display_physical_index) == 4);
    assert!(std::mem::offset_of!(AdlDisplayId, display_logical_adapter_index) == 8);
    assert!(std::mem::offset_of!(AdlDisplayId, display_physical_adapter_index) == 12);
};

const _: () = assert!(std::mem::size_of::<AdlDisplayInfo>() == 552);
const _: () = assert!(std::mem::size_of::<AdlDisplayEdidData>() == 288);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_id_decodes_native_field_order() {
        assert_eq!(std::mem::offset_of!(AdlDisplayId, display_logical_index), 0);
        assert_eq!(
            std::mem::offset_of!(AdlDisplayId, display_physical_index),
            4
        );
        assert_eq!(
            std::mem::offset_of!(AdlDisplayId, display_logical_adapter_index),
            8
        );
        assert_eq!(
            std::mem::offset_of!(AdlDisplayId, display_physical_adapter_index),
            12
        );
        let words = [11_i32, 22, 33, 44];
        // SAFETY: four initialized i32 values match the repr(C) ADLDisplayID layout.
        let id = unsafe { std::ptr::read_unaligned(words.as_ptr().cast::<AdlDisplayId>()) };
        assert_eq!(id.display_logical_index, 11);
        assert_eq!(id.display_physical_index, 22);
        assert_eq!(id.display_logical_adapter_index, 33);
        assert_eq!(id.display_physical_adapter_index, 44);
    }
}
