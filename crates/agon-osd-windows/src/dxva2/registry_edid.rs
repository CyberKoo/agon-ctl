use agon_osd::parse_edid;
use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    DICS_FLAG_GLOBAL, DIREG_DEV, HDEVINFO, SP_DEVICE_INTERFACE_DATA,
    SP_DEVICE_INTERFACE_DETAIL_DATA_W, SP_DEVINFO_DATA, SetupDiCreateDeviceInfoList,
    SetupDiDestroyDeviceInfoList, SetupDiGetDeviceInterfaceDetailW, SetupDiOpenDevRegKey,
    SetupDiOpenDeviceInterfaceW,
};
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    HKEY, KEY_READ, REG_BINARY, RegCloseKey, RegQueryValueExW,
};

struct DeviceInfoSet(HDEVINFO);

impl Drop for DeviceInfoSet {
    fn drop(&mut self) {
        // SAFETY: this owns the successfully created device info set.
        unsafe { SetupDiDestroyDeviceInfoList(self.0) };
    }
}

struct RegistryKey(HKEY);

impl Drop for RegistryKey {
    fn drop(&mut self) {
        // SAFETY: this owns the successfully opened registry key.
        unsafe { RegCloseKey(self.0) };
    }
}

pub(super) fn read_edid_from_registry(device_path: &str) -> Option<[u8; 128]> {
    // SAFETY: null class and parent create an empty, unfiltered local info set.
    let raw_set = unsafe { SetupDiCreateDeviceInfoList(std::ptr::null(), std::ptr::null_mut()) };
    if raw_set == -1 {
        return None;
    }
    let set = DeviceInfoSet(raw_set);
    let mut interface = SP_DEVICE_INTERFACE_DATA {
        cbSize: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
        ..Default::default()
    };
    // SAFETY: the exact interface path is NUL-terminated and outputs are initialized.
    if unsafe {
        SetupDiOpenDeviceInterfaceW(set.0, wide_null(device_path).as_ptr(), 0, &mut interface)
    } == 0
    {
        return None;
    }
    let mut required = 0;
    // SAFETY: a null detail buffer with size zero requests its required byte size.
    unsafe {
        SetupDiGetDeviceInterfaceDetailW(
            set.0,
            &interface,
            std::ptr::null_mut(),
            0,
            &mut required,
            std::ptr::null_mut(),
        )
    };
    if !(std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32..=65536)
        .contains(&required)
    {
        return None;
    }
    // usize storage provides sufficient alignment, including on 32-bit Windows.
    let mut detail = vec![0_usize; (required as usize).div_ceil(std::mem::size_of::<usize>())];
    let detail_ptr = detail
        .as_mut_ptr()
        .cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
    // SAFETY: allocation is aligned and large enough for the detail header and path.
    unsafe {
        (*detail_ptr).cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32
    };
    let mut device = SP_DEVINFO_DATA {
        cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
        ..Default::default()
    };
    // SAFETY: set/interface are valid and both output buffers have the advertised sizes.
    if unsafe {
        SetupDiGetDeviceInterfaceDetailW(
            set.0,
            &interface,
            detail_ptr,
            required,
            std::ptr::null_mut(),
            &mut device,
        )
    } == 0
    {
        return None;
    }
    // DIREG_DEV opens this exact devnode's hardware (Device Parameters) key.
    // EDID is a value inside it, not another subkey. No symbolic-link parsing.
    // SAFETY: device was resolved by SetupAPI from the selected interface in this set.
    let raw_key =
        unsafe { SetupDiOpenDevRegKey(set.0, &device, DICS_FLAG_GLOBAL, 0, DIREG_DEV, KEY_READ) };
    if raw_key as isize == -1 || raw_key.is_null() {
        return None;
    }
    let key = RegistryKey(raw_key);
    let value = wide_null("EDID");
    let mut length = 0;
    let mut value_type = 0;
    // SAFETY: null data requests the size; the type and length pointers are writable.
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            value.as_ptr(),
            std::ptr::null_mut(),
            &mut value_type,
            std::ptr::null_mut(),
            &mut length,
        )
    };
    if status != ERROR_SUCCESS || value_type != REG_BINARY || !(128..=32768).contains(&length) {
        return None;
    }
    // Read the whole value: a fixed 128-byte query fails with ERROR_MORE_DATA
    // for ordinary EDIDs with extension blocks. The public API retains block 0.
    let mut bytes = vec![0_u8; length as usize];
    // SAFETY: bytes has length writable bytes; registry changes/errors are rejected.
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            value.as_ptr(),
            std::ptr::null_mut(),
            &mut value_type,
            bytes.as_mut_ptr(),
            &mut length,
        )
    };
    if status != ERROR_SUCCESS || value_type != REG_BINARY || length < 128 {
        return None;
    }
    let base: [u8; 128] = bytes[..128].try_into().ok()?;
    parse_edid(&base)?;
    Some(base)
}

fn wide_null(value: &str) -> Vec<u16> {
    let mut wide = value.encode_utf16().collect::<Vec<_>>();
    wide.push(0);
    wide
}
