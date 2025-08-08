use std::collections::HashSet;

use bytes::{Buf, Bytes};

/// A reader for DNS messages that allows reading various components
pub struct DnsMessageReader<'a> {
    /// Internal buffer containing the DNS message.
    buffer: &'a [u8],
    /// Position in bytes.
    position: usize,
}

impl<'a> DnsMessageReader<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self {
            buffer,
            position: 0,
        }
    }

    /// Seek the a position inside the buffer.
    pub fn seek(&mut self, pos: usize) -> anyhow::Result<()> {
        if pos > self.buffer.len() {
            return Err(anyhow::anyhow!("seek out of bounds"));
        }
        self.position = pos;
        Ok(())
    }

    /// Read a single byte from the DNS message.
    pub fn read_u8(&mut self) -> anyhow::Result<u8> {
        if self.position >= self.buffer.len() {
            return Err(anyhow::anyhow!("Buffer underflow while reading u8"));
        }
        let byte = self.buffer[self.position];
        self.position += 1;
        Ok(byte)
    }

    /// Read a u16 from the DNS message.
    pub fn read_u16(&mut self) -> anyhow::Result<u16> {
        if self.position + 2 > self.buffer.len() {
            return Err(anyhow::anyhow!("Buffer underflow while reading u16"));
        }

        let bytes = &self.buffer[self.position..self.position + 2];
        let word = u16::from_be_bytes([bytes[0], bytes[1]]);

        self.position += 2;

        Ok(word)
    }

    /// Read a u32 from the DNS message.
    pub fn read_u32(&mut self) -> anyhow::Result<u32> {
        if self.position + 4 > self.buffer.len() {
            return Err(anyhow::anyhow!("Buffer underflow while reading u32"));
        }
        let data = &self.buffer[self.position..self.position + 4];
        let qword = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);

        self.position += 4;
        Ok(qword)
    }

    /// Read a DNS name (qname) from the message.
    pub fn read_qname(&mut self) -> anyhow::Result<String> {
        let mut pos = self.position;
        let mut jumped = false;
        let mut seen = HashSet::new();
        let mut name = String::new();

        loop {
            if pos >= self.buffer.len() {
                return Err(anyhow::anyhow!("Out of bounds while reading qname"));
            }

            // Check for loops
            if !seen.insert(pos) {
                return Err(anyhow::anyhow!("DNS compression pointer loop detected"));
            }

            let length = self.buffer[pos];

            // Check if it's a pointer (two most significant bits are 1)
            if length & 0xC0 == 0xC0 {
                // Must have two bytes for pointer
                if pos + 1 >= self.buffer.len() {
                    return Err(anyhow::anyhow!("Pointer points outside buffer"));
                }

                let b2 = self.buffer[pos + 1];
                let offset = (((length as usize) & 0x3F) << 8) | (b2 as usize);

                if offset >= self.buffer.len() {
                    return Err(anyhow::anyhow!("Pointer offset out of bounds"));
                }

                if !jumped {
                    // Move the reader position past the pointer
                    self.position = pos + 2;
                }

                pos = offset;
                jumped = true;
                continue;
            } else if length == 0 {
                // End of name
                if !jumped {
                    self.position = pos + 1;
                }
                break;
            } else {
                let label_len = length as usize;
                pos += 1;

                if pos + label_len > self.buffer.len() {
                    return Err(anyhow::anyhow!("Label length goes past buffer"));
                }

                let label_bytes = &self.buffer[pos..pos + label_len];

                // Use from_utf8_lossy to avoid UTF-8 panic
                let label_str = String::from_utf8_lossy(label_bytes);

                name.push_str(&label_str);
                name.push('.');

                pos += label_len;

                if !jumped {
                    self.position = pos;
                }
            }
        }

        if name.ends_with('.') {
            name.pop();
        }

        Ok(name)
    }

    /// Read a specified number of bytes from the DNS message.
    pub fn read_bytes(&mut self, length: usize) -> anyhow::Result<&'a [u8]> {
        if self.position + length > self.buffer.len() {
            return Err(anyhow::anyhow!("Buffer underflow while reading bytes"));
        }
        let data = &self.buffer[self.position..self.position + length];
        self.position += length;
        Ok(data)
    }

    /// Return the current position in the buffer.
    pub fn position(&self) -> usize {
        self.position
    }

    /// Return the remaining bytes in the buffer.
    pub fn remaining(&self) -> usize {
        self.buffer.len() - self.position
    }
}

mod tests {}
