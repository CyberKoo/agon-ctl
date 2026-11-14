use super::*;
use agon_osd::{Feature, Osd, OsdError};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

type Events = Rc<RefCell<Vec<(&'static str, &'static str)>>>;

struct Mock {
    name: &'static str,
    events: Events,
    replies: VecDeque<Result<[u8; 11], TransportError>>,
    write_error: Option<TransportError>,
    write_plan: VecDeque<Result<(), TransportError>>,
}

impl Mock {
    fn new(name: &'static str, events: &Events) -> Self {
        Self {
            name,
            events: events.clone(),
            replies: VecDeque::new(),
            write_error: None,
            write_plan: VecDeque::new(),
        }
    }
}

impl agon_osd::PacketTransport for Mock {
    fn write_packet(&mut self, _: &[u8]) -> Result<(), TransportError> {
        self.events.borrow_mut().push((self.name, "write"));
        self.write_plan
            .pop_front()
            .unwrap_or_else(|| self.write_error.clone().map_or(Ok(()), Err))
    }

    fn read_packet(&mut self, response: &mut [u8]) -> Result<(), TransportError> {
        self.events.borrow_mut().push((self.name, "read"));
        let reply = self
            .replies
            .pop_front()
            .unwrap_or_else(|| Ok(frame(0x10, 0)))?;
        response.copy_from_slice(&reply);
        Ok(())
    }
}

fn frame(code: u8, result: u8) -> [u8; 11] {
    let mut bytes = [0x6e, 0x88, 0x02, result, code, 0, 0, 100, 0, 42, 0];
    bytes[10] = bytes[..10].iter().fold(0x50, |sum, byte| sum ^ byte);
    bytes
}

fn io_error() -> TransportError {
    TransportError::Io {
        operation: "mock",
        code: 1,
    }
}

fn channels(events: &Events, raw_count: usize) -> Channels<Mock, Mock> {
    Channels {
        standard: Some(Mock::new("dxva2", events)),
        raw: ["raw0", "raw1", "raw2"]
            .into_iter()
            .take(raw_count)
            .map(|name| Mock::new(name, events))
            .collect(),
        routing: Routing::default(),
    }
}

// Forward all hooks so Osd exercises the real client budget and channel routing.
struct BorrowedChannels<'a>(&'a mut Channels<Mock, Mock>);

impl DdcTransport for BorrowedChannels<'_> {
    fn read_vcp(&mut self, address: Address) -> Result<VcpValue, DdcError> {
        self.0.read_vcp(address)
    }

    fn write_vcp(&mut self, address: Address, value: u16) -> Result<(), DdcError> {
        self.0.write_vcp(address, value)
    }

    fn read_capabilities(&mut self, offset: u16) -> Result<agon_osd::CapabilityFragment, DdcError> {
        self.0.read_capabilities(offset)
    }

    fn recover_read(&mut self, address: Address, error: &DdcError) -> bool {
        self.0.recover_read(address, error)
    }

    fn recover_capabilities(&mut self, error: &DdcError) -> bool {
        self.0.recover_capabilities(error)
    }
}

fn query(channels: &mut Channels<Mock, Mock>, feature: Feature) -> Result<VcpValue, OsdError> {
    Osd::new(BorrowedChannels(channels)).read(feature)
}

#[test]
fn read_failure_reissues_whole_query_and_remembers_success() {
    let events = Events::default();
    let mut channels = channels(&events, 1);
    channels
        .standard
        .as_mut()
        .unwrap()
        .replies
        .push_back(Err(io_error()));
    let address = Address::Standard(0x10);
    let error = channels.read_vcp(address).unwrap_err();
    assert_eq!(*events.borrow(), [("dxva2", "write"), ("dxva2", "read")]);
    assert!(channels.recover_read(address, &error));
    // Recovery must not itself perform any I/O.
    assert_eq!(events.borrow().len(), 2);
    assert_eq!(channels.read_vcp(address).unwrap().current, 42);
    channels.read_vcp(address).unwrap();
    assert_eq!(
        *events.borrow(),
        [
            ("dxva2", "write"),
            ("dxva2", "read"),
            ("raw0", "write"),
            ("raw0", "read"),
            ("raw0", "write"),
            ("raw0", "read"),
        ]
    );
}

#[test]
fn refreshed_dxva2_monitor_keeps_standard_route() {
    let events = Events::default();
    let mut channels = channels(&events, 1);
    channels
        .standard
        .as_mut()
        .unwrap()
        .replies
        .push_back(Err(TransportError::Io {
            operation: "GetVCPFeatureAndVCPFeatureReply",
            code: windows_sys::Win32::Foundation::ERROR_GRAPHICS_MONITOR_NO_LONGER_EXISTS,
        }));

    assert!(query(&mut channels, Feature::Brightness).is_ok());
    assert_eq!(
        *events.borrow(),
        [
            ("dxva2", "write"),
            ("dxva2", "read"),
            ("dxva2", "write"),
            ("dxva2", "read"),
        ]
    );
}

