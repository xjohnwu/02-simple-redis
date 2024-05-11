use std::ops::Deref;

use bytes::{Buf, BytesMut};

use crate::{RespDecode, RespEncode, RespError, RespFrame};

use super::{calc_total_length, parse_length, BUF_CAP, CRLF_LEN};

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Hash)]
pub struct RespArray(pub(crate) Vec<RespFrame>);

// - array: "*<number-of-elements>\r\n<element-1>...<element-n>"
impl RespEncode for RespArray {
    fn encode(self) -> Vec<u8> {
        if self.0.is_empty() {
            b"*-1\r\n".to_vec()
        } else {
            let mut buf = Vec::with_capacity(BUF_CAP);
            buf.extend_from_slice(&format!("*{}\r\n", self.0.len()).into_bytes());
            for frame in self.0 {
                buf.extend_from_slice(&frame.encode());
            }
            buf
        }
    }
}

// - array: "*<number-of-elements>\r\n<element-1>...<element-n>"
// - "*2\r\n$3\r\nget\r\n$5\r\nhello\r\n"
// or null array: "*-1\r\n"
impl RespDecode for RespArray {
    const PREFIX: &'static str = "*";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        if len < 0 {
            Ok(RespArray::null())
        } else {
            let len = len as usize;
            let total_len = calc_total_length(buf, end, len, Self::PREFIX)?;

            if buf.len() < total_len {
                return Err(RespError::NotComplete);
            }

            buf.advance(end + CRLF_LEN);

            let mut frames = Vec::with_capacity(len);
            for _ in 0..len {
                frames.push(RespFrame::decode(buf)?);
            }

            Ok(RespArray::new(frames))
        }
    }

    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        if len < 0 {
            Ok(4)
        } else {
            calc_total_length(buf, end, len as usize, Self::PREFIX)
        }
    }
}

impl RespArray {
    pub fn new(s: impl Into<Vec<RespFrame>>) -> Self {
        RespArray(s.into())
    }

    pub fn null() -> Self {
        RespArray(Vec::new())
    }
}

impl Deref for RespArray {
    type Target = Vec<RespFrame>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BulkString;
    use anyhow::Result;

    #[test]
    fn test_array_encode() {
        let frame: RespFrame = RespArray::new(vec![
            BulkString::new("set".to_string()).into(),
            BulkString::new("hello".to_string()).into(),
            BulkString::new("world".to_string()).into(),
        ])
        .into();
        assert_eq!(
            &frame.encode(),
            b"*3\r\n$3\r\nset\r\n$5\r\nhello\r\n$5\r\nworld\r\n"
        );
    }

    #[test]
    fn test_null_array() {
        assert_eq!(RespArray::null(), RespArray(Vec::new()));
    }

    #[test]
    fn test_null_array_encode() {
        let frame: RespFrame = RespArray::null().into();
        assert_eq!(frame.encode(), b"*-1\r\n");
    }

    #[test]
    fn test_null_array_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*-1\r\n");

        let frame = RespArray::decode(&mut buf)?;
        assert_eq!(frame, RespArray::null());

        Ok(())
    }

    #[test]
    fn test_array_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*2\r\n$3\r\nset\r\n$5\r\nhello\r\n");

        let frame = RespArray::decode(&mut buf)?;
        assert_eq!(frame, RespArray::new([b"set".into(), b"hello".into()]));

        buf.extend_from_slice(b"*2\r\n$3\r\nset\r\n");
        let ret = RespArray::decode(&mut buf);
        assert_eq!(ret.unwrap_err(), RespError::NotComplete);

        buf.extend_from_slice(b"$5\r\nhello\r\n");
        let frame = RespArray::decode(&mut buf)?;
        assert_eq!(frame, RespArray::new([b"set".into(), b"hello".into()]));

        Ok(())
    }
}
