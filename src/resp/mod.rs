mod decode;
mod encode;

use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

use enum_dispatch::enum_dispatch;

#[enum_dispatch]
pub trait RespEncode {
    fn encode(self) -> Vec<u8>;
}

pub trait RespDecode {
    fn decode(buf: Self) -> Result<RespFrame, String>;
}

#[enum_dispatch(RespEncode)]
#[derive(Debug, PartialEq, PartialOrd)]
pub enum RespFrame {
    SimpleString(SimpleString),
    Error(SimpleError),
    Integer(i64),
    BulkString(BulkString),
    Array(RespArray),
    Null(RespNull),
    NullArray(RespNullArray),
    NullBulkString(RespNullBulkString),
    Boolean(bool),
    Double(f64),
    Map(RespMap),
    Set(RespSet),
}

#[derive(Debug, PartialEq, PartialOrd)]
pub struct SimpleString(String);

#[derive(Debug, PartialEq, PartialOrd)]
pub struct SimpleError(String);

#[derive(Debug, PartialEq, PartialOrd)]
pub struct BulkString(Vec<u8>);

#[derive(Debug, PartialEq, PartialOrd)]
pub struct RespArray(Vec<RespFrame>);

#[derive(Debug, PartialEq, PartialOrd)]
pub struct RespNull;

#[derive(Debug, PartialEq, PartialOrd)]
pub struct RespNullArray;

#[derive(Debug, PartialEq, PartialOrd)]
pub struct RespNullBulkString;

#[derive(Debug, PartialEq, PartialOrd)]
pub struct RespMap(BTreeMap<String, RespFrame>);

#[derive(Debug, PartialEq, PartialOrd)]
pub struct RespSet(Vec<RespFrame>);

impl Deref for SimpleString {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for SimpleError {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for BulkString {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for RespArray {
    type Target = Vec<RespFrame>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for RespMap {
    type Target = BTreeMap<String, RespFrame>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RespMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for RespSet {
    type Target = Vec<RespFrame>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl SimpleString {
    pub fn new(s: impl Into<String>) -> Self {
        SimpleString(s.into())
    }
}

impl SimpleError {
    pub fn new(s: impl Into<String>) -> Self {
        SimpleError(s.into())
    }
}

impl BulkString {
    pub fn new(s: impl Into<Vec<u8>>) -> Self {
        BulkString(s.into())
    }
}

impl RespArray {
    pub fn new(arr: impl Into<Vec<RespFrame>>) -> Self {
        RespArray(arr.into())
    }
}
// impl RespMap {
//     pub fn new(map: impl Into<BTreeMap<String, RespFrame>>) -> Self {
//         RespMap(map.into())
//     }
// }
impl RespMap {
    pub fn new() -> Self {
        RespMap(BTreeMap::new())
    }
}

impl Default for RespMap {
    fn default() -> Self {
        RespMap::new()
    }
}

impl RespSet {
    pub fn new(set: impl Into<Vec<RespFrame>>) -> Self {
        let s = set.into();
        // let mut set = BTreeSet::new();
        // for frame in s {
        //     set.insert(frame);
        // }
        RespSet(s)
    }
}
