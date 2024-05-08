use crate::{cmd::CommandError, BulkString, RespArray, RespFrame};

use super::{extract_args, validate_command, CommandExecutor, Echo};

impl CommandExecutor for Echo {
    fn execute(self, _backend: &crate::Backend) -> RespFrame {
        RespFrame::BulkString(BulkString(self.message.into_bytes()))
    }
}

impl TryFrom<RespArray> for Echo {
    type Error = CommandError;
    fn try_from(value: RespArray) -> Result<Self, Self::Error> {
        validate_command(&value, &["echo"], 1)?;
        let mut args = extract_args(value, 1)?.into_iter();
        let message = match args.next() {
            Some(RespFrame::BulkString(s)) => String::from_utf8(s.0)?,
            _ => return Err(CommandError::InvalidArgument("Invalid message".to_string())),
        };

        Ok(Echo { message })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Backend, RespDecode};
    use anyhow::Result;
    use bytes::BytesMut;

    #[test]
    fn test_echo_from_resp_array() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*2\r\n$4\r\necho\r\n$5\r\nhello\r\n");
        let frame = RespArray::decode(&mut buf)?;
        let echo = Echo::try_from(frame)?;
        assert_eq!(echo.message, "hello");
        Ok(())
    }

    #[test]
    fn test_echo_execute() {
        let backend = Backend::new();
        let echo = Echo {
            message: "hello".to_string(),
        };
        let frame = echo.execute(&backend);
        assert_eq!(
            frame,
            RespFrame::BulkString(BulkString("hello".to_string().into_bytes()))
        );
    }
}
