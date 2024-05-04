/*
- 如何解析 Frame
    - simple string: "+OK\r\n"
    - error: "-Error message\r\n"
    - bulk error: "!<length>\r\n<error>\r\n"
    - integer: ":[<+|->]<value>\r\n"
    - bulk string: "$<length>\r\n<data>\r\n"
    - null bulk string: "$-1\r\n"
    - array: "*<number-of-elements>\r\n<element-1>...<element-n>"
        - "*2\r\n$3\r\nget\r\n$5\r\nhello\r\n"
    - null array: "*-1\r\n"
    - null: "_\r\n"
    - boolean: "#<t|f>\r\n"
    - double: ",[<+|->]<integral>[.<fractional>][<E|e>[sign]<exponent>]\r\n"
    - map: "%<number-of-entries>\r\n<key-1><value-1>...<key-n><value-n>"
    - set: "~<number-of-elements>\r\n<element-1>...<element-n>"
 */

use crate::{
    BulkString, RespArray, RespDecode, RespError, RespFrame, RespMap, RespNull, RespNullArray,
    RespSet, SimpleError, SimpleString,
};
use bytes::{Buf, BytesMut};

const CRLF_LEN: usize = 2;

impl RespDecode for RespFrame {
    const PREFIX: &'static str = "";
    fn decode(buf: &mut BytesMut) -> Result<RespFrame, RespError> {
        let mut iter = buf.iter().peekable();
        match iter.peek() {
            Some(b'+') => {
                let frame = SimpleString::decode(buf)?;
                Ok(frame.into())
            }
            Some(b'-') => {
                let frame = SimpleError::decode(buf)?;
                Ok(frame.into())
            }
            Some(b':') => {
                let frame = i64::decode(buf)?;
                Ok(frame.into())
            }
            Some(b'#') => {
                let frame = bool::decode(buf)?;
                Ok(frame.into())
            }
            Some(b',') => {
                let frame = f64::decode(buf)?;
                Ok(frame.into())
            }
            Some(b'$') => {
                // try null bulk string first
                match BulkString::decode(buf) {
                    Ok(frame) => Ok(frame.into()),
                    Err(RespError::NotComplete) => Err(RespError::NotComplete),
                    Err(_) => {
                        let frame = BulkString::decode(buf)?;
                        Ok(frame.into())
                    }
                }
            }
            Some(b'*') => {
                // try null array first
                match RespNullArray::decode(buf) {
                    Ok(frame) => Ok(frame.into()),
                    Err(RespError::NotComplete) => Err(RespError::NotComplete),
                    Err(_) => {
                        let frame = RespArray::decode(buf)?;
                        Ok(frame.into())
                    }
                }
            }
            Some(b'%') => {
                let frame = RespMap::decode(buf)?;
                Ok(frame.into())
            }
            Some(b'~') => {
                let frame = RespSet::decode(buf)?;
                Ok(frame.into())
            }
            _ => Err(RespError::InvalidFrameType(format!(
                "Invalid frame type: {:?}",
                buf
            ))),
        }
    }
    fn expected_length(buf: &[u8]) -> Result<usize, RespError> {
        let mut iter = buf.iter().peekable();
        match iter.peek() {
            Some(b'+') => SimpleString::expected_length(buf),
            Some(b'-') => SimpleError::expected_length(buf),
            Some(b':') => i64::expected_length(buf),
            Some(b'#') => bool::expected_length(buf),
            Some(b',') => f64::expected_length(buf),
            Some(b'$') => BulkString::expected_length(buf),
            Some(b'*') => RespArray::expected_length(buf),
            Some(b'%') => RespMap::expected_length(buf),
            Some(b'~') => RespSet::expected_length(buf),
            _ => Err(RespError::NotComplete),
        }
    }
}

