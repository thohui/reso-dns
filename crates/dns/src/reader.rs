use std::collections::HashSet;

use anyhow::{bail, ensure};

use crate::domain_name::DomainName;

/// A reader for DNS messages that allows reading various components
pub struct DnsMessageReader<'a> {
    /// Internal buffer containing the DNS message.
    buffer: &'a [u8],
    /// Position in bytes.
    position: usize,
}

impl<'a> DnsMessageReader<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer, position: 0 }
    }

    /// Seek the a position inside the buffer.
    pub fn seek(&mut self, pos: usize) -> anyhow::Result<()> {
        let len = self.buffer.len();
        ensure!(pos <= len, "seek out of bounds: pos={} len={}", pos, len);
        self.position = pos;
        Ok(())
    }

    #[inline]
    fn need(&self, need: usize, what: &str) -> anyhow::Result<()> {
        let rem = self.remaining();
        ensure!(
            need <= rem,
            "buffer underflow at pos {} while reading {}: need {} bytes, have {}",
            self.position,
            what,
            need,
            rem
        );
        Ok(())
    }

    #[inline]
    fn need_at(&self, upto_exclusive: usize, what: &str) -> anyhow::Result<()> {
        ensure!(
            upto_exclusive <= self.buffer.len(),
            "buffer underflow while reading {}: need bytes up to {}, len {} (pos {})",
            what,
            upto_exclusive,
            self.buffer.len(),
            self.position
        );
        Ok(())
    }

    /// Read a single byte from the DNS message.
    pub fn read_u8(&mut self) -> anyhow::Result<u8> {
        self.need(std::mem::size_of::<u8>(), "u8")?;
        let byte = self.buffer[self.position];
        self.position += 1;
        Ok(byte)
    }

    /// Read a u16 from the DNS message.
    pub fn read_u16(&mut self) -> anyhow::Result<u16> {
        self.need(std::mem::size_of::<u16>(), "u16")?;

        let bytes = &self.buffer[self.position..self.position + 2];
        let word = u16::from_be_bytes([bytes[0], bytes[1]]);

        self.position += 2;

        Ok(word)
    }

    /// Read a u32 from the DNS message.
    pub fn read_u32(&mut self) -> anyhow::Result<u32> {
        self.need(std::mem::size_of::<u32>(), "u32")?;

        let data = &self.buffer[self.position..self.position + 4];
        let qword = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);

        self.position += 4;
        Ok(qword)
    }

    /// Read a DNS name (qname) from the message.
    pub fn read_qname(&mut self) -> anyhow::Result<DomainName> {
        let mut pos = self.position;
        let mut jumped = false;
        let mut seen = HashSet::new();
        let mut name = String::new();

        loop {
            if pos >= self.buffer.len() {
                bail!("qname of out bounds at pos {} (buf len {})", pos, self.buffer.len())
            }

            // Check for loops
            if !seen.insert(pos) {
                bail!("qname compression pointer loop detected at pos {}", pos);
            }

            let length = self.buffer[pos];

            // Check if it's a pointer (two most significant bits are 1)
            if length & 0xC0 == 0xC0 {
                // Must have two bytes for pointer
                self.need_at(pos + 2, "compression pointer")?;

                let b2 = self.buffer[pos + 1];
                let offset = (((length as usize) & 0x3F) << 8) | (b2 as usize);

                if offset >= self.buffer.len() {
                    bail!(
                        "compression pointer offset {} out of bounds (buf len {})",
                        offset,
                        self.buffer.len()
                    );
                }

                if !jumped {
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
                    bail!(
                        "label overruns buffer at pos {}: need {} bytes, have {}",
                        pos,
                        label_len,
                        self.buffer.len().saturating_sub(pos)
                    );
                }

                let label_bytes = &self.buffer[pos..pos + label_len];

                let label_str = String::from_utf8_lossy(label_bytes);

                name.push_str(&label_str);
                name.push('.');

                pos += label_len;

                if !jumped {
                    self.position = pos;
                }
            }
        }

        if name.is_empty() {
            return Ok(DomainName::root());
        }

        name.pop();
        DomainName::from_ascii(name)
    }

    /// Read an uncompressed dns name from the next `length` bytes.
    ///
    /// This function is mainly intended for EDNS where compression is forbidden.
    pub fn read_qname_uncompressed(&mut self, len: usize) -> anyhow::Result<DomainName> {
        ensure!(len > 0, "read_qname_uncompressed called with len = 0");

        self.need(len, "uncompressed qname")?;

        let start = self.position;
        let end = start + len;
        let mut pos = start;
        let mut name = String::new();

        loop {
            if pos >= end {
                bail!(
                    "unterminated qname in uncompressed name (no root label within len = {})",
                    len
                );
            }

            let length = self.buffer[pos];
            pos += 1;

            if length == 0 {
                // End of name
                break;
            }

            // Compression not allowed in EDNS qnames
            if length & 0xC0 != 0 {
                bail!(
                    "compression pointer (0x{:02x}) not allowed in uncompressed qname",
                    length
                );
            }

            let label_len = length as usize;

            if pos + label_len > end {
                bail!(
                    "label overruns option boundary in uncompressed qname: pos={} label_len={} end={}",
                    pos,
                    label_len,
                    end
                );
            }

            let label_bytes = &self.buffer[pos..pos + label_len];
            pos += label_len;

            if !name.is_empty() {
                name.push('.');
            }
            name.push_str(&String::from_utf8_lossy(label_bytes));
        }

        ensure!(
            pos == end,
            "extra bytes after qname in uncompressed name: pos={} end={}",
            pos,
            end
        );

        self.position = end;

        DomainName::from_ascii(name)
    }

    /// Read a specified number of bytes from the DNS message.
    pub fn read_bytes(&mut self, length: usize) -> anyhow::Result<&'a [u8]> {
        self.need(length, "raw bytes")?;
        let data = &self.buffer[self.position..self.position + length];
        self.position += length;
        Ok(data)
    }

    #[inline]
    /// Current reading position in the buffer.
    pub fn position(&self) -> usize {
        self.position
    }

    /// Return the number of unread bytes remaining in the reader's buffer.
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// use reso_dns::reader::DnsMessageReader;
    /// let buf = [0u8, 1, 2];
    /// let mut r = DnsMessageReader::new(&buf);
    /// assert_eq!(r.remaining(), 3);
    /// r.seek(1).unwrap();
    /// assert_eq!(r.remaining(), 2);
    /// ```
    #[inline]
    pub fn remaining(&self) -> usize {
        self.buffer.len() - self.position
    }
}

/// Trait for types that can be directly parsed from a DNS message.
pub trait DnsReadable: Sized {
    fn read_from(reader: &mut DnsMessageReader) -> anyhow::Result<Self>;
}

mod tests {
    use crate::{DnsMessageWriter, domain_name::DomainName};

    #[test]
    fn test_read_qname_uncompressed() {
        use super::DnsMessageReader;
        let name = "mail.google.com";
        let dname = DomainName::from_user(name).unwrap();
        let mut writer = DnsMessageWriter::new();

        writer.write_qname_uncompressed(&dname).unwrap();

        let bytes = writer.into_bytes();
        let mut reader = DnsMessageReader::new(&bytes);

        let decoded = reader.read_qname_uncompressed(bytes.len()).unwrap();

        assert!(dname.as_str() == decoded.as_str());
    }
}
