use std::ffi::{c_char, c_void};

pub type NvPhysicalGpuHandle = *mut c_void;
pub type NvStatus = i32;

pub const NVAPI_OK: i32 = 0;
pub const NVAPI_MAX_PHYSICAL_GPUS: usize = 64;
pub const NVAPI_I2C_SPEED_DEPRECATED: u32 = 0xFFFF;
pub const NV_EDID_DATA_SIZE: usize = 256;

pub const NVAPI_INITIALIZE: u32 = 0x150E828;
pub const NVAPI_ENUM_PHYSICAL_GPUS: u32 = 0xE5AC921F;
pub const NVAPI_GPU_GET_CONNECTED_DISPLAY_IDS: u32 = 0x078DBA2;
pub const NVAPI_GPU_GET_EDID: u32 = 0x37D32E69;
// Converts a display ID into a GPU handle and output mask.
pub const NVAPI_SYS_GET_GPU_AND_OUTPUT_ID_FROM_DISPLAY_ID: u32 = 0x112BA1A5;
pub const NVAPI_I2C_WRITE: u32 = 0xE812EB07;
pub const NVAPI_I2C_READ: u32 = 0x2FDE12C5;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct NvGpuDisplayIds {
    pub version: u32,
    pub connector_type: u32,
    pub display_id: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NvEdid {
    pub version: u32,
    pub edid_data: [u8; NV_EDID_DATA_SIZE],
    pub sizeof_edid: u32,
    pub edid_id: u32,
    pub offset: u32,
}

impl Default for NvEdid {
    fn default() -> Self {
        Self {
            version: (0x30000 + std::mem::size_of::<Self>()) as u32,
            edid_data: [0; NV_EDID_DATA_SIZE],
            sizeof_edid: 0,
            edid_id: 0,
            offset: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NvI2cInfoV3 {
    pub version: u32,
    pub display_mask: u32,
    pub is_ddc_port: u8,
    pub i2c_dev_address: u8,
    pub i2c_reg_address: *mut u8,
    pub reg_addr_size: u32,
    pub data: *mut u8,
    pub data_size: u32,
    pub i2c_speed: u32,
    pub i2c_speed_khz: u32,
    pub port_id: u8,
    pub is_port_id_set: u32,
}

impl Default for NvI2cInfoV3 {
    fn default() -> Self {
        Self {
            version: 0,
            display_mask: 0,
            is_ddc_port: 0,
            i2c_dev_address: 0,
            i2c_reg_address: std::ptr::null_mut(),
            reg_addr_size: 0,
            data: std::ptr::null_mut(),
            data_size: 0,
            i2c_speed: 0,
            i2c_speed_khz: 0,
            port_id: 0,
            is_port_id_set: 0,
        }
    }
}

pub type NvQueryInterface = unsafe extern "C" fn(u32) -> *mut c_void;
pub type NvInitialize = unsafe extern "C" fn() -> NvStatus;
pub type NvEnumPhysicalGpus = unsafe extern "C" fn(*mut NvPhysicalGpuHandle, *mut u32) -> NvStatus;
pub type NvGpuGetConnectedDisplayIds =
    unsafe extern "C" fn(NvPhysicalGpuHandle, *mut NvGpuDisplayIds, *mut u32, u32) -> NvStatus;
// NVAPI_INTERFACE uses __cdecl (nvapi_lite_salstart.h), including on Win32.
pub type NvSysGetGpuAndOutputIdFromDisplayId =
    unsafe extern "C" fn(u32, *mut NvPhysicalGpuHandle, *mut u32) -> NvStatus;
pub type NvGpuGetEdid = unsafe extern "C" fn(NvPhysicalGpuHandle, u32, *mut NvEdid) -> NvStatus;
pub type NvI2cRead = unsafe extern "C" fn(NvPhysicalGpuHandle, *mut NvI2cInfoV3) -> NvStatus;
pub type NvI2cWrite = unsafe extern "C" fn(NvPhysicalGpuHandle, *mut NvI2cInfoV3) -> NvStatus;

pub type NvApiShortString = [c_char; 64];
pub type NvGetErrorMessage = unsafe extern "C" fn(NvStatus, NvApiShortString) -> NvStatus;

const _: () = assert!(std::mem::size_of::<NvGpuDisplayIds>() == 16);
const _: () = assert!(std::mem::size_of::<NvEdid>() == 272);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<NvI2cInfoV3>() == 64);
#[cfg(target_pointer_width = "32")]
const _: () = assert!(std::mem::size_of::<NvI2cInfoV3>() == 44);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i2c_default_has_zero_fields_and_null_buffers() {
        let info = NvI2cInfoV3::default();
        assert_eq!(info.version, 0);
        assert_eq!(info.display_mask, 0);
        assert_eq!(info.is_ddc_port, 0);
        assert_eq!(info.i2c_dev_address, 0);
        assert!(info.i2c_reg_address.is_null());
        assert_eq!(info.reg_addr_size, 0);
        assert!(info.data.is_null());
        assert_eq!(info.data_size, 0);
        assert_eq!(info.i2c_speed, 0);
        assert_eq!(info.i2c_speed_khz, 0);
        assert_eq!(info.port_id, 0);
        assert_eq!(info.is_port_id_set, 0);
    }
}
