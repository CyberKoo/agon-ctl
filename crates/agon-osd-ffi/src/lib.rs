//! C ABI for native applications. See `include/agon_osd.h` for ownership rules.

mod backend;
mod capability;
mod error;
mod feature;
mod session;

pub use capability::*;
pub use feature::*;

use agon_osd::{DisplayInfo, Feature, VcpValue};
use error::{ApiError, INVALID_ARGUMENT, OK};
use session::Session;
use std::ffi::{CStr, CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;

/// Opaque session owning a serialized hardware worker.
pub struct AgonSession {
    inner: Session,
}

/// Owned snapshot, independent of future refreshes and session destruction.
pub struct AgonDisplayList {
    entries: Vec<DisplayRecord>,
}

/// An owned, per-call UTF-8 error message.
pub struct AgonError {
    message: CString,
}

/// A function's numeric status. Values are declared in `agon_osd.h`.
pub type AgonStatus = u32;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AgonValue {
    pub current: u16,
    pub maximum: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AgonDisplayInfo {
    pub id: *const c_char,
    pub name: *const c_char,
    pub location: *const c_char,
    pub vendor_id: u32,
    pub model_id: u32,
    pub serial_number: u32,
}

impl Default for AgonDisplayInfo {
    fn default() -> Self {
        Self {
            id: ptr::null(),
            name: ptr::null(),
            location: ptr::null(),
            vendor_id: 0,
            model_id: 0,
            serial_number: 0,
        }
    }
}

impl From<VcpValue> for AgonValue {
    fn from(value: VcpValue) -> Self {
        Self {
            current: value.current,
            maximum: value.maximum,
        }
    }
}

struct DisplayRecord {
    id: CString,
    name: CString,
    location: CString,
    vendor_id: u32,
    model_id: u32,
    serial_number: u32,
}

impl From<DisplayInfo> for DisplayRecord {
    fn from(info: DisplayInfo) -> Self {
        Self {
            id: c_string(info.id()),
            name: c_string(info.name()),
            location: c_string(info.location()),
            vendor_id: info.vendor_id(),
            model_id: info.model_id(),
            serial_number: info.serial_number(),
        }
    }
}

pub(crate) fn c_string(value: &str) -> CString {
    // Device labels can contain embedded NULs; preserve the rest of the label.
    CString::new(value.replace('\0', "\\0")).expect("embedded NULs were escaped")
}

// All callers are exports with the same pointer validity contract as the C header.
unsafe fn initialize<T: Default>(output: *mut T) -> Result<(), ApiError> {
    if output.is_null() {
        return Err(ApiError::new(INVALID_ARGUMENT, "output pointer is NULL"));
    }
    // SAFETY: checked non-null; caller supplies writable, aligned output storage.
    unsafe { output.write(T::default()) };
    Ok(())
}

unsafe fn input_string(input: *const c_char, label: &str) -> Result<String, ApiError> {
    if input.is_null() {
        return Err(ApiError::new(INVALID_ARGUMENT, format!("{label} is NULL")));
    }
    // SAFETY: caller supplies a readable NUL-terminated string for this call.
    let value = unsafe { CStr::from_ptr(input) }
        .to_str()
        .map_err(|_| ApiError::new(INVALID_ARGUMENT, format!("{label} is not UTF-8")))?;
    if value.is_empty() {
        return Err(ApiError::new(INVALID_ARGUMENT, format!("{label} is empty")));
    }
    Ok(value.to_owned())
}

unsafe fn request_inputs<'a>(
    session: *mut AgonSession,
    id: *const c_char,
    feature: *const c_char,
) -> Result<(&'a Session, String, Feature), ApiError> {
    // SAFETY: a non-null session must be a live handle for the duration of the call.
    let session = unsafe { session.as_ref() }
        .ok_or_else(|| ApiError::new(INVALID_ARGUMENT, "session is NULL"))?;
    // SAFETY: input validity is part of this export's contract.
    let id = unsafe { input_string(id, "display ID") }?;
    // SAFETY: input validity is part of this export's contract.
    let name = unsafe { input_string(feature, "feature name") }?;
    let feature = Feature::from_name(&name)
        .ok_or_else(|| ApiError::new(INVALID_ARGUMENT, format!("unknown feature: {name}")))?;
    Ok((&session.inner, id, feature))
}

unsafe fn call(
    out_error: *mut *mut AgonError,
    operation: impl FnOnce() -> Result<(), ApiError>,
) -> AgonStatus {
    if !out_error.is_null() {
        // SAFETY: caller supplies writable storage, separate from other arguments.
        unsafe { out_error.write(ptr::null_mut()) };
    }
    let result = catch_unwind(AssertUnwindSafe(operation))
        .unwrap_or_else(|_| Err(ApiError::internal("internal Rust panic")));
    match result {
        Ok(()) => OK,
        Err(error) => {
            if !out_error.is_null() {
                let owned = Box::new(AgonError {
                    message: c_string(&error.message),
                });
                // SAFETY: caller owns this returned allocation and frees it via agon_error_destroy.
                unsafe { out_error.write(Box::into_raw(owned)) };
            }
            error.status
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn agon_abi_version() -> u32 {
    2
}

/// Creates a session and its dedicated hardware thread.
/// # Safety
/// Output pointers must satisfy the writable, non-overlapping contract in `agon_osd.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agon_session_create(
    out_session: *mut *mut AgonSession,
    out_error: *mut *mut AgonError,
) -> AgonStatus {
    // SAFETY: all pointer operations follow the caller's output contract.
    unsafe {
        call(out_error, || {
            if out_session.is_null() {
                return Err(ApiError::new(INVALID_ARGUMENT, "session output is NULL"));
            }
            out_session.write(ptr::null_mut());
            let inner = Session::new()?;
            out_session.write(Box::into_raw(Box::new(AgonSession { inner })));
            Ok(())
        })
    }
}

/// Joins the worker and destroys its resources. NULL is accepted.
/// # Safety
/// The handle must be live, uniquely owned, and have no concurrent callers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agon_session_destroy(session: *mut AgonSession) {
    if !session.is_null() {
        // SAFETY: this consumes the allocation returned by agon_session_create once.
        let _ = catch_unwind(AssertUnwindSafe(|| unsafe { drop(Box::from_raw(session)) }));
    }
}

/// Refreshes device handles and returns an owned snapshot.
/// # Safety
/// Handles, outputs and lifetimes must follow `agon_osd.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agon_session_refresh(
    session: *mut AgonSession,
    out_list: *mut *mut AgonDisplayList,
    out_error: *mut *mut AgonError,
) -> AgonStatus {
    // SAFETY: pointer validity and disjoint outputs are required by the caller contract.
    unsafe {
        call(out_error, || {
            if out_list.is_null() {
                return Err(ApiError::new(INVALID_ARGUMENT, "list output is NULL"));
            }
            out_list.write(ptr::null_mut());
            let session = session
                .as_ref()
                .ok_or_else(|| ApiError::new(INVALID_ARGUMENT, "session is NULL"))?;
            let entries = session
                .inner
                .refresh()?
                .into_iter()
                .map(DisplayRecord::from)
                .collect();
            out_list.write(Box::into_raw(Box::new(AgonDisplayList { entries })));
            Ok(())
        })
    }
}

/// # Safety
/// `list` must be NULL or a live snapshot; it must not be destroyed concurrently.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agon_display_list_count(list: *const AgonDisplayList) -> usize {
    // SAFETY: caller supplies NULL or a live immutable snapshot.
    unsafe { list.as_ref() }.map_or(0, |list| list.entries.len())
}

