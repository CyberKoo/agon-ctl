use crate::error::{ApiError, INVALID_ARGUMENT};
use crate::{AgonError, AgonStatus, c_string, call, initialize};
use agon_osd::{Feature, SupportPath};
use std::ffi::{CString, c_char};
use std::ptr;
use std::sync::OnceLock;

/// A feature's support-value selection strategy.
pub type AgonSupportPath = u32;
pub(crate) const SUPPORT_PATH_NOT_APPLICABLE: AgonSupportPath = 0;
pub(crate) const SUPPORT_PATH_CAPABILITY_LIST: AgonSupportPath = 1;
pub(crate) const SUPPORT_PATH_MAXIMUM: AgonSupportPath = 2;

/// An owned list of named values selected by a display-reported maximum.
pub struct AgonNamedValueList {
    entries: Vec<(CString, u16)>,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AgonFeatureInfo {
    pub name: *const c_char,
    pub address: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AgonNamedValue {
    pub name: *const c_char,
    pub value: u16,
}

impl Default for AgonFeatureInfo {
    fn default() -> Self {
        Self {
            name: ptr::null(),
            address: ptr::null(),
        }
    }
}

impl Default for AgonNamedValue {
    fn default() -> Self {
        Self {
            name: ptr::null(),
            value: 0,
        }
    }
}

struct FeatureRecord {
    name: CString,
    address: CString,
}

fn feature_records() -> &'static [FeatureRecord] {
    static FEATURES: OnceLock<Vec<FeatureRecord>> = OnceLock::new();
    FEATURES.get_or_init(|| {
        Feature::ALL
            .iter()
            .map(|feature| {
                let definition = feature.definition();
                FeatureRecord {
                    name: c_string(definition.name),
                    address: c_string(&definition.address.to_string()),
                }
            })
            .collect()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn agon_feature_count() -> usize {
    Feature::ALL.len()
}

/// # Safety
/// `out_info` must be writable. Returned strings have library lifetime.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agon_feature_get(
    index: usize,
    out_info: *mut AgonFeatureInfo,
) -> AgonStatus {
    // SAFETY: caller supplies writable output storage.
    unsafe {
        call(ptr::null_mut(), || {
            initialize(out_info)?;
            let feature = feature_records()
                .get(index)
                .ok_or_else(|| ApiError::new(INVALID_ARGUMENT, "invalid feature index"))?;
            out_info.write(AgonFeatureInfo {
                name: feature.name.as_ptr(),
                address: feature.address.as_ptr(),
            });
            Ok(())
        })
    }
}

/// # Safety
/// `out_path` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agon_feature_support_path(
    feature_index: usize,
    out_path: *mut AgonSupportPath,
) -> AgonStatus {
    // SAFETY: caller supplies writable output storage.
    unsafe {
        call(ptr::null_mut(), || {
            initialize(out_path)?;
            let feature = Feature::ALL
                .get(feature_index)
                .ok_or_else(|| ApiError::new(INVALID_ARGUMENT, "invalid feature index"))?;
            let path = match feature.support_path() {
                Some(SupportPath::CapabilityList) => SUPPORT_PATH_CAPABILITY_LIST,
                Some(SupportPath::Maximum) => SUPPORT_PATH_MAXIMUM,
                None => SUPPORT_PATH_NOT_APPLICABLE,
            };
            out_path.write(path);
            Ok(())
        })
    }
}

/// # Safety
/// `out_values` must be writable. The returned list must be destroyed exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agon_feature_named_values(
    feature_index: usize,
    maximum: u16,
    out_values: *mut *mut AgonNamedValueList,
    out_error: *mut *mut AgonError,
) -> AgonStatus {
    // SAFETY: caller supplies writable output storage and follows the error contract.
    unsafe {
        call(out_error, || {
            if out_values.is_null() {
                return Err(ApiError::new(INVALID_ARGUMENT, "list output is NULL"));
            }
            out_values.write(ptr::null_mut());
            let definition = Feature::ALL
                .get(feature_index)
                .map(|feature| feature.definition())
                .ok_or_else(|| ApiError::new(INVALID_ARGUMENT, "invalid feature index"))?;
            let entries = definition
                .named_values(maximum)
                .iter()
                .map(|value| (c_string(value.name), value.value))
                .collect();
            out_values.write(Box::into_raw(Box::new(AgonNamedValueList { entries })));
            Ok(())
        })
    }
}

/// # Safety
/// `list` must be NULL or a live snapshot; it must not be destroyed concurrently.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agon_named_value_list_count(list: *const AgonNamedValueList) -> usize {
    // SAFETY: caller supplies NULL or a live immutable snapshot.
    unsafe { list.as_ref() }.map_or(0, |list| list.entries.len())
}

/// # Safety
/// `list` must be live and `out_value` writable. Names are borrowed until list destruction.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agon_named_value_list_get(
    list: *const AgonNamedValueList,
    index: usize,
    out_value: *mut AgonNamedValue,
) -> AgonStatus {
    // SAFETY: caller supplies a live list or NULL, and writable output storage.
    unsafe {
        call(ptr::null_mut(), || {
            initialize(out_value)?;
            let list = list
                .as_ref()
                .ok_or_else(|| ApiError::new(INVALID_ARGUMENT, "list is NULL"))?;
            let (name, value) = list
                .entries
                .get(index)
                .ok_or_else(|| ApiError::new(INVALID_ARGUMENT, "invalid named value index"))?;
            out_value.write(AgonNamedValue {
                name: name.as_ptr(),
                value: *value,
            });
            Ok(())
        })
    }
}

/// # Safety
/// `list` must be NULL or a uniquely owned snapshot returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agon_named_value_list_destroy(list: *mut AgonNamedValueList) {
    // SAFETY: caller transfers the snapshot allocation back exactly once.
    unsafe {
        if let Some(list) = list.as_mut() {
            drop(Box::from_raw(list));
        }
    }
}