#[test]
fn failed_dxva2_monitor_refresh_falls_back_to_raw_channel() {
    let events = Events::default();
    let mut channels = channels(&events, 1);
    channels
        .standard
        .as_mut()
        .unwrap()
        .replies
        .push_back(Err(TransportError::Io {
            operation: "dxva2 physical-monitor reenumeration found no unique match",
            code: windows_sys::Win32::Foundation::ERROR_GRAPHICS_MONITOR_NO_LONGER_EXISTS,
        }));

    assert!(query(&mut channels, Feature::Brightness).is_ok());
    assert_eq!(
        *events.borrow(),
        [
            ("dxva2", "write"),
            ("dxva2", "read"),
            ("raw0", "write"),
            ("raw0", "read"),
        ]
    );
}

#[test]
fn decode_failure_also_switches_whole_transaction() {
    let events = Events::default();
    let mut channels = channels(&events, 1);
    let mut reply = frame(0x10, 0);
    reply[10] ^= 1;
    channels
        .standard
        .as_mut()
        .unwrap()
        .replies
        .push_back(Ok(reply));
    assert!(query(&mut channels, Feature::Brightness).is_ok());
    assert_eq!(
        *events.borrow(),
        [
            ("dxva2", "write"),
            ("dxva2", "read"),
            ("raw0", "write"),
            ("raw0", "read"),
        ]
    );
}

#[test]
fn firmware_unsupported_does_not_fall_back() {
    let events = Events::default();
    let mut channels = channels(&events, 1);
    channels
        .standard
        .as_mut()
        .unwrap()
        .replies
        .push_back(Ok(frame(0x10, 1)));
    assert!(matches!(
        query(&mut channels, Feature::Brightness),
        Err(OsdError::Ddc(DdcError::Unsupported { .. }))
    ));
    assert_eq!(events.borrow().len(), 2);
}

#[test]
fn transport_unsupported_can_select_next_channel() {
    let events = Events::default();
    let mut channels = channels(&events, 1);
    channels.standard.as_mut().unwrap().write_error =
        Some(TransportError::UnsupportedFeature("mock"));
    assert!(query(&mut channels, Feature::Brightness).is_ok());
    assert_eq!(
        *events.borrow(),
        [("dxva2", "write"), ("raw0", "write"), ("raw0", "read")]
    );
}

#[test]
fn invalid_request_does_not_fall_back() {
    let events = Events::default();
    let mut channels = channels(&events, 1);
    channels.standard.as_mut().unwrap().write_error = Some(TransportError::InvalidRequest("mock"));
    assert!(query(&mut channels, Feature::Brightness).is_err());
    assert_eq!(*events.borrow(), [("dxva2", "write")]);
}

#[test]
fn caller_budget_is_not_multiplied_by_channel_count() {
    let events = Events::default();
    let mut channels = channels(&events, 3);
    channels
        .standard
        .as_mut()
        .unwrap()
        .replies
        .push_back(Err(io_error()));
    for raw in &mut channels.raw {
        raw.replies.push_back(Err(io_error()));
    }
    assert!(query(&mut channels, Feature::Brightness).is_err());
    assert_eq!(
        *events.borrow(),
        [
            ("dxva2", "write"),
            ("dxva2", "read"),
            ("raw0", "write"),
            ("raw0", "read"),
            ("raw1", "write"),
            ("raw1", "read"),
        ]
    );
}

#[test]
fn standard_and_extended_preferences_are_independent() {
    let events = Events::default();
    let mut channels = channels(&events, 2);
    channels.raw[0].replies.push_back(Err(io_error()));
    let extended = Feature::ColorGamut;
    query(&mut channels, extended).unwrap();
    query(&mut channels, Feature::Brightness).unwrap();
    query(&mut channels, extended).unwrap();
    assert_eq!(
        *events.borrow(),
        [
            ("raw0", "write"),
            ("raw0", "read"),
            ("raw1", "write"),
            ("raw1", "read"),
            ("dxva2", "write"),
            ("dxva2", "read"),
            ("raw1", "write"),
            ("raw1", "read"),
        ]
    );
}

#[test]
fn no_extended_candidate_reports_unsupported_without_io() {
    let events = Events::default();
    let mut channels = channels(&events, 0);
    let address = Address::e2a0(0x10);
    assert_eq!(
        channels.read_vcp(address),
        Err(DdcError::UnsupportedFeature { address })
    );
    assert!(!channels.recover_read(address, &DdcError::UnsupportedFeature { address }));
    assert!(events.borrow().is_empty());
}