/// # Safety
/// `list` must be live and `out_info` writable. Strings are borrowed until list destruction.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agon_display_list_get(
    list: *const AgonDisplayList,
    index: usize,
    out_info: *mut AgonDisplayInfo,
) -> AgonStatus {
    // SAFETY: output validity and the snapshot lifetime are guaranteed by the caller.
    unsafe {
        call(ptr::null_mut(), || {
            initialize(out_info)?;
            let entry = list
                .as_ref()
                .and_then(|list| list.entries.get(index))
                .ok_or_else(|| ApiError::new(INVALID_ARGUMENT, "invalid display list or index"))?;
            out_info.write(AgonDisplayInfo {
                id: entry.id.as_ptr(),
                name: entry.name.as_ptr(),
                location: entry.location.as_ptr(),
                vendor_id: entry.vendor_id,
                model_id: entry.model_id,
                serial_number: entry.serial_number,
            });
            Ok(())
        })
    }
}

/// # Safety
/// `list` must be NULL or a uniquely owned snapshot returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agon_display_list_destroy(list: *mut AgonDisplayList) {
    if !list.is_null() {
        // SAFETY: caller transfers the snapshot allocation back exactly once.
        unsafe { drop(Box::from_raw(list)) };
    }
}

/// # Safety
/// Session, UTF-8 input strings and output pointers must follow `agon_osd.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agon_read(
    session: *mut AgonSession,
    display_id: *const c_char,
    feature_name: *const c_char,
    out_value: *mut AgonValue,
    out_error: *mut *mut AgonError,
) -> AgonStatus {
    // SAFETY: caller supplies live inputs and disjoint writable outputs.
    unsafe {
        call(out_error, || {
            initialize(out_value)?;
            let (session, id, feature) = request_inputs(session, display_id, feature_name)?;
            out_value.write(session.read(id, feature)?.into());
            Ok(())
        })
    }
}

