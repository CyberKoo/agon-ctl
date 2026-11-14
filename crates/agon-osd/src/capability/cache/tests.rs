use super::*;
use crate::{Address, CapabilityFragment, VcpValue};
use std::collections::VecDeque;

struct FragmentTransport {
    script: VecDeque<Result<CapabilityFragment, DdcError>>,
    offsets: Vec<u16>,
}

impl FragmentTransport {
    fn new(script: Vec<Result<CapabilityFragment, DdcError>>) -> Self {
        Self {
            script: script.into(),
            offsets: Vec::new(),
        }
    }
}

impl DdcTransport for FragmentTransport {
    fn read_vcp(&mut self, address: Address) -> Result<VcpValue, DdcError> {
        Err(DdcError::UnsupportedFeature { address })
    }

    fn write_vcp(&mut self, address: Address, _value: u16) -> Result<(), DdcError> {
        Err(DdcError::UnsupportedFeature { address })
    }

    fn read_capabilities(&mut self, offset: u16) -> Result<CapabilityFragment, DdcError> {
        self.offsets.push(offset);
        self.script
            .pop_front()
            .unwrap_or(Err(DdcError::InvalidResponse(
                "unexpected capabilities fragment request",
            )))
    }
}

fn fragment(offset: u16, data: &[u8]) -> Result<CapabilityFragment, DdcError> {
    Ok(CapabilityFragment {
        offset,
        data: data.to_vec(),
    })
}

fn raw_script(raw: &str) -> (Vec<Result<CapabilityFragment, DdcError>>, Vec<u16>) {
    let mut script = Vec::new();
    let mut offsets = Vec::new();
    let mut offset = 0_u16;
    for chunk in raw.as_bytes().chunks(7) {
        offsets.push(offset);
        script.push(fragment(offset, chunk));
        offset += chunk.len() as u16;
    }
    offsets.push(offset);
    script.push(fragment(offset, &[]));
    (script, offsets)
}

#[test]
fn capabilities_are_reassembled_once_and_then_reused() {
    let raw = "(vcp(86(01 02 05) E2(A0(20(00 02 04)))))";
    let (script, expected_offsets) = raw_script(raw);
    let mut ddc = DdcClient::new(FragmentTransport::new(script));
    let mut cache = CapabilityCache::default();

    let first = cache.get_or_read(&mut ddc).unwrap();
    assert_eq!(first.raw(), raw);
    assert_eq!(
        first.listed_values(Address::Standard(0x86)),
        Some(&[1, 2, 5][..])
    );
    assert_eq!(
        first.listed_values(Address::e2a0(0x20)),
        Some(&[0, 2, 4][..])
    );

    let second = cache.get_or_read(&mut ddc).unwrap();
    assert_eq!(second.raw(), raw);
    assert_eq!(ddc.into_inner().offsets, expected_offsets);
}

#[test]
fn failed_read_restarts_from_zero_and_does_not_cache_partial_data() {
    let raw = "(vcp(60(01 02)))";
    let (success, success_offsets) = raw_script(raw);
    let script = [
        fragment(0, b"(vcp("),
        Err(DdcError::UnsupportedCapabilities),
    ]
    .into_iter()
    .chain(success)
    .collect();
    let expected_offsets = [0, 5]
        .into_iter()
        .chain(success_offsets)
        .collect::<Vec<_>>();
    let mut ddc = DdcClient::new(FragmentTransport::new(script));
    let mut cache = CapabilityCache::default();

    assert!(matches!(
        cache.get_or_read(&mut ddc),
        Err(OsdError::Ddc(DdcError::UnsupportedCapabilities))
    ));
    assert_eq!(
        cache
            .get_or_read(&mut ddc)
            .unwrap()
            .listed_values(Address::Standard(0x60)),
        Some(&[1, 2][..])
    );
    cache.get_or_read(&mut ddc).unwrap();
    assert_eq!(ddc.into_inner().offsets, expected_offsets);
}

#[test]
fn parse_failure_is_not_cached() {
    let (invalid, invalid_offsets) = raw_script("(");
    let (valid, valid_offsets) = raw_script("(vcp(60(01 02)))");
    let script = invalid.into_iter().chain(valid).collect();
    let expected_offsets = invalid_offsets
        .into_iter()
        .chain(valid_offsets)
        .collect::<Vec<_>>();
    let mut ddc = DdcClient::new(FragmentTransport::new(script));
    let mut cache = CapabilityCache::default();

    assert!(matches!(
        cache.get_or_read(&mut ddc),
        Err(OsdError::InvalidCapabilities(_))
    ));
    assert_eq!(
        cache
            .get_or_read(&mut ddc)
            .unwrap()
            .listed_values(Address::Standard(0x60)),
        Some(&[1, 2][..])
    );
    cache.get_or_read(&mut ddc).unwrap();
    assert_eq!(ddc.into_inner().offsets, expected_offsets);
}
