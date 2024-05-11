use crate::{RespArray, RespFrame};
use dashmap::DashMap;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Backend(Arc<BackendInner>);

#[derive(Debug)]
pub struct BackendInner {
    pub(crate) map: DashMap<String, RespFrame>,
    pub(crate) hmap: DashMap<String, DashMap<String, RespFrame>>,
    pub(crate) set: DashMap<String, DashMap<RespFrame, ()>>,
}

impl Deref for Backend {
    type Target = BackendInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for Backend {
    fn default() -> Self {
        Self(Arc::new(BackendInner::default()))
    }
}

impl Default for BackendInner {
    fn default() -> Self {
        Self {
            map: DashMap::new(),
            hmap: DashMap::new(),
            set: DashMap::new(),
        }
    }
}

impl Backend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &str) -> Option<RespFrame> {
        self.map.get(key).map(|v| v.value().clone())
    }

    pub fn set(&self, key: String, value: RespFrame) {
        self.map.insert(key, value);
    }

    pub fn hget(&self, key: &str, field: &str) -> Option<RespFrame> {
        self.hmap
            .get(key)
            .and_then(|v| v.get(field).map(|v| v.value().clone()))
    }

    pub fn hset(&self, key: String, field: String, value: RespFrame) {
        let hmap = self.hmap.entry(key).or_default();
        hmap.insert(field, value);
    }

    pub fn hgetall(&self, key: &str) -> Option<DashMap<String, RespFrame>> {
        self.hmap.get(key).map(|v| v.clone())
    }

    pub fn hmget(&self, key: &str, fields: &[String]) -> Option<RespArray> {
        self.hmap.get(key).map(|hmap| {
            let mut data = Vec::with_capacity(fields.len());
            for field in fields {
                match hmap.get(field) {
                    Some(value) => data.push(value.value().clone()),
                    None => data.push(RespFrame::Null(crate::RespNull)),
                }
            }
            RespArray::new(data)
        })
    }

    pub fn sadd(&self, key: String, members: Vec<RespFrame>) {
        let hset = self.set.entry(key).or_default();
        for member in members {
            hset.insert(member, ());
        }
    }

    pub fn s_is_member(&self, key: &str, member: RespFrame) -> bool {
        match self.set.get(key) {
            Some(hset) => hset.contains_key(&member),
            None => false,
        }
    }
}
