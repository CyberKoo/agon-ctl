use crate::capability::CapabilityCache;
use crate::ddc::DdcClient;
use crate::{DdcError, DdcTransport, DeviceCapabilities, Feature, OsdError, ValueDomain, VcpValue};

pub struct Osd<T> {
    ddc: DdcClient<T>,
    capabilities: CapabilityCache,
}

impl<T: DdcTransport> Osd<T> {
    pub fn new(transport: T) -> Self {
        Self {
            ddc: DdcClient::new(transport),
            capabilities: CapabilityCache::default(),
        }
    }

    pub fn into_inner(self) -> T {
        self.ddc.into_inner()
    }

    pub fn read(&mut self, feature: Feature) -> Result<VcpValue, OsdError> {
        self.ddc
            .read(feature.definition().address)
            .map_err(Into::into)
    }

    pub fn set(&mut self, feature: Feature, value: u16) -> Result<VcpValue, OsdError> {
        let before = self.ddc.read(feature.definition().address)?;
        self.set_after_read(feature, before, value)
    }

    pub fn set_from_str(&mut self, feature: Feature, input: &str) -> Result<VcpValue, OsdError> {
        let definition = feature.definition();
        let before = self.ddc.read(definition.address)?;
        let value = definition.parse_value_for_maximum(input, before.maximum)?;
        self.set_after_read(feature, before, value)
    }

    fn set_after_read(
        &mut self,
        feature: Feature,
        before: VcpValue,
        value: u16,
    ) -> Result<VcpValue, OsdError> {
        let definition = feature.definition();
        let address = definition.address;
        // Only purely numeric features use the display-reported maximum as an upper bound.
        // Named inputs are resolved through the named-value table, while direct numeric input
        // remains available for firmware variants. Applying the maximum to named features would
        // reject valid values: color-gamut values are non-contiguous (0-14, then 128-138), so an
        // understated maximum for E2A0 0x20 could reject the CMR range before sending a packet.
        // Named values from the supported list are sent unchanged, without maximum clamping.
        if definition.domain == ValueDomain::Number && value > before.maximum {
            return Err(OsdError::ValueAboveMaximum {
                value,
                maximum: before.maximum,
            });
        }
        self.ddc
            .write(address, value)
            .map_err(|error| match error {
                DdcError::Transport(
                    crate::TransportError::Io { .. } | crate::TransportError::UnexpectedEof(_),
                ) => OsdError::WriteUncertain(error),
                _ => OsdError::Ddc(error),
            })?;
        self.ddc.read(address).map_err(OsdError::ReadbackFailed)
    }

    /// Returns this connection's capability snapshot, reading it on first use.
    /// Read and parse errors are not cached, so a later call starts over.
    pub fn read_capabilities(&mut self) -> Result<&DeviceCapabilities, OsdError> {
        self.capabilities.get_or_read(&mut self.ddc)
    }

