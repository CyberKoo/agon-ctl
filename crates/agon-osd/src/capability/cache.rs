use super::DeviceCapabilities;
use crate::ddc::DdcClient;
use crate::{DdcError, DdcTransport, OsdError};

#[derive(Default)]
pub(crate) struct CapabilityCache {
    snapshot: Option<DeviceCapabilities>,
}

impl CapabilityCache {
    pub(crate) fn get_or_read<T: DdcTransport>(
        &mut self,
        ddc: &mut DdcClient<T>,
    ) -> Result<&DeviceCapabilities, OsdError> {
        if self.snapshot.is_none() {
            let snapshot = read_from_device(ddc)?;
            self.snapshot = Some(snapshot);
        }
        Ok(self
            .snapshot
            .as_ref()
            .expect("capability snapshot initialized above"))
    }
}

fn read_from_device<T: DdcTransport>(
    ddc: &mut DdcClient<T>,
) -> Result<DeviceCapabilities, OsdError> {
    let mut bytes = Vec::new();
    let mut offset = 0_u16;
    loop {
        let fragment = ddc.read_capabilities(offset)?;
        if fragment.offset != offset {
            return Err(DdcError::InvalidResponse("capabilities fragment offset mismatch").into());
        }
        if fragment.data.len() > 32 {
            return Err(DdcError::InvalidResponse("capabilities fragment is too long").into());
        }
        let fragment_len = fragment.data.len();
        let end = fragment
            .data
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(fragment_len);
        bytes.extend_from_slice(&fragment.data[..end]);
        if end != fragment_len || fragment_len == 0 {
            break;
        }
        offset = offset
            .checked_add(fragment_len as u16)
            .ok_or(DdcError::InvalidResponse(
                "capabilities string exceeds the offset range",
            ))?;
    }
    let raw = String::from_utf8(bytes)
        .map_err(|_| DdcError::InvalidResponse("capabilities string is not valid UTF-8"))?;
    DeviceCapabilities::parse(raw).map_err(OsdError::InvalidCapabilities)
}

#[cfg(test)]
mod tests;