fn extract_fixed_data(
    buf: &mut BytesMut,
    expect: &str,
    expect_type: &str,
) -> Result<(), RespError> {
    if buf.len() < expect.len() {
        return Err(RespError::NotComplete);
    }

    if !buf.starts_with(expect.as_bytes()) {
        return Err(RespError::InvalidFrameType(format!(
            "expect: {}, got: {:?}",
            expect_type, buf
        )));
    }

    buf.advance(expect.len());
    Ok(())
}

fn extract_simple_frame_data(buf: &[u8], prefix: &str) -> Result<usize, RespError> {
    if buf.len() < 3 {
        return Err(RespError::NotComplete);
    }
    if !buf.starts_with(prefix.as_bytes()) {
        return Err(RespError::InvalidFrameType(format!(
            "expect: SimpleString(+), got: {:?}",
            buf
        )));
    }
    let end = find_crlf(buf, 1).ok_or(RespError::NotComplete)?;

    Ok(end)
}

// find nth CRLF in the buffer
fn find_crlf(buf: &[u8], nth: usize) -> Option<usize> {
    let mut count = 0;
    for i in 1..buf.len() - 1 {
        if buf[i] == b'\r' && buf[i + 1] == b'\n' {
            count += 1;
            if count == nth {
                return Some(i);
            }
        }
    }

    None
}

fn parse_length(buf: &[u8], prefix: &str) -> Result<(usize, usize), RespError> {
    let end = extract_simple_frame_data(buf, prefix)?;
    let s = String::from_utf8_lossy(&buf[prefix.len()..end]);
    Ok((end, s.parse()?))
}

fn calc_total_length(buf: &[u8], end: usize, len: usize, prefix: &str) -> Result<usize, RespError> {
    let mut total = end + CRLF_LEN;
    let mut data = &buf[total..];
    match prefix {
        "*" | "~" => {
            // find nth CRLF in the buffer. For array and set, we need to find 1 CRLF for each element
            for _ in 0..len {
                let len = RespFrame::expected_length(data)?;
                data = &data[len..];
                total += len;
            }
            Ok(total)
        }
        "%" => {
            // find nth CRLF in the buffer. For map, we need to find 2 CRLF for each key-value pair
            let len = SimpleString::expected_length(data)?;
            data = &data[len..];
            total += len;

            let len = RespFrame::expected_length(data)?;
            // data = &data[len..];
            total += len;

            Ok(total)
        }
        _ => Ok(len + CRLF_LEN),
    }
}

impl RespDecode for SimpleString {
    const PREFIX: &'static str = "+";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let end = extract_simple_frame_data(buf, Self::PREFIX)?;

        // split the buffer
        let data = buf.split_to(end + CRLF_LEN);
        let s = String::from_utf8_lossy(&data[1..end]);
        Ok(SimpleString::new(s))
    }
    fn expected_length(buf: &[u8]) -> Result<usize, RespError> {
        let end = extract_simple_frame_data(buf, Self::PREFIX)?;
        Ok(end + CRLF_LEN)
    }
}

impl RespDecode for SimpleError {
    const PREFIX: &'static str = "-";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let end = extract_simple_frame_data(buf, Self::PREFIX)?;

        // split the buffer
        let data = buf.split_to(end + CRLF_LEN);
        let s = String::from_utf8_lossy(&data[1..end]);
        Ok(SimpleError::new(s))
    }

    fn expected_length(buf: &[u8]) -> Result<usize, RespError> {
        let end = extract_simple_frame_data(buf, Self::PREFIX)?;
        Ok(end + CRLF_LEN)
    }
}

impl RespDecode for BulkString {
    const PREFIX: &'static str = "$";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        let remained = &buf[end + CRLF_LEN..];
        if remained.len() < len + CRLF_LEN {
            return Err(RespError::NotComplete);
        }

        buf.advance(end + CRLF_LEN);

        let data = buf.split_to(len + CRLF_LEN);
        Ok(BulkString::new(data[..len].to_vec()))
    }

    fn expected_length(buf: &[u8]) -> Result<usize, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        Ok(end + CRLF_LEN + len + CRLF_LEN)
    }
}

