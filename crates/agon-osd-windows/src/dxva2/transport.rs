use super::Dxva2Display;
use agon_osd::{PacketTransport, TransportError};
use windows_sys::Win32::Devices::Display::{GetVCPFeatureAndVCPFeatureReply, SetVCPFeature};
use windows_sys::Win32::Foundation::ERROR_GRAPHICS_MONITOR_NO_LONGER_EXISTS;

fn is_missing_monitor_error(error: &TransportError) -> bool {
    matches!(
        error,
        TransportError::Io { code, .. }
            if *code == ERROR_GRAPHICS_MONITOR_NO_LONGER_EXISTS
    )
}

pub(crate) fn is_refreshed_monitor_error(error: &agon_osd::DdcError) -> bool {
    matches!(
        error,
        agon_osd::DdcError::Transport(TransportError::Io {
            operation: "GetVCPFeatureAndVCPFeatureReply",
            code,
        }) if *code == ERROR_GRAPHICS_MONITOR_NO_LONGER_EXISTS
    )
}

impl PacketTransport for Dxva2Display {
    fn write_packet(&mut self, packet: &[u8]) -> Result<(), TransportError> {
        let body = packet_body(packet)?;
        match body {
            [0x01, vcp] => {
                self.pending_read = Some(*vcp);
                Ok(())
            }
            [0x03, vcp, high, low] => {
                self.pending_read = None;
                let value = u16::from_be_bytes([*high, *low]);
                // SAFETY: the physical monitor handle is valid for the synchronous dxva2 call.
                let result =
                    unsafe { SetVCPFeature(self.monitor.hPhysicalMonitor, *vcp, u32::from(value)) };
                if result == 0 {
                    Err(last_error("SetVCPFeature"))
                } else {
                    Ok(())
                }
            }
            _ => Err(TransportError::UnsupportedFeature(
                "extended DDC addresses require a raw I2C channel",
            )),
        }
    }

    fn read_packet(&mut self, response: &mut [u8]) -> Result<(), TransportError> {
        if response.len() != 11 {
            return Err(TransportError::InvalidRequest(
                "dxva2 response buffer must be 11 bytes",
            ));
        }
        let vcp = self
            .pending_read
            .take()
            .ok_or(TransportError::InvalidRequest(
                "dxva2 read called without a pending VCP query",
            ))?;
        let mut code_type = 0;
        let mut current = 0;
        let mut maximum = 0;
        // SAFETY: the monitor handle and all three output pointers are valid.
        let result = unsafe {
            GetVCPFeatureAndVCPFeatureReply(
                self.monitor.hPhysicalMonitor,
                vcp,
                &mut code_type,
                &mut current,
                &mut maximum,
            )
        };
        if result == 0 {
            let error = last_error("GetVCPFeatureAndVCPFeatureReply");
            if is_missing_monitor_error(&error) {
                self.refresh_monitor()?;
            }
            return Err(error);
        }
        let [max_high, max_low] = (maximum as u16).to_be_bytes();
        let [cur_high, cur_low] = (current as u16).to_be_bytes();
        response.copy_from_slice(&[
            0x6E,
            0x88,
            0x02,
            0,
            vcp,
            code_type as u8,
            max_high,
            max_low,
            cur_high,
            cur_low,
            0,
        ]);
        response[10] = response[..10]
            .iter()
            .fold(0x50_u8, |checksum, byte| checksum ^ byte);
        Ok(())
    }
}

fn packet_body(packet: &[u8]) -> Result<&[u8], TransportError> {
    if packet.len() < 4 || packet[0] != 0x51 || packet[1] & 0x80 == 0 {
        return Err(TransportError::InvalidRequest(
            "invalid DDC packet for dxva2",
        ));
    }
    let length = usize::from(packet[1] & 0x7F);
    if length + 3 != packet.len() {
        return Err(TransportError::InvalidRequest("DDC packet length mismatch"));
    }
    Ok(&packet[2..2 + length])
}

fn last_error(operation: &'static str) -> TransportError {
    TransportError::Io {
        operation,
        code: std::io::Error::last_os_error().raw_os_error().unwrap_or(-1),
    }
}
