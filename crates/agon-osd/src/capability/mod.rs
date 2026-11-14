mod cache;
mod parser;

pub(crate) use cache::CapabilityCache;
pub use parser::CapabilityParseError;

use crate::Address;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceCapabilities {
    raw: String,
    values: BTreeMap<u32, Vec<u8>>,
}

impl DeviceCapabilities {
    pub fn parse(raw: impl Into<String>) -> Result<Self, CapabilityParseError> {
        let raw = raw.into();
        let values = parser::parse(&raw)?;
        Ok(Self { raw, values })
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn listed_values(&self, address: Address) -> Option<&[u8]> {
        self.values.get(&address_key(address)).map(Vec::as_slice)
    }
}

fn address_key(address: Address) -> u32 {
    match address {
        Address::Standard(code) => u32::from(code),
        Address::Extended { prefix, code } => u32::from_be_bytes([0, prefix[0], prefix[1], code]),
    }
}