impl RespDecode for i64 {
    const PREFIX: &'static str = ":";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let end = extract_simple_frame_data(buf, ":")?;

        // split the buffer
        let data = buf.split_to(end + CRLF_LEN);
        let s = String::from_utf8_lossy(&data[1..end]);
        Ok(s.parse()?)
    }

    fn expected_length(buf: &[u8]) -> Result<usize, RespError> {
        let end = extract_simple_frame_data(buf, ":")?;
        Ok(end + CRLF_LEN)
    }
}

impl RespDecode for bool {
    const PREFIX: &'static str = "#";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        match extract_fixed_data(buf, "#t\r\n", "Bool") {
            Ok(_) => Ok(true),
            Err(RespError::NotComplete) => Err(RespError::NotComplete),
            Err(_) => match extract_fixed_data(buf, "#f\r\n", "Bool") {
                Ok(_) => Ok(false),
                Err(e) => Err(e),
            },
        }
    }

    fn expected_length(buf: &[u8]) -> Result<usize, RespError> {
        if buf.starts_with(b"#t\r\n") || buf.starts_with(b"#f\r\n") {
            Ok(4)
        } else {
            Err(RespError::InvalidFrame("Invalid frame".to_string()))
        }
    }
}

impl RespDecode for f64 {
    const PREFIX: &'static str = ",";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let prefix = ",";
        let end = extract_simple_frame_data(buf, prefix)?;

        // split the buffer
        let data = buf.split_to(end + CRLF_LEN);
        let s = String::from_utf8_lossy(&data[prefix.len()..end]);
        Ok(s.parse()?)
    }

    fn expected_length(buf: &[u8]) -> Result<usize, RespError> {
        let prefix = ",";
        let end = extract_simple_frame_data(buf, prefix)?;
        Ok(end + CRLF_LEN)
    }
}

impl RespDecode for RespNull {
    const PREFIX: &'static str = "_";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        extract_fixed_data(buf, "_\r\n", "Null").map(|_| RespNull)
    }

    fn expected_length(_buf: &[u8]) -> Result<usize, RespError> {
        Ok(3)
    }
}

// - array: "*<number-of-elements>\r\n<element-1>...<element-n>"
// - "*2\r\n$3\r\nget\r\n$5\r\nhello\r\n"
impl RespDecode for RespArray {
    const PREFIX: &'static str = "*";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;

        let total_len = calc_total_length(buf, end, len, Self::PREFIX)?;

        if buf.len() < total_len {
            return Err(RespError::NotComplete);
        }

        buf.advance(end + CRLF_LEN);

        let mut frames = Vec::with_capacity(len);
        for _ in 0..len {
            let frame = RespFrame::decode(buf)?;
            frames.push(frame);
        }
        Ok(RespArray::new(frames))
    }

    fn expected_length(buf: &[u8]) -> Result<usize, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        calc_total_length(buf, end, len, Self::PREFIX)
    }
}

impl RespDecode for RespNullArray {
    const PREFIX: &'static str = "*";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        extract_fixed_data(buf, "*-1\r\n", "NullArray")?;
        Ok(RespNullArray)
    }
    fn expected_length(_buf: &[u8]) -> Result<usize, RespError> {
        Ok(5)
    }
}

impl RespDecode for RespMap {
    const PREFIX: &'static str = "%";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        let total_len = calc_total_length(buf, end, len, Self::PREFIX)?;

        if buf.len() < total_len {
            return Err(RespError::NotComplete);
        }

        buf.advance(end + CRLF_LEN);

        let mut map = RespMap::new();

        for _ in 0..len {
            let key = SimpleString::decode(buf)?;
            let value = RespFrame::decode(buf)?;
            map.insert(key.0, value);
        }

        Ok(map)
    }
    fn expected_length(buf: &[u8]) -> Result<usize, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        calc_total_length(buf, end, len, Self::PREFIX)
    }
}

