use anyhow::anyhow;
use bytes::{Bytes, BytesMut};
use lru_cache::LruCache;
use std::{hash::Hash, sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::Instant};

use crate::dns::{
    self,
    message::{DnsMessage, EdnsOption},
};
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct CacheKey {
    pub name: Arc<str>,
    pub qtype: dns::message::RecordType,
    pub qclass: dns::message::ClassType,
    pub do_bit: bool, // edns bit
}

impl CacheKey {
    pub fn from_message(message: &DnsMessage) -> anyhow::Result<Self> {
        if let Some(question) = message.questions().first() {
            Ok(CacheKey {
                name: question.qname.clone(),
                qclass: question.qclass,
                qtype: question.qtype,
                do_bit: message
                    .edns()
                    .as_ref()
                    .map(|e| (e.z_flags & 0x8000) != 0)
                    .unwrap_or(false),
            })
        } else {
            Err(anyhow!("no question in message"))
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct CacheEntry {
    expires_at: Instant,
    raw_response: DnsResponseBytes,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct DnsResponseBytes(Bytes);
impl DnsResponseBytes {
    pub fn new(bytes: Bytes) -> Self {
        Self(bytes)
    }
    pub fn into_custom_response(self, transaction_id: u16) -> Bytes {
        let mut bytes = BytesMut::from(&self.0[0..]);
        // replace the transaction id in the cached response.
        bytes[0] = (transaction_id >> 8) as u8;
        bytes[1] = (transaction_id & 0xFF) as u8;
        bytes.freeze()
    }
}
pub struct CacheService {
    cache: Mutex<LruCache<CacheKey, CacheEntry>>,
}

impl CacheService {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(LruCache::new(8192)),
        }
    }
    pub async fn lookup(&self, key: &CacheKey) -> Option<DnsResponseBytes> {
        let mut c = self.cache.lock().await;

        if let Some(entry) = c.get_mut(key) {
            if entry.expires_at > Instant::now() {
                let resp = &entry.raw_response;
                return Some(resp.clone());
            } else {
                c.remove(key);
            }
        }
        None
    }

    pub async fn insert(
        &self,
        query_msg: &DnsMessage,
        resp_bytes: Bytes,
        resp_msg: DnsMessage,
    ) -> anyhow::Result<()> {
        // dont cache truncated
        if resp_msg.flags.tc {
            return Ok(());
        }

        let ttl = resp_msg
            .answers()
            .iter()
            .chain(resp_msg.authority_records())
            .chain(resp_msg.additional_records())
            .map(|r| r.ttl())
            .min()
            .unwrap_or(0);

        if ttl == 0 {
            return Ok(());
        }

        // skip if there is an edns cookie (for now)
        let has_cookie = resp_msg
            .edns()
            .as_ref()
            .map(|e| {
                e.options
                    .iter()
                    .any(|opt| matches!(opt, EdnsOption::Cookie(_)))
            })
            .unwrap_or(false);
        if has_cookie {
            return Ok(());
        }

        let key = CacheKey::from_message(query_msg)?;
        let entry = CacheEntry {
            expires_at: Instant::now() + Duration::from_secs(ttl as u64),
            raw_response: DnsResponseBytes::new(resp_bytes),
        };

        self.cache.lock().await.insert(key, entry);
        Ok(())
    }
}
