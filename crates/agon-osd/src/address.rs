use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Address {
    Standard(u8),
    Extended { prefix: [u8; 2], code: u8 },
}

impl Address {
    pub const fn e2a0(code: u8) -> Self {
        Self::Extended {
            prefix: [0xE2, 0xA0],
            code,
        }
    }

    pub const fn f8e1(code: u8) -> Self {
        Self::Extended {
            prefix: [0xF8, 0xE1],
            code,
        }
    }

    pub(crate) fn encode_address(self, body: &mut [u8; 6]) -> usize {
        match self {
            Self::Standard(code) => {
                body[1] = code;
                2
            }
            Self::Extended { prefix, code } => {
                body[1] = prefix[0];
                body[2] = prefix[1];
                body[3] = code;
                4
            }
        }
    }

    pub(crate) fn encode_read(self, body: &mut [u8; 6]) -> usize {
        body[0] = 0x01;
        self.encode_address(body)
    }

    pub(crate) fn encode_write(self, value: u16, body: &mut [u8; 6]) -> usize {
        body[0] = 0x03;
        let len = self.encode_address(body);
        let [high, low] = value.to_be_bytes();
        body[len] = high;
        body[len + 1] = low;
        len + 2
    }
}

impl fmt::Display for Address {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Standard(code) => write!(formatter, "{code:02x}"),
            Self::Extended { prefix, code } => {
                write!(formatter, "{:02x}:{:02x}:{code:02x}", prefix[0], prefix[1])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Standard address read: body = [0x01, VCP code], length 2.
    #[test]
    fn standard_read_body_is_two_bytes() {
        let mut body = [0; 6];
        assert_eq!(Address::Standard(0x60).encode_read(&mut body), 2);
        assert_eq!(&body[..2], &[0x01, 0x60]);
    }

    /// Standard address write: body = [0x03, VCP code, high byte, low byte], length 4.
    #[test]
    fn standard_write_body_is_four_bytes() {
        let mut body = [0; 6];
        assert_eq!(Address::Standard(0x10).encode_write(0x0123, &mut body), 4);
        assert_eq!(&body[..4], &[0x03, 0x10, 0x01, 0x23]);
    }

    /// Extended address read: body = [0x01, prefix 1, prefix 2, subcode], length 4.
    #[test]
    fn extended_read_body_is_four_bytes() {
        let mut body = [0; 6];
        assert_eq!(Address::e2a0(0x20).encode_read(&mut body), 4);
        assert_eq!(&body[..4], &[0x01, 0xE2, 0xA0, 0x20]);
    }

    /// Extended address write: body = [0x03, prefix 1, prefix 2, subcode, high byte, low byte], length 6.
    #[test]
    fn extended_write_body_is_six_bytes() {
        let mut body = [0; 6];
        assert_eq!(Address::f8e1(0x12).encode_write(0x0002, &mut body), 6);
        assert_eq!(&body[..6], &[0x03, 0xF8, 0xE1, 0x12, 0x00, 0x02]);
    }
}
