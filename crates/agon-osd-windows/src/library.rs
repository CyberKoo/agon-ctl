use crate::WindowsError;
use std::ffi::{CStr, c_void};
use windows_sys::Win32::Foundation::FreeLibrary;
use windows_sys::Win32::System::LibraryLoader::{
    GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW,
};

pub(crate) struct Library {
    provider: &'static str,
    handle: *mut c_void,
}

impl Library {
    pub(crate) fn load(provider: &'static str, names: &[&str]) -> Result<Self, WindowsError> {
        let mut last_error = None;
        for name in names {
            let mut wide = [0_u16; 260];
            let mut length = 0;
            for (index, unit) in name.encode_utf16().enumerate() {
                if index + 1 >= wide.len() {
                    return Err(WindowsError::InvalidData("SDK library name is too long"));
                }
                wide[index] = unit;
                length = index + 1;
            }
            wide[length] = 0;
            // SAFETY: `wide` is NUL-terminated, and the System32 search flag prevents path hijacking.
            let handle = unsafe {
                LoadLibraryExW(
                    wide.as_ptr(),
                    std::ptr::null_mut(),
                    LOAD_LIBRARY_SEARCH_SYSTEM32,
                )
            };
            if !handle.is_null() {
                return Ok(Self { provider, handle });
            }
            last_error = Some(std::io::Error::last_os_error());
        }
        Err(WindowsError::Library {
            provider,
            source: last_error.unwrap_or_else(std::io::Error::last_os_error),
        })
    }

    pub(crate) fn symbol<T: Copy>(&self, name: &'static CStr) -> Result<T, WindowsError> {
        // SAFETY: the module remains loaded for the lifetime of the returned function pointer.
        let address = unsafe { GetProcAddress(self.handle, name.as_ptr().cast()) };
        if address.is_none() {
            return Err(WindowsError::MissingSymbol {
                provider: self.provider,
                symbol: name.to_str().unwrap_or("<non-UTF-8 symbol>"),
            });
        }
        // SAFETY: the caller chooses T to match the documented SDK function signature.
        Ok(unsafe { std::mem::transmute_copy(&address) })
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        // SAFETY: the handle is owned by this object and is released once.
        unsafe { FreeLibrary(self.handle) };
    }
}