#[test]
fn write_failure_never_falls_back_or_rewrites() {
    let events = Events::default();
    let mut channels = channels(&events, 1);
    channels.standard.as_mut().unwrap().write_error = Some(io_error());
    assert_eq!(
        channels.write_vcp(Address::Standard(0x10), 100),
        Err(DdcError::Transport(io_error()))
    );
    assert_eq!(*events.borrow(), [("dxva2", "write")]);
    assert!(channels.routing.attempt.is_none());
    assert!(channels.routing.next.is_none());
}

#[test]
fn read_vcp_uses_one_route_for_the_whole_transaction() {
    let events = Events::default();
    let mut channels = channels(&events, 2);
    channels.routing.preferred[1] = Some(Route::Raw(1));
    channels.read_vcp(Address::e2a0(0x10)).unwrap();
    assert_eq!(*events.borrow(), [("raw1", "write"), ("raw1", "read")]);
}

#[test]
fn failed_query_without_recovery_does_not_replace_successful_preference() {
    let events = Events::default();
    let mut channels = channels(&events, 1);
    let address = Address::Standard(0x10);
    channels
        .standard
        .as_mut()
        .unwrap()
        .replies
        .push_back(Err(io_error()));
    let error = channels.read_vcp(address).unwrap_err();
    assert!(channels.recover_read(address, &error));
    channels.raw[0].replies.push_back(Err(io_error()));
    assert!(channels.read_vcp(address).is_err());
    // No recovery was requested: a new transaction starts at the old preference.
    channels.read_vcp(address).unwrap();
    assert_eq!(events.borrow().last(), Some(&("dxva2", "read")));
}

#[test]
fn writes_use_remembered_preference_not_an_unproven_recovery_route() {
    let events = Events::default();
    let mut channels = channels(&events, 2);
    channels
        .standard
        .as_mut()
        .unwrap()
        .replies
        .push_back(Err(io_error()));
    query(&mut channels, Feature::Brightness).unwrap();
    let address = Address::Standard(0x10);
    channels.write_vcp(address, 100).unwrap();
    assert_eq!(events.borrow().last(), Some(&("raw0", "write")));
    channels.raw[0].replies.push_back(Err(io_error()));
    let error = channels.read_vcp(address).unwrap_err();
    assert!(channels.recover_read(address, &error));
    channels.write_vcp(address, 100).unwrap();
    assert_eq!(events.borrow().last(), Some(&("raw0", "write")));
}

#[test]
fn preferred_raw_recovers_back_to_dxva2() {
    let events = Events::default();
    let mut channels = channels(&events, 1);
    channels
        .standard
        .as_mut()
        .unwrap()
        .replies
        .push_back(Err(io_error()));
    query(&mut channels, Feature::Brightness).unwrap();
    assert_eq!(channels.routing.preferred[0], Some(Route::Raw(0)));
    events.borrow_mut().clear();

    channels.raw[0].replies.push_back(Err(io_error()));
    query(&mut channels, Feature::Brightness).unwrap();
    assert_eq!(
        *events.borrow(),
        [
            ("raw0", "write"),
            ("raw0", "read"),
            ("dxva2", "write"),
            ("dxva2", "read"),
        ]
    );
    assert_eq!(channels.routing.preferred[0], Some(Route::Standard));
}

#[test]
fn preferred_last_raw_wraps_to_raw0_without_using_dxva2_for_extended() {
    let events = Events::default();
    let mut channels = channels(&events, 2);
    channels.raw[0].replies.push_back(Err(io_error()));
    query(&mut channels, Feature::ColorGamut).unwrap();
    assert_eq!(channels.routing.preferred[1], Some(Route::Raw(1)));
    events.borrow_mut().clear();

    channels.raw[1].replies.push_back(Err(io_error()));
    query(&mut channels, Feature::ColorGamut).unwrap();
    assert_eq!(
        *events.borrow(),
        [
            ("raw1", "write"),
            ("raw1", "read"),
            ("raw0", "write"),
            ("raw0", "read"),
        ]
    );
    assert_eq!(channels.routing.preferred[1], Some(Route::Raw(0)));
}

#[test]
fn single_channel_recovers_from_io_checksum_and_invalid_response() {
    let mut checksum = frame(0x10, 0);
    checksum[10] ^= 1;
    let mut invalid = frame(0x10, 0);
    invalid[0] = 0;
    invalid[10] = invalid[..10].iter().fold(0x50, |sum, byte| sum ^ byte);
    for standard in [true, false] {
        for failure in [Err(io_error()), Ok(checksum), Ok(invalid)] {
            let events = Events::default();
            let mut channels = channels(&events, usize::from(!standard));
            let (name, mock) = if standard {
                ("dxva2", channels.standard.as_mut().unwrap())
            } else {
                channels.standard = None;
                ("raw0", &mut channels.raw[0])
            };
            mock.replies.push_back(failure);
            query(&mut channels, Feature::Brightness).unwrap();
            assert_eq!(
                *events.borrow(),
                [
                    (name, "write"),
                    (name, "read"),
                    (name, "write"),
                    (name, "read"),
                ]
            );
        }
    }
}

