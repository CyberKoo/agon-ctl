use agon_osd_macos_sys::cf::{
    CFDictionaryGetTypeID, CFDictionaryGetValue, CFGetTypeID, CFRelease, CFStringCreateWithCString,
    CFStringGetCString, CFStringGetLength, CFStringGetMaximumSizeForEncoding, CFStringGetTypeID,
};
use std::ffi::{CStr, c_char};

pub(crate) use agon_osd_macos_sys::cf::CfRef;

const CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;

pub(crate) struct OwnedCf {
    raw: CfRef,
}

impl OwnedCf {
    /// # Safety
    /// `raw` must be null or a valid CoreFoundation object owned under the create rule.
    pub(crate) unsafe fn from_created(raw: CfRef) -> Option<Self> {
        (!raw.is_null()).then(|| Self { raw })
    }

    pub(crate) fn as_ref(&self) -> CfRef {
        self.raw
    }
}

impl Drop for OwnedCf {
    fn drop(&mut self) {
        // SAFETY: `raw` is a non-null CoreFoundation object owned under the create rule.
        unsafe { CFRelease(self.raw) };
    }
}

pub(crate) struct CfString {
    _owned: OwnedCf,
    raw: CfRef,
}

impl CfString {
    pub(crate) fn new(value: &str) -> Option<Self> {
        let Ok(value) = std::ffi::CString::new(value) else {
            return None;
        };
        // SAFETY: the CString is NUL-terminated and alive for the duration of the call.
        let raw = unsafe {
            CFStringCreateWithCString(std::ptr::null(), value.as_ptr(), CF_STRING_ENCODING_UTF8)
        };
        // SAFETY: a successful CFStringCreateWithCString result follows the create rule.
        let owned = unsafe { OwnedCf::from_created(raw)? };
        Some(Self { _owned: owned, raw })
    }

    pub(crate) fn raw(&self) -> CfRef {
        self.raw
    }
}

pub(crate) struct CfDictionary {
    _owned: OwnedCf,
    raw: CfRef,
}

impl CfDictionary {
    pub(crate) unsafe fn from_created(raw: CfRef) -> Option<Self> {
        // SAFETY: the caller guarantees that `raw` is an owned CF object or null.
        let owned = unsafe { OwnedCf::from_created(raw)? };
        // SAFETY: `raw` is non-null; checking its dynamic type validates the dictionary contract.
        if unsafe { CFGetTypeID(raw) } != unsafe { CFDictionaryGetTypeID() } {
            return None;
        }
        Some(Self { _owned: owned, raw })
    }

    pub(crate) fn string(&self, key: &str) -> Option<String> {
        string_from_dictionary(self.raw, key)
    }

    pub(crate) fn nested_string(&self, key: &str, nested_key: &str) -> Option<String> {
        let dictionary = typed_value(self.raw, key, unsafe { CFDictionaryGetTypeID() })?;
        string_from_dictionary(dictionary, nested_key)
    }
}

fn string_from_dictionary(dictionary: CfRef, key: &str) -> Option<String> {
    // SAFETY: CFStringGetTypeID takes no arguments and has no memory safety preconditions.
    let value = typed_value(dictionary, key, unsafe { CFStringGetTypeID() })?;
    string_value(value)
}

fn typed_value(dictionary: CfRef, key: &str, expected_type: usize) -> Option<CfRef> {
    let key = CfString::new(key)?;
    // SAFETY: both references are valid CoreFoundation objects for this call.
    let value = unsafe { CFDictionaryGetValue(dictionary, key.raw) };
    if value.is_null() {
        return None;
    }
    // SAFETY: `value` is a non-null CoreFoundation object retained by the dictionary.
    (unsafe { CFGetTypeID(value) } == expected_type).then_some(value)
}

fn string_value(value: CfRef) -> Option<String> {
    // SAFETY: `value` has been dynamically verified as a CFString.
    let length = unsafe { CFStringGetLength(value) };
    // SAFETY: `value` remains alive while its maximum UTF-8 buffer size is calculated.
    let capacity = unsafe { CFStringGetMaximumSizeForEncoding(length, CF_STRING_ENCODING_UTF8) };
    let capacity = usize::try_from(capacity).ok()?.checked_add(1)?;
    let buffer_size = i64::try_from(capacity).ok()?;
    let mut buffer = vec![0 as c_char; capacity];
    // SAFETY: the buffer has `capacity` writable bytes and the source is a valid CFString.
    let converted = unsafe {
        CFStringGetCString(
            value,
            buffer.as_mut_ptr(),
            buffer_size,
            CF_STRING_ENCODING_UTF8,
        )
    };
    if converted == 0 {
        return None;
    }
    // SAFETY: a successful CFStringGetCString call NUL-terminates the output buffer.
    Some(
        unsafe { CStr::from_ptr(buffer.as_ptr()) }
            .to_string_lossy()
            .into_owned(),
    )
}

pub(crate) unsafe fn string_from_created(value: CfRef) -> Option<String> {
    // SAFETY: the caller guarantees that `value` is an owned CFTypeRef or null.
    let owned = unsafe { OwnedCf::from_created(value)? };
    // SAFETY: `value` is non-null and owned for the duration of this function.
    if unsafe { CFGetTypeID(value) } != unsafe { CFStringGetTypeID() } {
        return None;
    }
    string_value(owned.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_created_objects_are_rejected_without_release() {
        // SAFETY: all three constructors explicitly accept null.
        unsafe {
            assert!(OwnedCf::from_created(std::ptr::null()).is_none());
            assert!(CfDictionary::from_created(std::ptr::null()).is_none());
            assert!(string_from_created(std::ptr::null()).is_none());
        }
    }

    #[test]
    fn created_string_is_owned_and_converted() {
        let value = std::ffi::CString::new("AGON 显示器").unwrap();
        // SAFETY: the CString is NUL-terminated and alive during the call.
        let raw = unsafe {
            CFStringCreateWithCString(std::ptr::null(), value.as_ptr(), CF_STRING_ENCODING_UTF8)
        };
        assert!(!raw.is_null());
        // SAFETY: ownership of the newly created CFString is transferred to the wrapper.
        let owned = unsafe { OwnedCf::from_created(raw) }.unwrap();
        assert_eq!(owned.as_ref(), raw);
        assert_eq!(string_value(owned.as_ref()).as_deref(), Some("AGON 显示器"));
        drop(owned);
    }

    #[test]
    fn wrong_dictionary_type_is_released_and_rejected() {
        let value = std::ffi::CString::new("not a dictionary").unwrap();
        // SAFETY: the CString is NUL-terminated and alive during the call.
        let raw = unsafe {
            CFStringCreateWithCString(std::ptr::null(), value.as_ptr(), CF_STRING_ENCODING_UTF8)
        };
        assert!(!raw.is_null());
        // SAFETY: this is an owned CF object; the constructor checks its dynamic type.
        assert!(unsafe { CfDictionary::from_created(raw) }.is_none());
    }
}
