use super::*;
use agon_osd::{Address, DdcError, OsdError, SupportPath, TransportError};
use error::*;

#[test]
fn abi_version_and_value_layout_match_header() {
    assert_eq!(agon_abi_version(), 2);
    assert_eq!(std::mem::size_of::<AgonValue>(), 4);
    assert_eq!(std::mem::align_of::<AgonValue>(), 2);
    assert_eq!(
        AgonValue::from(VcpValue {
            current: 17,
            maximum: 100
        }),
        AgonValue {
            current: 17,
            maximum: 100
        }
    );
}

#[test]
fn null_handles_and_null_destructors_are_supported() {
    // SAFETY: these exports explicitly accept NULL; no handle is dereferenced.
    unsafe {
        agon_session_destroy(ptr::null_mut());
        agon_display_list_destroy(ptr::null_mut());
        agon_named_value_list_destroy(ptr::null_mut());
        agon_capability_list_destroy(ptr::null_mut());
        agon_error_destroy(ptr::null_mut());
        assert_eq!(agon_display_list_count(ptr::null()), 0);
        assert_eq!(agon_named_value_list_count(ptr::null()), 0);
        assert_eq!(agon_capability_list_count(ptr::null()), 0);
        let mut capability = 99;
        assert_eq!(
            agon_capability_list_get(ptr::null(), 0, &mut capability),
            INVALID_ARGUMENT
        );
        assert_eq!(capability, 0);
        assert_eq!(
            CStr::from_ptr(agon_error_message(ptr::null())).to_bytes(),
            b""
        );
    }
}