/// # Safety
/// Session, UTF-8 input strings and output pointers must follow `agon_osd.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agon_set(
    session: *mut AgonSession,
    display_id: *const c_char,
    feature_name: *const c_char,
    value: u16,
    out_value: *mut AgonValue,
    out_error: *mut *mut AgonError,
) -> AgonStatus {
    // SAFETY: caller supplies live inputs and disjoint writable outputs.
    unsafe {
        call(out_error, || {
            initialize(out_value)?;
            let (session, id, feature) = request_inputs(session, display_id, feature_name)?;
            out_value.write(session.set(id, feature, value)?.into());
            Ok(())
        })
    }
}

/// # Safety
/// Session, UTF-8 inputs and both disjoint output pointers must follow `agon_osd.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agon_probe(
    session: *mut AgonSession,
    display_id: *const c_char,
    feature_name: *const c_char,
    out_supported: *mut u8,
    out_value: *mut AgonValue,
    out_error: *mut *mut AgonError,
) -> AgonStatus {
    // SAFETY: caller supplies live inputs and disjoint writable outputs.
    unsafe {
        call(out_error, || {
            initialize(out_supported)?;
            initialize(out_value)?;
            let (session, id, feature) = request_inputs(session, display_id, feature_name)?;
            if let Some(value) = session.probe(id, feature)? {
                out_value.write(value.into());
                out_supported.write(1);
            }
            Ok(())
        })
    }
}

/// # Safety
/// `error` must be NULL or a live error object. The returned message is borrowed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agon_error_message(error: *const AgonError) -> *const c_char {
    // SAFETY: caller supplies NULL or a live immutable error object.
    unsafe { error.as_ref() }.map_or(c"".as_ptr(), |error| error.message.as_ptr())
}

/// # Safety
/// `error` must be NULL or a uniquely owned error returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agon_error_destroy(error: *mut AgonError) {
    if !error.is_null() {
        // SAFETY: caller transfers this allocation back exactly once.
        unsafe { drop(Box::from_raw(error)) };
    }
}

#[cfg(test)]
mod tests;
