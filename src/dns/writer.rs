use anyhow::ensure;
use bytes::{BufMut, Bytes, BytesMut};

pub struct DnsMessageWriter {
    buf: BytesMut,
    max_len: usize,
}

impl DnsMessageWriter {
    /// Create a new DNS message writer with a custom buffer capacity.
    pub fn new_with_max(max_len: usize) -> Self {
        Self {
            buf: BytesMut::with_capacity(max_len.min(512)), // 512 is min dns message payload size.
            max_len,
        }
    }

    /// Create a new DNS message writer with the default dns message size (512 bytes)
    pub fn new() -> Self {
        Self::new_with_max(512)
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

    /// Write a qname to the buffer.
    pub fn write_qname(&mut self, qname: &str) -> anyhow::Result<()> {
        // TODO: support compression.
        if qname == "." {
            // root label
            return self.write_u8(0);
        }

        let mut total = 1; // for the final zero
        for label in qname.trim_end_matches('.').split('.') {
            ensure!(!label.is_empty(), "empty label in qname '{}'", qname);
            ensure!(label.len() <= 63, "label '{}' exceeds 63 bytes", label);
            total += 1 + label.len();
        }
        ensure!(
            total <= 255,
            "qname too long ({} bytes): '{}'",
            total,
            qname
        );

        self.ensure_space(total, "qname")?;
        for label in qname.trim_end_matches('.').split('.') {
            self.buf.put_u8(label.len() as u8);
            self.buf.extend_from_slice(label.as_bytes());
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

    /// Get the underlying buffer.
    pub fn into_bytes(self) -> Bytes {
        self.buf.freeze()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.buf.len()
    }
}
