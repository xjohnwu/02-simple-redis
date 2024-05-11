use std::{
    hash::{Hash, Hasher},
    ops::Deref,
};

use bytes::BytesMut;

use crate::{RespDecode, RespEncode, RespError};

use super::{extract_simple_frame_data, CRLF_LEN};

#[derive(Debug, Clone)]
pub struct ApproximateFloat(pub f64);

impl PartialEq for ApproximateFloat {
    fn eq(&self, other: &Self) -> bool {
        (self.0 - other.0).abs() < 1e-18 // Change precision as needed
    }
}

impl Eq for ApproximateFloat {}

impl Hash for ApproximateFloat {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Convert to a fixed precision before hashing to maintain consistency
        ((self.0 * 1e18).round() as i64).hash(state)
    }
}

impl PartialOrd for ApproximateFloat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Deref for ApproximateFloat {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// - double: ",[<+|->]<integral>[.<fractional>][<E|e>[sign]<exponent>]\r\n"
impl RespEncode for ApproximateFloat {
    fn encode(self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(32);
        let ret = if self.0.abs() > 1e+8 || self.abs() < 1e-8 {
            format!(",{:+e}\r\n", self.0)
        } else {
            let sign = if self.0 < 0.0 { "" } else { "+" };
            format!(",{}{}\r\n", sign, self.0)
        };

        buf.extend_from_slice(&ret.into_bytes());
        buf
    }
}

// - double: ",[<+|->]<integral>[.<fractional>][<E|e>[sign]<exponent>]\r\n"
impl RespDecode for ApproximateFloat {
    const PREFIX: &'static str = ",";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let end = extract_simple_frame_data(buf, Self::PREFIX)?;
        let data = buf.split_to(end + CRLF_LEN);
        let s = String::from_utf8_lossy(&data[Self::PREFIX.len()..end]);
        Ok(ApproximateFloat(s.parse()?))
    }

    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        let end = extract_simple_frame_data(buf, Self::PREFIX)?;
        Ok(end + CRLF_LEN)
    }
}

#[cfg(test)]
mod tests {
    use crate::RespFrame;

    use super::*;
    use anyhow::Result;

    #[test]
    fn test_double_encode() {
        let frame: RespFrame = ApproximateFloat(123.456).into();
        assert_eq!(frame.encode(), b",+123.456\r\n");

        let frame: RespFrame = ApproximateFloat(-123.456).into();
        assert_eq!(frame.encode(), b",-123.456\r\n");

        let frame: RespFrame = ApproximateFloat(1.23456e+8).into();
        assert_eq!(frame.encode(), b",+1.23456e8\r\n");

        let frame: RespFrame = ApproximateFloat(-1.23456e-9).into();
        assert_eq!(&frame.encode(), b",-1.23456e-9\r\n");
    }

    #[test]
    fn test_double_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b",123.45\r\n");

        let frame = ApproximateFloat::decode(&mut buf)?;
        assert_eq!(frame, ApproximateFloat(123.45));

        buf.extend_from_slice(b",+1.23456e-9\r\n");
        let frame = ApproximateFloat::decode(&mut buf)?;
        assert_eq!(frame, ApproximateFloat(1.23456e-9));

        Ok(())
    }
}
