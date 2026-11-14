use std::ffi::c_void;

pub type CtlApiHandle = *mut c_void;
pub type CtlDeviceAdapterHandle = *mut c_void;
pub type CtlDisplayOutputHandle = *mut c_void;
pub type CtlResult = i32;

pub const CTL_IMPL_VERSION: u32 = 0x10001;
pub const CTL_I2C_MAX_DATA_SIZE: usize = 0x80;
pub const CTL_OPERATION_TYPE_READ: u32 = 1;
pub const CTL_OPERATION_TYPE_WRITE: u32 = 2;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CtlApplicationId {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CtlInitArgs {
    pub size: u32,
    pub version: u8,
    pub app_version: u32,
    pub flags: u32,
    pub supported_version: u32,
    pub application_uid: CtlApplicationId,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CtlI2cAccessArgs {
    pub size: u32,
    pub version: u8,
    pub data_size: u32,
    pub address: u32,
    pub operation_type: u32,
    pub offset: u32,
    pub flags: u32,
    pub rad: u64,
    pub data: [u8; CTL_I2C_MAX_DATA_SIZE],
}

impl Default for CtlI2cAccessArgs {
    fn default() -> Self {
        Self {
            size: std::mem::size_of::<Self>() as u32,
            version: 0,
            data_size: 0,
            address: 0,
            operation_type: 0,
            offset: 0,
            flags: 0,
            rad: 0,
            data: [0; CTL_I2C_MAX_DATA_SIZE],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CtlPanelDescriptorAccessArgs {
    pub size: u32,
    pub version: u8,
    pub operation_type: u32,
    pub block_number: u32,
    pub descriptor_data_size: u32,
    pub descriptor_data: *mut u8,
}

impl Default for CtlPanelDescriptorAccessArgs {
    fn default() -> Self {
        Self {
            size: 0,
            version: 0,
            operation_type: 0,
            block_number: 0,
            descriptor_data_size: 0,
            descriptor_data: std::ptr::null_mut(),
        }
    }
}

pub type CtlInit = unsafe extern "C" fn(*mut CtlInitArgs, *mut CtlApiHandle) -> CtlResult;
pub type CtlClose = unsafe extern "C" fn(CtlApiHandle) -> CtlResult;
pub type CtlEnumerateDevices =
    unsafe extern "C" fn(CtlApiHandle, *mut u32, *mut CtlDeviceAdapterHandle) -> CtlResult;
pub type CtlEnumerateDisplayOutputs = unsafe extern "C" fn(
    CtlDeviceAdapterHandle,
    *mut u32,
    *mut CtlDisplayOutputHandle,
) -> CtlResult;
pub type CtlI2cAccess =
    unsafe extern "C" fn(CtlDisplayOutputHandle, *mut CtlI2cAccessArgs) -> CtlResult;
pub type CtlPanelDescriptorAccess =
    unsafe extern "C" fn(CtlDisplayOutputHandle, *mut CtlPanelDescriptorAccessArgs) -> CtlResult;

const _: () = assert!(std::mem::size_of::<CtlInitArgs>() == 0x24);
const _: () = assert!(std::mem::size_of::<CtlI2cAccessArgs>() == 0xA8);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<CtlPanelDescriptorAccessArgs>() == 0x20);
#[cfg(target_pointer_width = "32")]
const _: () = assert!(std::mem::size_of::<CtlPanelDescriptorAccessArgs>() == 0x18);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_descriptor_default_has_zero_fields_and_null_buffer() {
        let args = CtlPanelDescriptorAccessArgs::default();
        assert_eq!(args.size, 0);
        assert_eq!(args.version, 0);
        assert_eq!(args.operation_type, 0);
        assert_eq!(args.block_number, 0);
        assert_eq!(args.descriptor_data_size, 0);
        assert!(args.descriptor_data.is_null());
    }
}
