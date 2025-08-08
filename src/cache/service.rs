use bytes::Bytes;
use lru_cache::LruCache;
use std::{hash::Hash, sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::Instant};

use crate::dns::{
    self,
    message::{DnsMessage, EdnsOption},
    reader::DnsMessageReader,
};
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct CacheKey {
    pub name: String,
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
                do_bit: message.edns().is_some(),
            })
        } else {
            anyhow::bail!("no question in message")
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct CacheEntry {
    expires_at: Instant,
    raw_response: Bytes,
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
    pub async fn lookup(&self, key: &CacheKey) -> Option<Bytes> {
        let mut c = self.cache.lock().await;

        if let Some(entry) = c.get_mut(key) {
            if entry.expires_at > Instant::now() {
                return Some(entry.raw_response.clone());
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
        resp_msg: &DnsMessage,
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
            raw_response: resp_bytes,
        };

        self.cache.lock().await.insert(key, entry);
        Ok(())
    }
}