    pub fn probe(&mut self, feature: Feature) -> Result<Option<VcpValue>, OsdError> {
        match self.ddc.read(feature.definition().address) {
            Ok(value) => Ok(Some(value)),
            Err(DdcError::Unsupported { .. } | DdcError::UnsupportedFeature { .. }) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::VCP_RESPONSE_SIZE;
    use crate::{Address, TransportError};

    /// Records every write frame and returns scripted write results in order.
    struct MockTransport {
        response: [u8; VCP_RESPONSE_SIZE],
        write_plan: Vec<Result<(), TransportError>>,
        read_plan: Vec<Result<(), TransportError>>,
        writes: Vec<Vec<u8>>,
    }

    impl MockTransport {
        /// Creates a display that returns valid responses with the given maximum and current values.
        fn reporting(maximum: u16, current: u16) -> Self {
            let mut response = [0u8; VCP_RESPONSE_SIZE];
            response[0] = 0x6E;
            response[1] = 0x88;
            response[2] = 0x02;
            response[6] = (maximum >> 8) as u8;
            response[7] = maximum as u8;
            response[8] = (current >> 8) as u8;
            response[9] = current as u8;
            response[VCP_RESPONSE_SIZE - 1] = response[..VCP_RESPONSE_SIZE - 1]
                .iter()
                .fold(0x50, |checksum, byte| checksum ^ byte);
            Self {
                response,
                write_plan: Vec::new(),
                read_plan: Vec::new(),
                writes: Vec::new(),
            }
        }

        fn with_write_plan(mut self, plan: Vec<Result<(), TransportError>>) -> Self {
            self.write_plan = plan;
            self
        }
    }

    impl crate::PacketTransport for MockTransport {
        fn write_packet(&mut self, packet: &[u8]) -> Result<(), TransportError> {
            self.writes.push(packet.to_vec());
            if self.write_plan.is_empty() {
                return Ok(());
            }
            self.write_plan.remove(0)
        }

        fn read_packet(&mut self, response: &mut [u8]) -> Result<(), TransportError> {
            if !self.read_plan.is_empty() {
                self.read_plan.remove(0)?;
            }
            response.copy_from_slice(&self.response);
            if let Some(packet) = self.writes.last() {
                if packet.len() == 5 {
                    response[4] = packet[3];
                }
            }
            response[10] = response[..10]
                .iter()
                .fold(0x50, |checksum, byte| checksum ^ byte);
            Ok(())
        }
    }

    /// Numeric domain: values above the display-reported maximum are rejected before writing.
    #[test]
    fn numeric_domain_rejects_value_above_reported_maximum() {
        let mut osd = Osd::new(MockTransport::reporting(100, 50));
        let error = osd.set(Feature::Brightness, 101).unwrap_err();
        assert_eq!(
            error,
            OsdError::ValueAboveMaximum {
                value: 101,
                maximum: 100
            }
        );
        assert_eq!(
            osd.into_inner().writes.len(),
            1,
            "只应发出读请求,不该发出写值报文"
        );
    }

    /// Named domain: the display-reported maximum does not constrain values.
    /// Color-gamut values are non-contiguous (0-14, then 128-138), so an understated
    /// maximum must not reject the CMR range locally.
    #[test]
    fn named_domain_is_not_clamped_by_reported_maximum() {
        let mut osd = Osd::new(MockTransport::reporting(14, 0));
        let value = osd
            .set(Feature::ColorGamut, 128)
            .expect("命名值域不应被显示器回报的 maximum 挡下");
        assert_eq!(value.maximum, 14);
        let transport = osd.into_inner();
        assert_eq!(transport.writes.len(), 3, "读请求 + 写值 + 回读");
        assert_eq!(
            &transport.writes[1][..8],
            &[0x51, 0x86, 0x03, 0xE2, 0xA0, 0x20, 0x00, 0x80],
            "写值报文应原样下发 128"
        );
    }

    #[test]
    fn named_input_is_resolved_with_the_preread_maximum() {
        let mut osd = Osd::new(MockTransport::reporting(3, 0));
        osd.set_from_str(Feature::HdrMode, "HDR_Picture")
            .expect("maximum=3 应选择 HDR_3 表");
        let transport = osd.into_inner();
        assert_eq!(transport.writes.len(), 3, "预读 + 写值 + 回读");
        assert_eq!(
            &transport.writes[1][..8],
            &[0x51, 0x86, 0x03, 0xF8, 0xE1, 0x52, 0x00, 0x01]
        );
    }

    #[test]
    fn invalid_dynamic_name_does_not_write() {
        let mut osd = Osd::new(MockTransport::reporting(11, 0));
        let error = osd
            .set_from_str(Feature::LightFxMode, "static")
            .unwrap_err();
        assert!(matches!(error, OsdError::InvalidValue(_)));
        assert_eq!(osd.into_inner().writes.len(), 1, "只应发出预读请求");
    }

    /// An unsupported channel is reported as an absent feature without retries.
    #[test]
    fn unsupported_channel_is_reported_as_absent_without_retrying() {
        let transport = MockTransport::reporting(100, 50).with_write_plan(vec![Err(
            TransportError::UnsupportedFeature("extended DDC addresses require a raw I2C channel"),
        )]);
        let mut osd = Osd::new(transport);
        assert_eq!(osd.probe(Feature::Gamma), Ok(None));
        assert_eq!(osd.into_inner().writes.len(), 1, "「通道不支持」不应重试");
    }

    #[test]
    fn unsupported_capabilities_are_not_retried_or_misreported_as_a_vcp_address() {
        let transport = MockTransport::reporting(100, 50).with_write_plan(vec![Err(
            TransportError::UnsupportedFeature("capabilities require a raw I2C channel"),
        )]);
        let mut osd = Osd::new(transport);
        assert_eq!(
            osd.read_capabilities(),
            Err(OsdError::Ddc(DdcError::UnsupportedCapabilities))
        );
        assert_eq!(osd.into_inner().writes.len(), 1);
    }

    /// Real transport failures are retried up to three attempts.
    #[test]
    fn transport_failures_are_retried() {
        let failure = || {
            Err(TransportError::Io {
                operation: "i2c-dev write",
                code: 5,
            })
        };
        let transport = MockTransport::reporting(100, 50).with_write_plan(vec![
            failure(),
            failure(),
            failure(),
        ]);
        let mut osd = Osd::new(transport);
        assert!(osd.probe(Feature::Brightness).is_err());
        assert_eq!(osd.into_inner().writes.len(), 3, "传输故障应重试到 3 次");
    }

    #[test]
    fn uncertain_write_is_not_repeated() {
        let transport = MockTransport::reporting(100, 50).with_write_plan(vec![
            Ok(()),
            Err(TransportError::Io {
                operation: "write",
                code: 5,
            }),
        ]);
        let mut osd = Osd::new(transport);
        assert!(matches!(
            osd.set(Feature::Brightness, 40),
            Err(OsdError::WriteUncertain(_))
        ));
        assert_eq!(osd.into_inner().writes.len(), 2);
    }

    #[test]
    fn failed_readback_does_not_repeat_the_successful_write() {
        let mut transport = MockTransport::reporting(100, 50);
        let failure = || {
            Err(TransportError::Io {
                operation: "read",
                code: 5,
            })
        };
        transport.read_plan = vec![Ok(()), failure(), failure(), failure()];
        let mut osd = Osd::new(transport);
        assert!(matches!(
            osd.set(Feature::Brightness, 40),
            Err(OsdError::ReadbackFailed(_))
        ));
        let transport = osd.into_inner();
        assert_eq!(
            transport
                .writes
                .iter()
                .filter(|packet| packet[2] == 0x03)
                .count(),
            1
        );
        assert_eq!(
            transport.writes.len(),
            5,
            "pre-read, write, three readback queries"
        );
    }

    /// Unsupported channels are not retried on the write path either.
    #[test]
    fn unsupported_channel_does_not_retry_on_write() {
        let transport = MockTransport::reporting(100, 50).with_write_plan(vec![
            Ok(()),
            Err(TransportError::UnsupportedFeature(
                "channel does not carry this feature",
            )),
        ]);
        let mut osd = Osd::new(transport);
        let error = osd.set(Feature::Brightness, 50).unwrap_err();
        assert_eq!(
            error,
            OsdError::Ddc(DdcError::UnsupportedFeature {
                address: Address::Standard(0x10)
            })
        );
        assert_eq!(
            osd.into_inner().writes.len(),
            2,
            "读请求 + 写值,写值失败后不应重试"
        );
    }
}
