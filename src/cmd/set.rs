use crate::{RespArray, RespFrame};

use super::{
    extract_args, validate_command, CommandError, CommandExecutor, SAdd, SIsMember, RESP_OK,
};

impl CommandExecutor for SAdd {
    fn execute(self, backend: &crate::Backend) -> RespFrame {
        backend.sadd(self.key.to_owned(), self.members);
        RESP_OK.clone()
    }
}

impl CommandExecutor for SIsMember {
    fn execute(self, backend: &crate::Backend) -> RespFrame {
        let ret = backend.s_is_member(&self.key, self.member);
        RespFrame::Boolean(ret)
    }
}

impl TryFrom<RespArray> for SAdd {
    type Error = CommandError;
    fn try_from(value: RespArray) -> Result<Self, Self::Error> {
        validate_command(&value, &["sadd"], value.len() - 1)?;

        let mut args = extract_args(value, 1)?.into_iter();
        match args.next() {
            Some(RespFrame::BulkString(key)) => {
                let members = args.collect();
                Ok(SAdd {
                    key: String::from_utf8_lossy(&key.0).into(),
                    members,
                })
            }
            _ => Err(CommandError::InvalidArgument(
                "Invalid key or members".to_string(),
            )),
        }
    }
}

impl TryFrom<RespArray> for SIsMember {
    type Error = CommandError;
    fn try_from(value: RespArray) -> Result<Self, Self::Error> {
        validate_command(&value, &["sismember"], value.len() - 1)?;

        let mut args = extract_args(value, 1)?.into_iter();
        match (args.next(), args.next()) {
            (Some(RespFrame::BulkString(key)), Some(member)) => Ok(SIsMember {
                key: String::from_utf8(key.0)?,
                member,
            }),
            _ => Err(CommandError::InvalidArgument(
                "Invalid key or field".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use dashmap::DashMap;

    use crate::{ApproximateFloat, Backend};

    #[test]
    fn test_double_set() {
        let set: DashMap<ApproximateFloat, ()> = DashMap::new();
        set.insert(ApproximateFloat(1.0), ());
        set.insert(ApproximateFloat(2.0), ());
        set.insert(ApproximateFloat(3.0), ());

        // Check if the set contains a value
        if set.contains_key(&ApproximateFloat(1.0)) {
            println!("Set contains 1.0");
        }

        // Remove a value
        set.remove(&ApproximateFloat(2.0));

        // Iterate over the set
        for key in set.iter() {
            println!("Key: {:?}", key.key());
        }
    }

    #[test]
    fn test_sadd() -> Result<()> {
        let backend = Backend::new();
        let vec = vec![
            RespFrame::BulkString("sadd".into()),
            RespFrame::BulkString("myset".into()),
            RespFrame::BulkString("a".into()),
            RespFrame::BulkString("b".into()),
            RespFrame::BulkString("c".into()),
        ];
        let cmd = RespArray(vec);

        let cmd = SAdd::try_from(cmd)?;
        cmd.execute(&backend);

        println!("{:?}", &backend.set);

        assert!(backend.s_is_member("myset", RespFrame::BulkString("a".into())));
        assert!(backend.s_is_member("myset", RespFrame::BulkString("b".into())));
        assert!(backend.s_is_member("myset", RespFrame::BulkString("c".into())));
        assert!(!backend.s_is_member("myset", RespFrame::BulkString("d".into())));

        Ok(())
    }
}