impl RespDecode for RespSet {
    const PREFIX: &'static str = "~";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;

        let total_len = calc_total_length(buf, end, len, Self::PREFIX)?;

        if buf.len() < total_len {
            return Err(RespError::NotComplete);
        }

        buf.advance(end + CRLF_LEN);

        let mut set = Vec::new();

        for _ in 0..len {
            let frame = RespFrame::decode(buf)?;
            set.push(frame);
        }

        Ok(RespSet::new(set))
    }
    fn expected_length(buf: &[u8]) -> Result<usize, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        calc_total_length(buf, end, len, Self::PREFIX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use bytes::BufMut;

    #[test]
    fn test_simple_string_decode() -> Result<()> {
        let mut buf = BytesMut::from("+OK\r\n");
        let frame = SimpleString::decode(&mut buf)?;
        assert_eq!(frame, SimpleString::new("OK".to_string()));

        buf.extend_from_slice(b"+hello\r");
        let err = SimpleString::decode(&mut buf);
        assert_eq!(err, Err(RespError::NotComplete));

        buf.put_u8(b'\n');
        let frame = SimpleString::decode(&mut buf)?;
        assert_eq!(frame, SimpleString::new("hello".to_string()));
        Ok(())
    }

    #[test]
    fn test_simple_error_decode() -> Result<()> {
        let mut buf = BytesMut::from("-Error message\r\n");
        let frame = SimpleError::decode(&mut buf)?;
        assert_eq!(frame, SimpleError::new("Error message".to_string()));

        buf.extend_from_slice(b"-hello\r");
        let err = SimpleError::decode(&mut buf);
        assert_eq!(err, Err(RespError::NotComplete));

        buf.put_u8(b'\n');
        let frame = SimpleError::decode(&mut buf)?;
        assert_eq!(frame, SimpleError::new("hello".to_string()));
        Ok(())
    }

    #[test]
    fn test_integer_decode() -> Result<()> {
        let mut buf = BytesMut::from(":123\r\n");
        let frame = i64::decode(&mut buf)?;
        assert_eq!(frame, 123);

        buf.extend_from_slice(b":-123\r");
        let err = i64::decode(&mut buf);
        assert_eq!(err, Err(RespError::NotComplete));

        buf.put_u8(b'\n');
        let frame = i64::decode(&mut buf)?;
        assert_eq!(frame, -123);
        Ok(())
    }

    #[test]
    fn test_bool_decode() -> Result<()> {
        let mut buf = BytesMut::from("#t\r\n");
        let frame = bool::decode(&mut buf)?;
        assert!(frame);

        buf.extend_from_slice(b"#f\r\n");
        let frame = bool::decode(&mut buf)?;
        assert!(!frame);

        buf.extend_from_slice(b"#f\r");
        let err = bool::decode(&mut buf);
        assert_eq!(err, Err(RespError::NotComplete));

        buf.put_u8(b'\n');
        let frame = bool::decode(&mut buf)?;
        assert!(!frame);
        Ok(())
    }

    #[test]
    fn test_double_decode() -> Result<()> {
        let mut buf = BytesMut::from(",123.456\r\n");
        let frame = f64::decode(&mut buf)?;
        assert_eq!(frame, 123.456);

        buf.extend_from_slice(b",-123.456\r");
        let err = f64::decode(&mut buf);
        assert_eq!(err, Err(RespError::NotComplete));

        buf.put_u8(b'\n');
        let frame = f64::decode(&mut buf)?;
        assert_eq!(frame, -123.456);
        Ok(())
    }

    #[test]
    fn test_array_encode() -> Result<()> {
        let mut buf = BytesMut::from("*2\r\n$3\r\nget\r\n$5\r\nhello\r\n");
        let frame = RespArray::decode(&mut buf)?;
        assert_eq!(
            frame,
            RespArray::new(vec![
                BulkString::new("get".to_string()).into(),
                BulkString::new("hello".to_string()).into()
            ])
        );
        Ok(())
    }
}