#[test]
fn null_session_and_output_arguments_return_owned_errors() {
    let mut previous = AgonError {
        message: c_string("previous error"),
    };
    let mut error = ptr::from_mut(&mut previous);
    let mut placeholder = AgonDisplayList {
        entries: Vec::new(),
    };
    let mut list = ptr::from_mut(&mut placeholder);
    let mut value = AgonValue {
        current: 123,
        maximum: 456,
    };
    let mut supported = 99;

    // SAFETY: all non-NULL pointers refer to live, disjoint writable storage.
    // NULL inputs are deliberate validation cases; returned errors are freed once.
    unsafe {
        assert_eq!(
            agon_session_create(ptr::null_mut(), &mut error),
            INVALID_ARGUMENT
        );
        assert!(!error.is_null());
        assert_ne!(error, ptr::from_mut(&mut previous));
        assert_eq!(
            CStr::from_ptr(agon_error_message(error)).to_str().unwrap(),
            "session output is NULL"
        );
        agon_error_destroy(error);

        assert_eq!(
            agon_session_refresh(ptr::null_mut(), &mut list, &mut error),
            INVALID_ARGUMENT
        );
        assert!(list.is_null());
        assert!(!error.is_null());
        agon_error_destroy(error);
        assert_eq!(
            agon_session_refresh(ptr::null_mut(), ptr::null_mut(), ptr::null_mut()),
            INVALID_ARGUMENT
        );

        assert_eq!(
            agon_read(
                ptr::null_mut(),
                c"display".as_ptr(),
                c"brightness".as_ptr(),
                &mut value,
                &mut error
            ),
            INVALID_ARGUMENT
        );
        assert_eq!(value, AgonValue::default());
        assert_eq!(
            CStr::from_ptr(agon_error_message(error)).to_bytes(),
            b"session is NULL"
        );
        agon_error_destroy(error);

        value = AgonValue {
            current: 123,
            maximum: 456,
        };
        assert_eq!(
            agon_set(
                ptr::null_mut(),
                c"display".as_ptr(),
                c"brightness".as_ptr(),
                42,
                &mut value,
                &mut error
            ),
            INVALID_ARGUMENT
        );
        assert_eq!(value, AgonValue::default());
        agon_error_destroy(error);

        value = AgonValue {
            current: 123,
            maximum: 456,
        };
        assert_eq!(
            agon_probe(
                ptr::null_mut(),
                c"display".as_ptr(),
                c"brightness".as_ptr(),
                &mut supported,
                &mut value,
                &mut error
            ),
            INVALID_ARGUMENT
        );
        assert_eq!(supported, 0);
        assert_eq!(value, AgonValue::default());
        agon_error_destroy(error);

        let mut capabilities = ptr::null_mut();
        assert_eq!(
            agon_feature_capabilities(
                ptr::null_mut(),
                c"display".as_ptr(),
                c"color-gamut".as_ptr(),
                &mut capabilities,
                &mut error
            ),
            INVALID_ARGUMENT
        );
        assert!(capabilities.is_null());
        assert_eq!(
            CStr::from_ptr(agon_error_message(error)).to_bytes(),
            b"session is NULL"
        );
        agon_error_destroy(error);
        assert_eq!(
            agon_feature_capabilities(
                ptr::null_mut(),
                c"display".as_ptr(),
                c"color-gamut".as_ptr(),
                ptr::null_mut(),
                &mut error
            ),
            INVALID_ARGUMENT
        );
        assert_eq!(
            CStr::from_ptr(agon_error_message(error)).to_bytes(),
            b"list output is NULL"
        );
        agon_error_destroy(error);

        assert_eq!(
            agon_read(
                ptr::null_mut(),
                ptr::null(),
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut()
            ),
            INVALID_ARGUMENT
        );
        assert_eq!(
            agon_set(
                ptr::null_mut(),
                ptr::null(),
                ptr::null(),
                0,
                ptr::null_mut(),
                ptr::null_mut()
            ),
            INVALID_ARGUMENT
        );
        assert_eq!(
            agon_probe(
                ptr::null_mut(),
                ptr::null(),
                ptr::null(),
                ptr::null_mut(),
                &mut value,
                ptr::null_mut()
            ),
            INVALID_ARGUMENT
        );
        supported = 99;
        assert_eq!(
            agon_probe(
                ptr::null_mut(),
                ptr::null(),
                ptr::null(),
                &mut supported,
                ptr::null_mut(),
                ptr::null_mut()
            ),
            INVALID_ARGUMENT
        );
        assert_eq!(supported, 0);
    }
    assert_eq!(previous.message.to_bytes(), b"previous error");
    assert!(placeholder.entries.is_empty());
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
#[test]
fn unsupported_session_creation_clears_output() {
    // This is output-only storage, never passed as an input handle or dereferenced.
    let mut placeholder = std::mem::MaybeUninit::<AgonSession>::uninit();
    let mut session = placeholder.as_mut_ptr();
    let mut error = ptr::null_mut();
    // SAFETY: outputs are live and disjoint; the placeholder pointer is overwritten.
    unsafe {
        assert_eq!(
            agon_session_create(&mut session, &mut error),
            UNSUPPORTED_PLATFORM
        );
        assert!(session.is_null());
        assert!(!error.is_null());
        assert!(
            !CStr::from_ptr(agon_error_message(error))
                .to_bytes()
                .is_empty()
        );
        agon_error_destroy(error);
        assert_eq!(
            agon_session_create(&mut session, ptr::null_mut()),
            UNSUPPORTED_PLATFORM
        );
        assert!(session.is_null());
    }
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
#[test]
fn native_session_creation_and_destruction_work_without_enumeration() {
    let mut session = ptr::null_mut();
    let mut error = ptr::null_mut();
    // SAFETY: outputs are valid and disjoint; successful creation owns the session.
    unsafe {
        assert_eq!(agon_session_create(&mut session, &mut error), OK);
        assert!(!session.is_null());
        assert!(error.is_null());
        let mut value = AgonValue {
            current: 1,
            maximum: 2,
        };
        assert_eq!(
            agon_read(
                session,
                c"missing".as_ptr(),
                c"brightness".as_ptr(),
                &mut value,
                &mut error
            ),
            DISPLAY_NOT_FOUND
        );
        assert_eq!(value, AgonValue::default());
        assert!(!error.is_null());
        agon_error_destroy(error);
        let mut capabilities = ptr::null_mut();
        assert_eq!(
            agon_feature_capabilities(
                session,
                c"missing".as_ptr(),
                c"color-gamut".as_ptr(),
                &mut capabilities,
                &mut error
            ),
            DISPLAY_NOT_FOUND
        );
        assert!(capabilities.is_null());
        assert!(!error.is_null());
        agon_error_destroy(error);
        agon_session_destroy(session);
    }
}

#[test]
fn call_resets_error_output_without_freeing_previous_error() {
    let mut previous = AgonError {
        message: c_string("still owned by caller"),
    };
    let mut error = ptr::from_mut(&mut previous);
    // SAFETY: error is writable output storage; call must not consume its old value.
    unsafe {
        assert_eq!(call(&mut error, || Ok(())), OK);
        assert!(error.is_null());
        assert_eq!(
            call(ptr::null_mut(), || Err(ApiError::new(
                INVALID_ARGUMENT,
                "ignored"
            ))),
            INVALID_ARGUMENT
        );
    }
    assert_eq!(previous.message.to_bytes(), b"still owned by caller");
}

#[test]
fn errors_are_independent_owned_messages_and_escape_nuls() {
    let mut first = ptr::null_mut();
    let mut second = ptr::null_mut();
    let mut message = String::from("first\0message — monitor");
    // SAFETY: error slots are disjoint; message pointers are read only while their
    // owning errors are live, and each allocation is destroyed exactly once.
    unsafe {
        assert_eq!(
            call(&mut first, || Err(ApiError::new(
                INVALID_ARGUMENT,
                &message
            ))),
            INVALID_ARGUMENT
        );
        message.clear();
        assert_eq!(
            call(&mut second, || Err(ApiError::internal("second"))),
            INTERNAL_ERROR
        );
        assert_ne!(first, second);
        let borrowed = agon_error_message(first);
        assert_eq!(
            CStr::from_ptr(borrowed).to_str().unwrap(),
            "first\\0message — monitor"
        );
        agon_error_destroy(second);
        assert_eq!(agon_error_message(first), borrowed);
        assert_eq!(
            CStr::from_ptr(borrowed).to_str().unwrap(),
            "first\\0message — monitor"
        );
        agon_error_destroy(first);
    }
}

#[test]
fn all_osd_error_variants_map_to_status_and_preserve_messages() {
    let address = Address::Standard(0x10);
    let ddc_errors = [
        (
            DdcError::Transport(TransportError::InvalidRequest("bad request")),
            TRANSPORT_ERROR,
        ),
        (
            DdcError::Transport(TransportError::UnexpectedEof("read")),
            TRANSPORT_ERROR,
        ),
        (
            DdcError::Transport(TransportError::Io {
                operation: "read",
                code: 5,
            }),
            TRANSPORT_ERROR,
        ),
        (
            DdcError::Transport(TransportError::UnsupportedFeature("transport unsupported")),
            TRANSPORT_ERROR,
        ),
        (DdcError::InvalidResponse("bad response"), INVALID_RESPONSE),
        (DdcError::ChecksumMismatch, INVALID_RESPONSE),
        (
            DdcError::Unsupported {
                address,
                result_code: 1,
            },
            UNSUPPORTED_FEATURE,
        ),
        (
            DdcError::UnsupportedFeature { address },
            UNSUPPORTED_FEATURE,
        ),
        (DdcError::UnsupportedCapabilities, UNSUPPORTED_FEATURE),
    ];
    let invalid_value = Feature::HdrMode
        .definition()
        .parse_value_for_maximum("not-an-hdr-mode", 3)
        .unwrap_err();
    let invalid_capabilities = agon_osd::DeviceCapabilities::parse("(").unwrap_err();
    let mut cases = vec![
        (
            OsdError::ValueAboveMaximum {
                value: 101,
                maximum: 100,
            },
            VALUE_ABOVE_MAXIMUM,
        ),
        (OsdError::InvalidValue(invalid_value), INVALID_ARGUMENT),
        (
            OsdError::InvalidCapabilities(invalid_capabilities),
            INVALID_RESPONSE,
        ),
    ];
    for (ddc, status) in ddc_errors {
        cases.push((OsdError::Ddc(ddc.clone()), status));
        cases.push((OsdError::WriteUncertain(ddc.clone()), WRITE_UNCERTAIN));
        cases.push((OsdError::ReadbackFailed(ddc), READBACK_FAILED));
    }
    for (source, expected) in cases {
        let message = source.to_string();
        let api = ApiError::from(source);
        assert_eq!(api.status, expected, "{message}");
        assert_eq!(api.message, message);
        let mut error = ptr::null_mut();
        // SAFETY: error is writable and its returned allocation remains live
        // until the message comparison, then is destroyed exactly once.
        unsafe {
            assert_eq!(call(&mut error, || Err(api)), expected);
            assert_eq!(
                CStr::from_ptr(agon_error_message(error)).to_str().unwrap(),
                message
            );
            agon_error_destroy(error);
        }
    }
}

#[test]
fn call_catches_panics_before_the_abi_boundary() {
    let mut error = ptr::null_mut();
    // SAFETY: output is writable; panic is raised in the Rust closure, not in
    // an extern C function, and call catches it before returning a status.
    unsafe {
        assert_eq!(
            call(&mut error, || panic!("test operation panic")),
            INTERNAL_ERROR
        );
        assert_eq!(
            CStr::from_ptr(agon_error_message(error)).to_bytes(),
            b"internal Rust panic"
        );
        agon_error_destroy(error);
        assert_eq!(
            call(ptr::null_mut(), || panic!("no error output")),
            INTERNAL_ERROR
        );
        assert_eq!(call(&mut error, || Ok(())), OK);
        assert!(error.is_null());
    }
}

#[test]
fn input_strings_reject_null_empty_and_invalid_utf8() {
    let invalid = [0xff_u8, 0];
    // SAFETY: non-NULL inputs are live NUL-terminated arrays. Invalid UTF-8 is
    // intentionally supplied to the parser; it is not an invalid C string.
    unsafe {
        for (input, expected) in [
            (ptr::null(), "display ID is NULL"),
            (c"".as_ptr(), "display ID is empty"),
            (invalid.as_ptr().cast(), "display ID is not UTF-8"),
        ] {
            let error = input_string(input, "display ID").unwrap_err();
            assert_eq!(error.status, INVALID_ARGUMENT);
            assert_eq!(error.message, expected);
        }
        assert_eq!(
            input_string(c"display — écran".as_ptr(), "display ID").unwrap(),
            "display — écran"
        );
        // An owned result cannot borrow the caller's temporary C string.
        let owned = {
            let temporary = CString::new("temporary display").unwrap();
            input_string(temporary.as_ptr(), "display ID").unwrap()
        };
        assert_eq!(owned, "temporary display");
    }
}

#[test]
fn all_55_features_match_core_metadata() {
    assert_eq!(agon_feature_count(), 55);
    assert_eq!(agon_feature_count(), Feature::ALL.len());
    for (index, feature) in Feature::ALL.iter().enumerate() {
        let definition = feature.definition();
        let mut info = AgonFeatureInfo::default();
        // SAFETY: outputs are writable and returned strings have library lifetime.
        unsafe {
            assert_eq!(agon_feature_get(index, &mut info), OK);
            assert_eq!(CStr::from_ptr(info.name).to_str().unwrap(), definition.name);
            assert_eq!(
                CStr::from_ptr(info.address).to_str().unwrap(),
                definition.address.to_string()
            );
            let mut path = 99;
            assert_eq!(agon_feature_support_path(index, &mut path), OK);
            let expected_path = match feature.support_path() {
                Some(SupportPath::CapabilityList) => SUPPORT_PATH_CAPABILITY_LIST,
                Some(SupportPath::Maximum) => SUPPORT_PATH_MAXIMUM,
                None => SUPPORT_PATH_NOT_APPLICABLE,
            };
            assert_eq!(path, expected_path, "{feature}");
            let mut again = AgonFeatureInfo::default();
            assert_eq!(agon_feature_get(index, &mut again), OK);
            assert_eq!(info.name, again.name);
            assert_eq!(info.address, again.address);
        }
    }
}

#[test]
fn metadata_invalid_indices_and_null_outputs_are_rejected_and_reset() {
    // SAFETY: all non-NULL outputs are writable; static sentinel strings are live.
    unsafe {
        for index in [agon_feature_count(), usize::MAX] {
            let mut info = AgonFeatureInfo {
                name: c"old".as_ptr(),
                address: c"old".as_ptr(),
            };
            assert_eq!(agon_feature_get(index, &mut info), INVALID_ARGUMENT);
            assert!(info.name.is_null());
            assert!(info.address.is_null());
        }
        let mut path = 99;
        for index in [agon_feature_count(), usize::MAX] {
            assert_eq!(
                agon_feature_support_path(index, &mut path),
                INVALID_ARGUMENT
            );
            assert_eq!(path, SUPPORT_PATH_NOT_APPLICABLE);
        }
        assert_eq!(
            agon_feature_support_path(0, ptr::null_mut()),
            INVALID_ARGUMENT
        );
        assert_eq!(agon_feature_get(0, ptr::null_mut()), INVALID_ARGUMENT);
    }
}

#[test]
fn maximum_selected_named_value_lists_own_strings_and_validate_inputs() {
    let index_of = |feature| {
        Feature::ALL
            .iter()
            .position(|item| *item == feature)
            .unwrap()
    };
    let hdr = index_of(Feature::HdrMode);
    let light_fx_mode = index_of(Feature::LightFxMode);
    let display_scaling = index_of(Feature::DisplayScaling);
    let mut error = ptr::null_mut();
    let mut first = ptr::null_mut();
    let mut second = ptr::null_mut();
    let mut named = AgonNamedValue::default();

    // SAFETY: all outputs are live and disjoint. Every returned list/error is
    // destroyed exactly once, and borrowed names are used only while their list lives.
    unsafe {
        assert_eq!(
            agon_feature_named_values(hdr, 3, &mut first, &mut error),
            OK
        );
        assert!(error.is_null());
        assert_eq!(agon_named_value_list_count(first), 4);
        assert_eq!(agon_named_value_list_get(first, 1, &mut named), OK);
        assert_eq!(CStr::from_ptr(named.name).to_str().unwrap(), "hdr-picture");
        assert_eq!(named.value, 1);

        assert_eq!(
            agon_feature_named_values(hdr, 6, &mut second, &mut error),
            OK
        );
        assert_eq!(agon_named_value_list_count(second), 7);
        assert_eq!(agon_named_value_list_get(second, 2, &mut named), OK);
        assert_eq!(CStr::from_ptr(named.name).to_str().unwrap(), "hdr-picture");
        assert_eq!(named.value, 2);
        let second_name = named.name;
        agon_named_value_list_destroy(first);
        first = ptr::null_mut();
        assert_eq!(agon_named_value_list_get(second, 2, &mut named), OK);
        assert_eq!(named.name, second_name);
        assert_eq!(CStr::from_ptr(named.name).to_str().unwrap(), "hdr-picture");
        agon_named_value_list_destroy(second);

        assert_eq!(
            agon_feature_named_values(light_fx_mode, 11, &mut first, &mut error),
            OK
        );
        assert_eq!(agon_named_value_list_count(first), 0);
        named.name = c"old".as_ptr();
        named.value = 999;
        assert_eq!(
            agon_named_value_list_get(first, 0, &mut named),
            INVALID_ARGUMENT
        );
        assert!(named.name.is_null());
        assert_eq!(named.value, 0);
        agon_named_value_list_destroy(first);
        first = ptr::null_mut();

        assert_eq!(
            agon_feature_named_values(display_scaling, 0, &mut first, &mut error),
            OK
        );
        assert_eq!(agon_named_value_list_count(first), 27);
        assert_eq!(agon_named_value_list_get(first, 0, &mut named), OK);
        assert_eq!(CStr::from_ptr(named.name).to_str().unwrap(), "ratio-1-1");
        assert_eq!(named.value, 1);
        agon_named_value_list_destroy(first);
        first = ptr::null_mut();

        assert_eq!(
            agon_feature_named_values(index_of(Feature::Brightness), 0, &mut first, &mut error),
            OK
        );
        assert_eq!(agon_named_value_list_count(first), 0);
        agon_named_value_list_destroy(first);
        first = ptr::null_mut();

        assert_eq!(
            agon_feature_named_values(agon_feature_count(), 0, &mut first, &mut error),
            INVALID_ARGUMENT
        );
        assert!(first.is_null());
        assert!(!error.is_null());
        agon_error_destroy(error);
        error = ptr::null_mut();
        assert_eq!(
            agon_feature_named_values(hdr, 3, ptr::null_mut(), &mut error),
            INVALID_ARGUMENT
        );
        agon_error_destroy(error);
        assert_eq!(
            agon_named_value_list_get(ptr::null(), 0, &mut named),
            INVALID_ARGUMENT
        );
    }
}

#[test]
fn display_snapshot_owns_labels_independently_of_source_and_other_snapshots() {
    // Exercise record conversion independently of the worker-backed ABI tests.
    let list = {
        let source = [DisplayInfo::new(
            "display\0id".into(),
            "AGON\0 — écran".into(),
            0x1234,
            0x5678,
            42,
            "port\0one".into(),
        )];
        Box::into_raw(Box::new(AgonDisplayList {
            entries: source.iter().cloned().map(DisplayRecord::from).collect(),
        }))
    };
    let mut info = AgonDisplayInfo::default();
    // SAFETY: list is a live Box allocation. Borrowed strings are only accessed
    // before its destruction. The second snapshot is separately allocated.
    unsafe {
        assert_eq!(agon_display_list_count(list), 1);
        assert_eq!(agon_display_list_get(list, 0, &mut info), OK);
        assert_eq!(CStr::from_ptr(info.id).to_bytes(), b"display\\0id");
        assert_eq!(
            CStr::from_ptr(info.name).to_str().unwrap(),
            "AGON\\0 — écran"
        );
        assert_eq!(CStr::from_ptr(info.location).to_bytes(), b"port\\0one");
        assert_eq!(
            (info.vendor_id, info.model_id, info.serial_number),
            (0x1234, 0x5678, 42)
        );
        let borrowed_name = info.name;
        let other = Box::into_raw(Box::new(AgonDisplayList {
            entries: Vec::new(),
        }));
        assert_eq!(agon_display_list_count(other), 0);
        agon_display_list_destroy(other);
        assert_eq!(
            CStr::from_ptr(borrowed_name).to_str().unwrap(),
            "AGON\\0 — écran"
        );
        assert_eq!(agon_display_list_get(list, 0, &mut info), OK);
        assert_eq!(info.name, borrowed_name);
        for index in [1, usize::MAX] {
            assert_eq!(
                agon_display_list_get(list, index, &mut info),
                INVALID_ARGUMENT
            );
            assert_display_info_reset(&info);
            assert_eq!(agon_display_list_get(list, 0, &mut info), OK);
        }
        assert_eq!(
            agon_display_list_get(ptr::null(), 0, &mut info),
            INVALID_ARGUMENT
        );
        assert_display_info_reset(&info);
        assert_eq!(
            agon_display_list_get(list, 0, ptr::null_mut()),
            INVALID_ARGUMENT
        );
        agon_display_list_destroy(list);
    }
}

mod worker_abi {
    use super::*;
    use crate::backend::Backend;
    use std::sync::{Arc, Mutex};
    use std::thread::{self, ThreadId};

    #[derive(Debug, PartialEq)]
    enum Event {
        Create,
        Refresh,
        Read(String, Feature),
        Set(String, Feature, u16),
        Probe(String, Feature),
        Capabilities(String, Feature),
        Drop,
    }

    type Events = Arc<Mutex<Vec<(ThreadId, Event)>>>;

    struct MockBackend {
        events: Events,
        owner: ThreadId,
        current: u16,
        generation: u32,
    }

    impl MockBackend {
        fn record(&self, event: Event) {
            assert_eq!(thread::current().id(), self.owner);
            self.events.lock().unwrap().push((self.owner, event));
        }

        fn check_id(&self, id: &str) -> Result<(), ApiError> {
            if id == "display-écran" {
                Ok(())
            } else {
                Err(ApiError::new(
                    DISPLAY_NOT_FOUND,
                    format!("display not found: {id}"),
                ))
            }
        }

        fn value(&self) -> VcpValue {
            VcpValue {
                current: self.current,
                maximum: 100,
            }
        }
    }

    impl Backend for MockBackend {
        fn refresh(&mut self) -> Result<Vec<DisplayInfo>, ApiError> {
            self.record(Event::Refresh);
            self.generation += 1;
            Ok(vec![DisplayInfo::new(
                "display-écran".into(),
                format!("AGON\0 — generation {}", self.generation),
                0x1234,
                0x5678,
                42,
                "port\0one".into(),
            )])
        }

        fn read(&mut self, id: &str, feature: Feature) -> Result<VcpValue, ApiError> {
            self.record(Event::Read(id.into(), feature));
            self.check_id(id)?;
            Ok(self.value())
        }

        fn set(&mut self, id: &str, feature: Feature, value: u16) -> Result<VcpValue, ApiError> {
            self.record(Event::Set(id.into(), feature, value));
            self.check_id(id)?;
            // Reserved values inject backend failures without involving hardware.
            match value {
                98 => {
                    return Err(OsdError::WriteUncertain(DdcError::Transport(
                        TransportError::Io {
                            operation: "mock write",
                            code: 5,
                        },
                    ))
                    .into());
                }
                99 => {
                    return Err(OsdError::ReadbackFailed(DdcError::InvalidResponse(
                        "mock readback failed",
                    ))
                    .into());
                }
                101.. => {
                    return Err(OsdError::ValueAboveMaximum {
                        value,
                        maximum: 100,
                    }
                    .into());
                }
                _ => {}
            }
            self.current = value;
            Ok(self.value())
        }

        fn probe(&mut self, id: &str, feature: Feature) -> Result<Option<VcpValue>, ApiError> {
            self.record(Event::Probe(id.into(), feature));
            self.check_id(id)?;
            if feature == Feature::from_name("brightness").unwrap() {
                Ok(Some(self.value()))
            } else {
                Ok(None)
            }
        }

        fn capabilities(&mut self, id: &str, feature: Feature) -> Result<Vec<u8>, ApiError> {
            self.record(Event::Capabilities(id.into(), feature));
            self.check_id(id)?;
            Ok(if feature == Feature::ColorGamut {
                vec![0, 2, 4]
            } else {
                Vec::new()
            })
        }
    }

    impl Drop for MockBackend {
        fn drop(&mut self) {
            self.record(Event::Drop);
        }
    }

    fn mock_session() -> (*mut AgonSession, Events) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let worker_events = Arc::clone(&events);
        let inner = Session::with_factory(move || {
            let backend = MockBackend {
                events: worker_events,
                owner: thread::current().id(),
                current: 17,
                generation: 0,
            };
            backend.record(Event::Create);
            Ok(backend)
        })
        .unwrap();
        (Box::into_raw(Box::new(AgonSession { inner })), events)
    }

    // SAFETY: the slot must own a live error returned by the API, or be NULL.
    unsafe fn take_error(error: &mut *mut AgonError) -> String {
        let owned = std::mem::replace(error, ptr::null_mut());
        assert!(!owned.is_null());
        // SAFETY: owned remains live while the message is copied, then is freed once.
        unsafe {
            let message = CStr::from_ptr(agon_error_message(owned))
                .to_str()
                .unwrap()
                .to_owned();
            agon_error_destroy(owned);
            message
        }
    }

    fn assert_worker_events(events: &Events, expected: &[Event]) {
        let events = events.lock().unwrap();
        assert_eq!(events.len(), expected.len());
        let worker = events[0].0;
        assert_ne!(worker, thread::current().id());
        for ((thread, event), expected) in events.iter().zip(expected) {
            assert_eq!(*thread, worker);
            assert_eq!(event, expected);
        }
    }

    #[test]
    fn exported_calls_round_trip_through_worker_and_snapshot_outlives_session() {
        let (session, events) = mock_session();
        let brightness = Feature::from_name("brightness").unwrap();
        let unsupported = Feature::ALL
            .iter()
            .copied()
            .find(|feature| *feature != brightness)
            .unwrap();
        let unsupported_name = CString::new(unsupported.definition().name).unwrap();
        let mut list = ptr::null_mut();
        let mut replacement = ptr::null_mut();
        let mut error = ptr::null_mut();
        let mut info = AgonDisplayInfo::default();
        let mut value = AgonValue::default();
        let mut supported = 99;
        let mut capabilities = ptr::null_mut();
        let mut capability = 99;
        // SAFETY: session and lists are live owned handles, all outputs are disjoint,
        // and strings are read only while their list or CString owner is live.
        unsafe {
            assert_eq!(agon_session_refresh(session, &mut list, &mut error), OK);
            assert!(error.is_null());
            assert_eq!(agon_display_list_count(list), 1);
            assert_eq!(agon_display_list_get(list, 0, &mut info), OK);
            assert_eq!(CStr::from_ptr(info.id).to_str().unwrap(), "display-écran");
            assert_eq!(
                CStr::from_ptr(info.name).to_str().unwrap(),
                "AGON\\0 — generation 1"
            );
            assert_eq!(CStr::from_ptr(info.location).to_bytes(), b"port\\0one");
            assert_eq!(
                (info.vendor_id, info.model_id, info.serial_number),
                (0x1234, 0x5678, 42)
            );
            let borrowed = info;

            assert_eq!(
                agon_read(
                    session,
                    info.id,
                    c"brightness".as_ptr(),
                    &mut value,
                    &mut error
                ),
                OK
            );
            assert!(error.is_null());
            assert_eq!(
                value,
                AgonValue {
                    current: 17,
                    maximum: 100
                }
            );
            assert_eq!(
                agon_set(
                    session,
                    info.id,
                    c"brightness".as_ptr(),
                    63,
                    &mut value,
                    &mut error
                ),
                OK
            );
            assert!(error.is_null());
            assert_eq!(
                value,
                AgonValue {
                    current: 63,
                    maximum: 100
                }
            );
            assert_eq!(
                agon_read(
                    session,
                    info.id,
                    c"brightness".as_ptr(),
                    &mut value,
                    &mut error
                ),
                OK
            );
            assert_eq!(value.current, 63);
            assert!(error.is_null());
            assert_eq!(
                agon_probe(
                    session,
                    info.id,
                    c"brightness".as_ptr(),
                    &mut supported,
                    &mut value,
                    &mut error
                ),
                OK
            );
            assert!(error.is_null());
            assert_eq!(supported, 1);
            assert_eq!(
                value,
                AgonValue {
                    current: 63,
                    maximum: 100
                }
            );
            assert_eq!(
                agon_probe(
                    session,
                    info.id,
                    unsupported_name.as_ptr(),
                    &mut supported,
                    &mut value,
                    &mut error
                ),
                OK
            );
            assert!(error.is_null());
            assert_eq!(supported, 0);
            assert_eq!(value, AgonValue::default());

            assert_eq!(
                agon_feature_capabilities(
                    session,
                    info.id,
                    c"color-gamut".as_ptr(),
                    &mut capabilities,
                    &mut error
                ),
                OK
            );
            assert!(error.is_null());
            assert_eq!(agon_capability_list_count(capabilities), 3);

            assert_eq!(
                agon_session_refresh(session, &mut replacement, &mut error),
                OK
            );
            assert!(error.is_null());
            assert_eq!(agon_display_list_get(replacement, 0, &mut info), OK);
            assert_eq!(
                CStr::from_ptr(info.name).to_str().unwrap(),
                "AGON\\0 — generation 2"
            );
            agon_display_list_destroy(replacement);
            agon_session_destroy(session);
            // Destruction joins backend drop before returning; no session use follows.
            assert_worker_events(
                &events,
                &[
                    Event::Create,
                    Event::Refresh,
                    Event::Read("display-écran".into(), brightness),
                    Event::Set("display-écran".into(), brightness, 63),
                    Event::Read("display-écran".into(), brightness),
                    Event::Probe("display-écran".into(), brightness),
                    Event::Probe("display-écran".into(), unsupported),
                    Event::Capabilities("display-écran".into(), Feature::ColorGamut),
                    Event::Refresh,
                    Event::Drop,
                ],
            );
            assert_eq!(
                CStr::from_ptr(borrowed.name).to_str().unwrap(),
                "AGON\\0 — generation 1"
            );
            assert_eq!(
                CStr::from_ptr(borrowed.id).to_str().unwrap(),
                "display-écran"
            );
            assert_eq!(CStr::from_ptr(borrowed.location).to_bytes(), b"port\\0one");
            assert_eq!(agon_display_list_count(list), 1);
            assert_eq!(agon_display_list_get(list, 0, &mut info), OK);
            assert_eq!(info.name, borrowed.name);
            assert_eq!(info.id, borrowed.id);
            agon_display_list_destroy(list);
            for (index, expected) in [0, 2, 4].into_iter().enumerate() {
                assert_eq!(
                    agon_capability_list_get(capabilities, index, &mut capability),
                    OK
                );
                assert_eq!(capability, expected);
            }
            capability = 99;
            assert_eq!(
                agon_capability_list_get(capabilities, 3, &mut capability),
                INVALID_ARGUMENT
            );
            assert_eq!(capability, 0);
            agon_capability_list_destroy(capabilities);
        }
    }

    #[test]
    fn exported_input_validation_precedes_worker_dispatch_and_invalid_ids_return_errors() {
        let (session, events) = mock_session();
        let invalid_utf8 = [0xff_u8, 0];
        let id = c"display-écran".as_ptr();
        let feature = c"brightness".as_ptr();
        let mut error = ptr::null_mut();
        let cases = [
            (ptr::null(), feature, INVALID_ARGUMENT, "display ID is NULL"),
            (
                c"".as_ptr(),
                feature,
                INVALID_ARGUMENT,
                "display ID is empty",
            ),
            (
                invalid_utf8.as_ptr().cast(),
                feature,
                INVALID_ARGUMENT,
                "display ID is not UTF-8",
            ),
            (id, ptr::null(), INVALID_ARGUMENT, "feature name is NULL"),
            (id, c"".as_ptr(), INVALID_ARGUMENT, "feature name is empty"),
            (
                id,
                invalid_utf8.as_ptr().cast(),
                INVALID_ARGUMENT,
                "feature name is not UTF-8",
            ),
            (
                id,
                c"not-a-feature".as_ptr(),
                INVALID_ARGUMENT,
                "unknown feature: not-a-feature",
            ),
            (
                c"missing".as_ptr(),
                feature,
                DISPLAY_NOT_FOUND,
                "display not found: missing",
            ),
        ];
        // SAFETY: session is live; inputs are NULL or live NUL-terminated arrays,
        // including deliberate invalid UTF-8. Outputs never overlap.
        unsafe {
            for (id, feature, expected, message) in cases {
                for operation in ["read", "set", "probe", "capabilities"] {
                    let before = events.lock().unwrap().len();
                    let mut value = AgonValue {
                        current: 123,
                        maximum: 456,
                    };
                    let mut supported = 99;
                    let mut capabilities = ptr::null_mut();
                    let status = match operation {
                        "read" => agon_read(session, id, feature, &mut value, &mut error),
                        "set" => agon_set(session, id, feature, 42, &mut value, &mut error),
                        "probe" => {
                            agon_probe(session, id, feature, &mut supported, &mut value, &mut error)
                        }
                        "capabilities" => agon_feature_capabilities(
                            session,
                            id,
                            feature,
                            &mut capabilities,
                            &mut error,
                        ),
                        _ => unreachable!(),
                    };
                    assert_eq!(status, expected, "{operation}: {message}");
                    if operation == "capabilities" {
                        assert_eq!(
                            value,
                            AgonValue {
                                current: 123,
                                maximum: 456,
                            }
                        );
                    } else {
                        assert_eq!(value, AgonValue::default());
                    }
                    assert!(capabilities.is_null());
                    if operation == "probe" {
                        assert_eq!(supported, 0);
                    }
                    assert_eq!(take_error(&mut error), message);
                    assert_eq!(
                        events.lock().unwrap().len(),
                        before + usize::from(expected == DISPLAY_NOT_FOUND)
                    );
                }
            }
            // A backend error does not stop the worker or poison later calls.
            let mut value = AgonValue::default();
            assert_eq!(agon_read(session, id, feature, &mut value, &mut error), OK);
            assert!(error.is_null());
            assert_eq!(value.current, 17);
            agon_session_destroy(session);
        }
        let brightness = Feature::from_name("brightness").unwrap();
        assert_worker_events(
            &events,
            &[
                Event::Create,
                Event::Read("missing".into(), brightness),
                Event::Set("missing".into(), brightness, 42),
                Event::Probe("missing".into(), brightness),
                Event::Capabilities("missing".into(), brightness),
                Event::Read("display-écran".into(), brightness),
                Event::Drop,
            ],
        );
    }

    #[test]
    fn exported_set_preserves_write_uncertain_and_readback_failed_statuses() {
        let (session, events) = mock_session();
        let mut error = ptr::null_mut();
        let id = c"display-écran".as_ptr();
        let feature = c"brightness".as_ptr();
        // SAFETY: handle and C strings remain live; outputs are writable/disjoint,
        // and each returned error is consumed before its output slot is reused.
        unsafe {
            for (input, expected, message) in [
                (
                    98,
                    WRITE_UNCERTAIN,
                    OsdError::WriteUncertain(DdcError::Transport(TransportError::Io {
                        operation: "mock write",
                        code: 5,
                    }))
                    .to_string(),
                ),
                (
                    99,
                    READBACK_FAILED,
                    OsdError::ReadbackFailed(DdcError::InvalidResponse("mock readback failed"))
                        .to_string(),
                ),
                (
                    101,
                    VALUE_ABOVE_MAXIMUM,
                    OsdError::ValueAboveMaximum {
                        value: 101,
                        maximum: 100,
                    }
                    .to_string(),
                ),
            ] {
                let mut value = AgonValue {
                    current: 123,
                    maximum: 456,
                };
                assert_eq!(
                    agon_set(session, id, feature, input, &mut value, &mut error),
                    expected
                );
                assert_eq!(value, AgonValue::default());
                assert_eq!(take_error(&mut error), message);
            }
            let mut value = AgonValue::default();
            assert_eq!(
                agon_set(session, id, feature, 52, &mut value, &mut error),
                OK
            );
            assert!(error.is_null());
            assert_eq!(
                value,
                AgonValue {
                    current: 52,
                    maximum: 100
                }
            );
            assert_eq!(agon_read(session, id, feature, &mut value, &mut error), OK);
            assert!(error.is_null());
            assert_eq!(value.current, 52);
            agon_session_destroy(session);
        }
        let brightness = Feature::from_name("brightness").unwrap();
        assert_worker_events(
            &events,
            &[
                Event::Create,
                Event::Set("display-écran".into(), brightness, 98),
                Event::Set("display-écran".into(), brightness, 99),
                Event::Set("display-écran".into(), brightness, 101),
                Event::Set("display-écran".into(), brightness, 52),
                Event::Read("display-écran".into(), brightness),
                Event::Drop,
            ],
        );
    }
}

fn assert_display_info_reset(info: &AgonDisplayInfo) {
    assert!(info.id.is_null());
    assert!(info.name.is_null());
    assert!(info.location.is_null());
    assert_eq!(
        (info.vendor_id, info.model_id, info.serial_number),
        (0, 0, 0)
    );
}
