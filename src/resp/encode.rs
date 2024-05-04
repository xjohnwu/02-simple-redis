use crate::{
    BulkString, RespArray, RespEncode, RespMap, RespNull, RespNullArray, RespNullBulkString,
    RespSet, SimpleError, SimpleString,
};

const BUF_CAP: usize = 4096;

// - simple string: "+OK\r\n"
impl RespEncode for SimpleString {
    fn encode(self) -> Vec<u8> {
        format!("+{}\r\n", self.0).into_bytes()
    }
}

// - simple error: "-Error message\r\n"
impl RespEncode for SimpleError {
    fn encode(self) -> Vec<u8> {
        format!("-{}\r\n", self.0).into_bytes()
    }
}

// - integer: ":[<+|->]<value>\r\n"
impl RespEncode for i64 {
    fn encode(self) -> Vec<u8> {
        let sign = if self < 0 { "" } else { "+" };
        format!(":{}{}\r\n", sign, self).into_bytes()
    }
}

// - boolean: "#<t|f>\r\n"
impl RespEncode for bool {
    fn encode(self) -> Vec<u8> {
        format!("#{}\r\n", if self { "t" } else { "f" }).into_bytes()
    }
}

// - double: ",[<+|->]<integral>[.<fractional>][<E|e>[sign]<exponent>]\r\n"
impl RespEncode for f64 {
    fn encode(self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(32);
        buf.extend_from_slice(&format!(",{:+e}\r\n", self).into_bytes());
        buf
    }
}

// - bulk string: "$<length>\r\n\<data>\r\n"
impl RespEncode for BulkString {
    fn encode(self) -> Vec<u8> {
        let length = self.len();
        let mut buf = Vec::with_capacity(length + 5);
        buf.extend_from_slice(&format!("${}\r\n", length).into_bytes());
        buf.extend_from_slice(&self);
        buf.extend_from_slice(b"\r\n");
        buf
    }
}

// - null bulk string: "$-1\r\n"
impl RespEncode for RespNullBulkString {
    fn encode(self) -> Vec<u8> {
        b"$-1\r\n".to_vec()
    }
}

// - null: "_\r\n"
impl RespEncode for RespNull {
    fn encode(self) -> Vec<u8> {
        b"_\r\n".to_vec()
    }
}

// - null array: "*-1\r\n"
impl RespEncode for RespNullArray {
    fn encode(self) -> Vec<u8> {
        b"*-1\r\n".to_vec()
    }
}

// - array: "*<number-of-elements>\r\n<element-1>...<element-n>"
impl RespEncode for RespArray {
    fn encode(self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(BUF_CAP);
        buf.extend_from_slice(&format!("*{}\r\n", self.0.len()).into_bytes());
        for frame in self.0 {
            buf.extend_from_slice(&frame.encode());
        }
        buf
    }
}

// - map: "%<number-of-entries>\r\n<key-1><value-1>...<key-n><value-n>"
impl RespEncode for RespMap {
    fn encode(self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(BUF_CAP);
        buf.extend_from_slice(&format!("%{}\r\n", self.0.len()).into_bytes());
        for (key, value) in self.0 {
            buf.extend_from_slice(&SimpleString::new(key).encode());
            buf.extend_from_slice(&value.encode());
        }
        buf
    }
}

// - set: "~<number-of-elements>\r\n<element-1>...<element-n>"
impl RespEncode for RespSet {
    fn encode(self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(BUF_CAP);
        buf.extend_from_slice(&format!("~{}\r\n", self.0.len()).into_bytes());
        for frame in self.0 {
            buf.extend_from_slice(&frame.encode());
        }
        buf
    }
}

#[cfg(test)]
mod tests {
    use crate::RespFrame;

    use super::*;

    #[test]
    fn test_simple_string_encode() {
        let frame: RespFrame = SimpleString::new("OK".to_string()).into();
        assert_eq!(frame.encode(), b"+OK\r\n");
    }

    #[test]
    fn test_simple_error_encode() {
        let frame: RespFrame = SimpleError::new("Error message".to_string()).into();
        assert_eq!(frame.encode(), b"-Error message\r\n");
    }

