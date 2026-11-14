use crate::error::{ApiError, INVALID_ARGUMENT};
use crate::{AgonError, AgonSession, AgonStatus, call, initialize, request_inputs};
use std::ffi::c_char;
use std::ptr;

/// An owned list of values explicitly advertised by a display.
pub struct AgonCapabilityList {
    entries: Vec<u8>,
}

/// Reads the values explicitly advertised for a feature, without local filtering.
/// # Safety
/// Session, UTF-8 inputs and output pointers must follow `agon_osd.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agon_feature_capabilities(
    session: *mut AgonSession,
    display_id: *const c_char,
    feature_name: *const c_char,
    out_values: *mut *mut AgonCapabilityList,
    out_error: *mut *mut AgonError,
) -> AgonStatus {
    // SAFETY: caller supplies live inputs and disjoint writable outputs.
    unsafe {
        call(out_error, || {
            if out_values.is_null() {
                return Err(ApiError::new(INVALID_ARGUMENT, "list output is NULL"));
            }
            out_values.write(ptr::null_mut());
            let (session, id, feature) = request_inputs(session, display_id, feature_name)?;
            let entries = session.capabilities(id, feature)?;
            out_values.write(Box::into_raw(Box::new(AgonCapabilityList { entries })));
            Ok(())
        })
    }
}

/// # Safety
/// `list` must be NULL or a live snapshot; it must not be destroyed concurrently.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agon_capability_list_count(list: *const AgonCapabilityList) -> usize {
    // SAFETY: caller supplies NULL or a live immutable snapshot.
    unsafe { list.as_ref() }.map_or(0, |list| list.entries.len())
}

/// # Safety
/// `list` must be live and `out_value` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agon_capability_list_get(
    list: *const AgonCapabilityList,
    index: usize,
    out_value: *mut u8,
) -> AgonStatus {
    // SAFETY: caller supplies a live list or NULL, and writable output storage.
    unsafe {
        call(ptr::null_mut(), || {
            initialize(out_value)?;
            let list = list
                .as_ref()
                .ok_or_else(|| ApiError::new(INVALID_ARGUMENT, "list is NULL"))?;
            let value = list
                .entries
                .get(index)
                .ok_or_else(|| ApiError::new(INVALID_ARGUMENT, "invalid capability index"))?;
            out_value.write(*value);
            Ok(())
        })
    }
}

/// # Safety
/// `list` must be NULL or a uniquely owned snapshot returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agon_capability_list_destroy(list: *mut AgonCapabilityList) {
    // SAFETY: caller transfers the snapshot allocation back exactly once.
    unsafe {
        if let Some(list) = list.as_mut() {
            drop(Box::from_raw(list));
        }
    }
}
