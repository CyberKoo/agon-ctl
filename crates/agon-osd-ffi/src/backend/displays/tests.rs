use super::*;
use agon_osd::{Address, CapabilityFragment, DdcError, DdcTransport, VcpValue};
use std::cell::Cell;
use std::rc::Rc;

struct CapabilityTransport {
    raw: Vec<u8>,
    requests: Rc<Cell<usize>>,
}

impl CapabilityTransport {
    fn new(raw: &str) -> (Self, Rc<Cell<usize>>) {
        let requests = Rc::new(Cell::new(0));
        (
            Self {
                raw: raw.as_bytes().to_vec(),
                requests: Rc::clone(&requests),
            },
            requests,
        )
    }
}

impl DdcTransport for CapabilityTransport {
    fn read_vcp(&mut self, address: Address) -> Result<VcpValue, DdcError> {
        Err(DdcError::UnsupportedFeature { address })
    }

    fn write_vcp(&mut self, address: Address, _value: u16) -> Result<(), DdcError> {
        Err(DdcError::UnsupportedFeature { address })
    }

    fn read_capabilities(&mut self, offset: u16) -> Result<CapabilityFragment, DdcError> {
        self.requests.set(self.requests.get() + 1);
        let start = usize::from(offset);
        if start > self.raw.len() {
            return Err(DdcError::InvalidResponse("unexpected capability offset"));
        }
        let end = (start + 32).min(self.raw.len());
        Ok(CapabilityFragment {
            offset,
            data: self.raw[start..end].to_vec(),
        })
    }
}

fn info(id: &str) -> DisplayInfo {
    DisplayInfo::new(id.into(), "display".into(), 1, 2, 3, "location".into())
}

fn listed_values<T: DdcTransport>(displays: &mut Displays<T>, id: &str) -> Vec<u8> {
    displays
        .display(id)
        .unwrap()
        .read_capabilities()
        .unwrap()
        .listed_values(Address::Standard(0x60))
        .unwrap()
        .to_vec()
}

#[test]
fn lookup_requires_a_unique_exact_id() {
    let infos = [info("12"), info("123"), info("12")];
    assert_eq!(exact_index(&infos, "123").unwrap(), 1);
    assert_eq!(
        exact_index(&infos, "12").unwrap_err().status,
        crate::error::INTERNAL_ERROR
    );
    for id in ["", "1", " 123", "123 "] {
        assert_eq!(
            exact_index(&infos, id).unwrap_err().status,
            crate::error::DISPLAY_NOT_FOUND
        );
    }
    assert_eq!(
        exact_index(&[], "123").unwrap_err().status,
        crate::error::DISPLAY_NOT_FOUND
    );
}

#[test]
fn capability_caches_are_isolated_between_displays() {
    let (first, first_requests) = CapabilityTransport::new("(vcp(60(01)))");
    let (second, second_requests) = CapabilityTransport::new("(vcp(60(02)))");
    let mut displays = Displays::new();
    displays.replace(vec![(info("first"), first), (info("second"), second)]);

    assert_eq!(listed_values(&mut displays, "first"), [1]);
    assert_eq!(listed_values(&mut displays, "first"), [1]);
    assert_eq!(listed_values(&mut displays, "second"), [2]);
    assert_eq!(listed_values(&mut displays, "second"), [2]);

    assert_eq!(first_requests.get(), 2);
    assert_eq!(second_requests.get(), 2);
}

#[test]
fn replacement_rebuilds_connections_and_invalidates_capabilities() {
    let (old, old_requests) = CapabilityTransport::new("(vcp(60(01)))");
    let mut displays = Displays::new();
    displays.replace(vec![(info("display"), old)]);
    assert_eq!(listed_values(&mut displays, "display"), [1]);

    let (new, new_requests) = CapabilityTransport::new("(vcp(60(02)))");
    displays.replace(vec![(info("display"), new)]);

    assert_eq!(listed_values(&mut displays, "display"), [2]);
    assert_eq!(listed_values(&mut displays, "display"), [2]);
    assert_eq!(old_requests.get(), 2);
    assert_eq!(new_requests.get(), 2);
}
