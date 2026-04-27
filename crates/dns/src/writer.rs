use std::collections::HashMap;

use anyhow::ensure;
use bytes::{BufMut, Bytes, BytesMut};
use once_cell::sync::OnceCell;

use crate::domain_name::DomainName;

/// Build a wire-format suffix key from a slice of raw label byte slices.
///
/// The key is the concatenation of each label prefixed by its length byte,
/// followed by a root label (0). This is the exact wire encoding of the suffix.
fn wire_suffix_key(labels: &[&[u8]]) -> Vec<u8> {
    let len: usize = labels.iter().map(|l| 1 + l.len()).sum::<usize>() + 1;
    let mut key = Vec::with_capacity(len);
    for label in labels {
        key.push(label.len() as u8);
        key.extend_from_slice(label);
    }
    key.push(0);
    key
}

pub struct DnsMessageWriter {
    buf: BytesMut,
    max_len: usize,
    label_pointers: OnceCell<HashMap<Vec<u8>, u16>>,
}

impl Default for DnsMessageWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl DnsMessageWriter {
    /// Create a new DNS message writer with a custom buffer capacity.
    pub fn new_with_max(max_len: usize) -> Self {
        Self {
            buf: BytesMut::with_capacity(max_len.min(512)), // 512 is min dns message payload size.
            max_len,
            label_pointers: OnceCell::new(),
        }
    }

    /// Create a new DNS message writer.
    pub fn new() -> Self {
        Self::new_with_max(1232)
    }

    #[inline]
    fn ensure_space(&mut self, need: usize, what: &str) -> anyhow::Result<()> {
        let cur = self.buf.len();
        let new_len = cur
            .checked_add(need)
            .ok_or_else(|| anyhow::anyhow!("length overflow"))?;
        ensure!(
            new_len <= self.max_len,
            "buffer overflow while writing {}: need={} current_len={} max_len={}",
            what,
            need,
            cur,
            self.max_len
        );
        if new_len > self.buf.capacity() {
            // grow but never beyond max_len
            self.buf.reserve(new_len - self.buf.capacity());
        }
        Ok(())
    }

    /// Write a u8 to the buffer.
    pub fn write_u8(&mut self, value: u8) -> anyhow::Result<()> {
        self.ensure_space(std::mem::size_of::<u8>(), "u8")?;
        self.buf.put_u8(value);
        Ok(())
    }

    /// Write a u16 to the buffer.
    pub fn write_u16(&mut self, value: u16) -> anyhow::Result<()> {
        self.ensure_space(std::mem::size_of::<u16>(), "u16")?;
        self.buf.put_u16(value);
        Ok(())
    }

    /// Write a u32 to the buffer.
    pub fn write_u32(&mut self, value: u32) -> anyhow::Result<()> {
        self.ensure_space(std::mem::size_of::<u32>(), "u32")?;
        self.buf.put_u32(value);
        Ok(())
    }

    // Write a compressed qname to the buffer.
    pub fn write_qname(&mut self, name: &DomainName) -> anyhow::Result<()> {
        if name.is_root() {
            return self.write_u8(0);
        }

        let labels: Vec<&[u8]> = name.label_iter().collect();

        self.ensure_space(name.wire_len(), "qname")?;

        for i in 0..labels.len() {
            let suffix_key = wire_suffix_key(&labels[i..]);

            let ptrs = self.label_pointers.get_or_init(HashMap::default);
            if let Some(&offset) = ptrs.get(&suffix_key) {
                let ptr = 0xC000 | offset;
                self.write_u16(ptr)?;
                return Ok(());
            }

            let pos = self.position();

            let ptrs = self
                .label_pointers
                .get_mut()
                .ok_or(anyhow::anyhow!("expected label_pointers to be initialized"))?;

            if pos <= 0x3FFF {
                ptrs.insert(suffix_key, pos as u16);
            }

            let label = labels[i];
            self.write_u8(label.len() as u8)?;
            self.write_bytes(label)?;
        }

        self.write_u8(0)?;

        Ok(())
    }

    /// Write an uncompressed qname to the buffer.
    ///
    /// This function is mainly intended for EDNS where compression is forbidden.
    pub fn write_qname_uncompressed(&mut self, name: &DomainName) -> anyhow::Result<()> {
        if name.is_root() {
            return self.write_u8(0);
        }

        self.ensure_space(name.wire_len(), "qname")?;

        for label in name.label_iter() {
            self.buf.put_u8(label.len() as u8);
            self.buf.extend_from_slice(label);
        }
        self.buf.put_u8(0); // terminator
        Ok(())
    }

    /// Write raw bytes to the buffer.
    pub fn write_bytes(&mut self, data: &[u8]) -> anyhow::Result<()> {
        self.ensure_space(data.len(), "raw bytes")?;
        self.buf.extend_from_slice(data);
        Ok(())
    }

    /// Overwrite bytes at a defined position.
    pub fn overwrite_bytes(&mut self, position: usize, data: &[u8]) -> anyhow::Result<()> {
        let end = position
            .checked_add(data.len())
            .ok_or_else(|| anyhow::anyhow!("overwrite overflow"))?;
        anyhow::ensure!(
            end <= self.buf.len(),
            "overwrite_bytes OOB: pos={} len={} buf_len={}",
            position,
            data.len(),
            self.buf.len()
        );
        self.buf[position..end].copy_from_slice(data);

        Ok(())
    }

    /// Write a `String` to the buffer.
    pub fn write_string(&mut self, str: &String) -> anyhow::Result<()> {
        self.ensure_space(str.len(), "str")?;
        self.buf.extend_from_slice(str.as_bytes());
        Ok(())
    }

    /// Get the underlying buffer.
    pub fn into_bytes(self) -> Bytes {
        self.buf.freeze()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn position(&self) -> usize {
        self.buf.len()
    }
}

/// Trait for types that can be serialized into DNS wire format
pub trait DnsWritable {
    fn write_to(&self, writer: &mut DnsMessageWriter) -> anyhow::Result<()>;
}
