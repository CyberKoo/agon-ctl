use super::cf::{CfString, string_from_created};
use agon_osd_macos_sys::io_kit as bindings;
use std::ffi::{CStr, c_char};

const IO_SERVICE_PLANE: &[u8] = b"IOService\0";
const IO_REGISTRY_ITERATE_RECURSIVELY: u32 = 1;

pub(crate) struct IoObject {
    raw: u32,
}

impl IoObject {
    pub(crate) fn raw(&self) -> u32 {
        self.raw
    }

    pub(crate) fn name(&self) -> Option<String> {
        let mut buffer = [0 as c_char; 128];
        // SAFETY: `self` owns a valid IORegistry entry and the buffer is writable.
        let status = unsafe { bindings::IORegistryEntryGetName(self.raw, buffer.as_mut_ptr()) };
        if status != 0 {
            return None;
        }
        // SAFETY: a successful call writes a NUL-terminated io_name_t.
        Some(
            unsafe { CStr::from_ptr(buffer.as_ptr()) }
                .to_string_lossy()
                .into_owned(),
        )
    }

    pub(crate) fn path(&self) -> Option<String> {
        let mut buffer = [0 as c_char; 1024];
        // SAFETY: the plane is statically NUL-terminated and the output buffer is writable.
        let status = unsafe {
            bindings::IORegistryEntryGetPath(
                self.raw,
                IO_SERVICE_PLANE.as_ptr().cast(),
                buffer.as_mut_ptr(),
            )
        };
        if status != 0 {
            return None;
        }
        // SAFETY: a successful call writes a NUL-terminated io_string_t.
        Some(
            unsafe { CStr::from_ptr(buffer.as_ptr()) }
                .to_string_lossy()
                .into_owned(),
        )
    }

    pub(crate) fn property_string(&self, key: &str) -> Option<String> {
        let key = CfString::new(key)?;
        // SAFETY: the entry and CFString are valid; the returned property follows the create rule.
        let value = unsafe {
            bindings::IORegistryEntryCreateCFProperty(self.raw, key.raw(), std::ptr::null(), 0)
        };
        // SAFETY: `value` is the owned property returned by the create-rule API.
        unsafe { string_from_created(value) }
    }

    pub(crate) fn descendants(&self) -> Result<IoIterator, i32> {
        let mut iterator = 0;
        // SAFETY: the entry and plane are valid and `iterator` is writable. The returned
        // iterator is independently owned; it only traverses descendants of this entry.
        let status = unsafe {
            bindings::IORegistryEntryCreateIterator(
                self.raw,
                IO_SERVICE_PLANE.as_ptr().cast(),
                IO_REGISTRY_ITERATE_RECURSIVELY,
                &mut iterator,
            )
        };
        if status != 0 {
            return Err(status);
        }
        if iterator == 0 {
            return Err(-1);
        }
        Ok(IoIterator {
            object: IoObject { raw: iterator },
        })
    }

    pub(crate) fn parent(&self) -> Option<Self> {
        let mut parent = 0;
        // SAFETY: the plane is valid and `parent` points to writable object storage.
        let status = unsafe {
            bindings::IORegistryEntryGetParentEntry(
                self.raw,
                IO_SERVICE_PLANE.as_ptr().cast(),
                &mut parent,
            )
        };
        if status != 0 || parent == 0 {
            None
        } else {
            Some(Self { raw: parent })
        }
    }
}

impl Drop for IoObject {
    fn drop(&mut self) {
        // SAFETY: `raw` is a retained IOKit object exactly once owned by this wrapper.
        unsafe { bindings::IOObjectRelease(self.raw) };
    }
}

pub(crate) struct IoIterator {
    object: IoObject,
}

impl Iterator for IoIterator {
    type Item = IoObject;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: the iterator remains owned and valid until it is dropped.
        let raw = unsafe { bindings::IOIteratorNext(self.object.raw) };
        (raw != 0).then(|| IoObject { raw })
    }
}

pub(crate) fn registry_iterator() -> Result<IoIterator, i32> {
    // SAFETY: zero is kIOMasterPortDefault and the returned root entry is retained.
    let raw = unsafe { bindings::IORegistryGetRootEntry(0) };
    if raw == 0 {
        return Err(-1);
    }
    let root = IoObject { raw };
    root.descendants()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descendants_stay_within_target_subtree() {
        let entries: Vec<_> = registry_iterator().unwrap().collect();
        assert!(!entries.is_empty());
        // Exercise both non-leaf and leaf entries without depending on display hardware.
        for entry in [entries.first().unwrap(), entries.last().unwrap()] {
            let path = entry.path().unwrap();
            let prefix = format!("{}/", path.trim_end_matches('/'));
            let mut descendants = entry.descendants().unwrap();
            for descendant in descendants.by_ref() {
                let descendant_path = descendant.path().unwrap();
                assert!(
                    descendant_path.starts_with(&prefix),
                    "{descendant_path} is outside target subtree {path}"
                );
            }
            assert!(descendants.next().is_none());
        }
    }
}