    #[test]
    fn test_integer_encode() {
        let frame: RespFrame = 123.into();
        assert_eq!(frame.encode(), b":+123\r\n");

        let frame: RespFrame = (-123).into();
        assert_eq!(frame.encode(), b":-123\r\n");
    }

    #[test]
    fn test_boolean_encode() {
        let frame: RespFrame = true.into();
        assert_eq!(frame.encode(), b"#t\r\n");

        let frame: RespFrame = false.into();
        assert_eq!(frame.encode(), b"#f\r\n");
    }

    #[test]
    fn test_double_encode() {
        let frame: RespFrame = 123.456.into();
        assert_eq!(String::from_utf8_lossy(&frame.encode()), ",+1.23456e2\r\n");

        let frame: RespFrame = (-123.456).into();
        assert_eq!(frame.encode(), b",-1.23456e2\r\n");

        let frame: RespFrame = (-0.0123456).into();
        assert_eq!(frame.encode(), b",-1.23456e-2\r\n");

        let frame: RespFrame = 1.23456e+8.into();
        assert_eq!(frame.encode(), b",+1.23456e8\r\n");

        let frame: RespFrame = (-1.23456e-8).into();
        assert_eq!(frame.encode(), b",-1.23456e-8\r\n");
    }

    #[test]
    fn test_bulk_string_encode() {
        let frame: RespFrame = BulkString::new(b"Hello, world!".to_vec()).into();
        assert_eq!(frame.encode(), b"$13\r\nHello, world!\r\n");
    }

    #[test]
    fn test_null_bulk_string_encode() {
        let frame: RespFrame = RespNullBulkString.into();
        assert_eq!(frame.encode(), b"$-1\r\n");
    }

    #[test]
    fn test_null_encode() {
        let frame: RespFrame = RespNull.into();
        assert_eq!(frame.encode(), b"_\r\n");
    }

    #[test]
    fn test_null_array_encode() {
        let frame: RespFrame = RespNullArray.into();
        assert_eq!(frame.encode(), b"*-1\r\n");
    }

    #[test]
    fn test_array_encode() {
        let frame: RespFrame = RespArray::new(vec![
            123.into(),
            SimpleString::new("OK".to_string()).into(),
            BulkString::new(b"Hello, world!".to_vec()).into(),
        ])
        .into();
        assert_eq!(
            frame.encode(),
            b"*3\r\n:+123\r\n+OK\r\n$13\r\nHello, world!\r\n"
        );
    }

    // #[test]
    // fn test_map_encode() {
    //     let frame: RespFrame = RespMap::new(
    //         vec![
    //             ("key1".to_string(), 123.into()),
    //             ("key2".to_string(), SimpleString::new("OK".to_string()).into()),
    //             ("key3".to_string(), BulkString::new(b"Hello, world!".to_vec()).into()),
    //         ]
    //         .into_iter()
    //         .collect::<BTreeMap<String, RespFrame>>(), // Add type annotation here
    //     )
    //     .into();
    //     assert_eq!(
    //         frame.encode(),
    //         b"%3\r\n+key1\r\n:+123\r\n+key2\r\n+OK\r\n+key3\r\n$13\r\nHello, world!\r\n"
    //     );
    // }

    #[test]
    fn test_map_encode2() {
        let mut map = RespMap::new();
        map.insert("key1".to_string(), 123.into());
        map.insert(
            "key2".to_string(),
            SimpleString::new("OK".to_string()).into(),
        );
        map.insert(
            "key3".to_string(),
            BulkString::new(b"Hello, world!".to_vec()).into(),
        );

        assert_eq!(
            map.encode(),
            b"%3\r\n+key1\r\n:+123\r\n+key2\r\n+OK\r\n+key3\r\n$13\r\nHello, world!\r\n"
        );
    }

    #[test]
    fn test_set_encode() {
        let frame: RespFrame = RespSet::new(vec![
            123.into(),
            SimpleString::new("OK".to_string()).into(),
            BulkString::new(b"Hello, world!".to_vec()).into(),
        ])
        .into();
        assert_eq!(
            frame.encode(),
            b"~3\r\n:+123\r\n+OK\r\n$13\r\nHello, world!\r\n"
        );
    }
}