#[test]
fn single_transient_channel_still_has_only_three_osd_attempts() {
    let events = Events::default();
    let mut channels = channels(&events, 0);
    channels.standard.as_mut().unwrap().write_error = Some(io_error());
    assert!(matches!(
        query(&mut channels, Feature::Brightness),
        Err(OsdError::Ddc(DdcError::Transport(_)))
    ));
    assert_eq!(*events.borrow(), [("dxva2", "write"); 3]);
}

#[test]
fn single_unsupported_channel_does_not_retry() {
    // An existing dxva2 channel is not eligible for an extended query.
    for feature in [Feature::Brightness, Feature::ColorGamut] {
        let events = Events::default();
        let mut channels = channels(&events, usize::from(feature == Feature::ColorGamut));
        let name = if feature == Feature::Brightness {
            channels.standard.as_mut().unwrap().write_error =
                Some(TransportError::UnsupportedFeature("mock"));
            "dxva2"
        } else {
            channels.raw[0].write_error = Some(TransportError::UnsupportedFeature("mock"));
            "raw0"
        };
        assert!(matches!(
            query(&mut channels, feature),
            Err(OsdError::Ddc(DdcError::UnsupportedFeature { .. }))
        ));
        assert_eq!(*events.borrow(), [(name, "write")]);
    }
}

#[test]
fn no_eligible_route_clears_stale_attempt() {
    let events = Events::default();
    let mut channels = channels(&events, 0);
    let standard = Address::Standard(0x10);
    channels.standard.as_mut().unwrap().write_error = Some(io_error());
    channels.read_vcp(standard).unwrap_err();
    assert!(channels.routing.attempt.is_some());
    let extended = Feature::ColorGamut.definition().address;
    assert_eq!(
        channels.read_vcp(extended),
        Err(DdcError::UnsupportedFeature { address: extended })
    );
    assert!(channels.routing.attempt.is_none());
    assert!(channels.routing.next.is_none());
    assert!(!channels.recover_read(standard, &DdcError::Transport(io_error())));
    assert_eq!(*events.borrow(), [("dxva2", "write")]);
}

#[test]
fn osd_set_does_not_retry_or_fall_back_after_uncertain_write() {
    let events = Events::default();
    let mut channels = channels(&events, 1);
    channels.standard.as_mut().unwrap().write_plan = VecDeque::from([Ok(()), Err(io_error())]);
    let mut osd = Osd::new(channels);
    assert!(matches!(
        osd.set(Feature::Brightness, 40),
        Err(OsdError::WriteUncertain(DdcError::Transport(_)))
    ));
    assert_eq!(
        *events.borrow(),
        [("dxva2", "write"), ("dxva2", "read"), ("dxva2", "write"),]
    );
    let channels = osd.into_inner();
    assert!(channels.raw[0].write_plan.is_empty());
}

struct CapabilityMock {
    name: &'static str,
    events: Events,
}

impl CapabilityMock {
    fn new(name: &'static str, events: &Events) -> Self {
        Self {
            name,
            events: events.clone(),
        }
    }
}

impl DdcTransport for CapabilityMock {
    fn read_vcp(&mut self, address: Address) -> Result<VcpValue, DdcError> {
        Err(DdcError::UnsupportedFeature { address })
    }

    fn write_vcp(&mut self, address: Address, _value: u16) -> Result<(), DdcError> {
        Err(DdcError::UnsupportedFeature { address })
    }

    fn read_capabilities(&mut self, offset: u16) -> Result<agon_osd::CapabilityFragment, DdcError> {
        self.events.borrow_mut().push((self.name, "capabilities"));
        if self.name == "dxva2" {
            Err(DdcError::UnsupportedCapabilities)
        } else {
            Ok(agon_osd::CapabilityFragment {
                offset,
                data: vec![0, 2, 4],
            })
        }
    }
}

#[test]
fn capabilities_move_the_whole_fragment_query_to_a_raw_channel() {
    let events = Events::default();
    let mut channels = Channels::new(
        Some(CapabilityMock::new("dxva2", &events)),
        vec![CapabilityMock::new("raw0", &events)],
    );
    let error = channels.read_capabilities(0).unwrap_err();
    assert!(channels.recover_capabilities(&error));
    assert_eq!(events.borrow().len(), 1);
    let fragment = channels.read_capabilities(0).unwrap();
    assert_eq!(fragment.data, [0, 2, 4]);
    assert_eq!(
        *events.borrow(),
        [("dxva2", "capabilities"), ("raw0", "capabilities")]
    );
}
