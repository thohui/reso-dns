use bytes::{BufMut, Bytes, BytesMut};

pub struct DnsMessageWriter {
    buffer: BytesMut,
}

impl DnsMessageWriter {
    /// Create a new DNS message writer.
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::with_capacity(512).freeze().into(),
        }
    }

    /// Write a u8 to the buffer.
    pub fn write_u8(&mut self, value: u8) -> anyhow::Result<()> {
        if self.buffer.len() + std::mem::size_of::<u8>() > self.buffer.capacity() {
            return Err(anyhow::anyhow!(
                "Buffer overflow: not enough space to write u8"
            ));
        }
        self.buffer.put_u8(value);
        Ok(())
    }

    /// Write a u16 to the buffer.
    pub fn write_u16(&mut self, value: u16) -> anyhow::Result<()> {
        if self.buffer.len() + std::mem::size_of::<u16>() > self.buffer.capacity() {
            return Err(anyhow::anyhow!(
                "Buffer overflow: not enough space to write u16"
            ));
        }
        self.buffer.put_u16(value);
        Ok(())
    }

    /// Write a u32 to the buffer.
    pub fn write_u32(&mut self, value: u32) -> anyhow::Result<()> {
        if self.buffer.len() + std::mem::size_of::<u32>() > self.buffer.capacity() {
            return Err(anyhow::anyhow!(
                "Buffer overflow: not enough space to write u32"
            ));
        }
        self.buffer.put_u32(value);
        Ok(())
    }

    /// Write a domain name (QNAME) to the DNS message.
    pub fn write_qname(&mut self, qname: &str) -> anyhow::Result<()> {
        for label in qname.split('.') {
            if label.len() > 63 {
                return Err(anyhow::anyhow!(
                    "Label '{}' exceeds maximum length of 63 characters",
                    label
                ));
            }
            // Write length byte.
            self.write_u8(label.len() as u8)?;
            self.buffer.extend_from_slice(label.as_bytes());
        }
        // Write null byte (end of QNAME)
        self.write_u8(0)?;
        Ok(())
    }

    /// Write a byte slice to the DNS message.
    pub fn write_bytes(&mut self, data: &[u8]) -> anyhow::Result<()> {
        if self.buffer.len() + data.len() > self.buffer.capacity() {
            return Err(anyhow::anyhow!(
                "Buffer overflow: not enough space to write bytes"
            ));
        }
        self.buffer.extend_from_slice(data);
        Ok(())
    }

    /// Convert the internal buffer into a `Bytes` object.
    pub fn into_bytes(self) -> Bytes {
        self.buffer.into()
    }

    /// Get the current length of the buffer.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }
}
