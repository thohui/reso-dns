use crate::{
    domain_name::DomainName,
    error::{DnsReadError, ReadResult, Result},
};

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
    pub fn seek(&mut self, pos: usize) -> ReadResult<()> {
        let len = self.buffer.len();
        if pos > len {
            return Err(DnsReadError::SeekOutOfBounds { pos, len });
        }
        self.position = pos;
        Ok(())
    }

    #[inline]
    fn need(&self, need: usize) -> ReadResult<()> {
        let have = self.remaining();
        if need > have {
            return Err(DnsReadError::BufferUnderflow {
                pos: self.position,
                need,
                have,
            });
        }
        Ok(())
    }

    #[inline]
    fn need_at(&self, upto_exclusive: usize) -> ReadResult<()> {
        if upto_exclusive > self.buffer.len() {
            return Err(DnsReadError::BufferUnderflow {
                pos: self.position,
                need: upto_exclusive - self.position,
                have: self.buffer.len() - self.position,
            });
        }
        Ok(())
    }

    /// Read a single byte from the DNS message.
    pub fn read_u8(&mut self) -> ReadResult<u8> {
        self.need(std::mem::size_of::<u8>())?;
        let byte = self.buffer[self.position];
        self.position += 1;
        Ok(byte)
    }

    /// Read a u16 from the DNS message.
    pub fn read_u16(&mut self) -> ReadResult<u16> {
        self.need(std::mem::size_of::<u16>())?;

        let bytes = &self.buffer[self.position..self.position + 2];
        let word = u16::from_be_bytes([bytes[0], bytes[1]]);

        self.position += 2;

        Ok(word)
    }

    /// Read a u32 from the DNS message.
    pub fn read_u32(&mut self) -> ReadResult<u32> {
        self.need(std::mem::size_of::<u32>())?;

        let data = &self.buffer[self.position..self.position + 4];
        let qword = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);

        self.position += 4;
        Ok(qword)
    }

    /// Read a DNS name (qname) from the message.
    pub fn read_qname(&mut self) -> ReadResult<DomainName> {
        let mut pos = self.position;
        let mut jumped = false;
        let mut seen = Vec::new();
        let mut labels: Vec<Vec<u8>> = Vec::new();

        loop {
            if pos >= self.buffer.len() {
                return Err(DnsReadError::BufferUnderflow {
                    pos,
                    need: 1,
                    have: self.buffer.len().saturating_sub(pos),
                });
            }

            // Check for loops
            if seen.contains(&pos) {
                return Err(DnsReadError::CompressionLoop { offset: pos });
            }

            seen.push(pos);

            let length = self.buffer[pos];

            // Check if it's a pointer (two most significant bits are 1)
            if length & 0xC0 == 0xC0 {
                // Must have two bytes for pointer
                self.need_at(pos + 2)?;

                let b2 = self.buffer[pos + 1];
                let offset = (((length as usize) & 0x3F) << 8) | (b2 as usize);

                if offset >= self.buffer.len() {
                    return Err(DnsReadError::CompressionOutOfBounds {
                        offset,
                        len: self.buffer.len(),
                    });
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
                    return Err(DnsReadError::BufferUnderflow {
                        pos,
                        need: label_len,
                        have: self.buffer.len().saturating_sub(pos),
                    });
                }

                labels.push(self.buffer[pos..pos + label_len].to_vec());

                pos += label_len;

                if !jumped {
                    self.position = pos;
                }
            }
        }

        if labels.is_empty() {
            return Ok(DomainName::root());
        }

        DomainName::from_labels(labels)
    }

    /// Read an uncompressed dns name from the next `len` bytes.
    ///
    /// This function is mainly intended for EDNS where compression is forbidden.
    pub fn read_qname_uncompressed(&mut self, len: usize) -> ReadResult<DomainName> {
        if len == 0 {
            return Err(DnsReadError::BufferUnderflow {
                pos: self.position,
                need: 1,
                have: 0,
            });
        }

        self.need(len)?;

        let start = self.position;
        let end = start + len;
        let mut pos = start;
        let mut labels: Vec<Vec<u8>> = Vec::new();

        loop {
            if pos >= end {
                return Err(DnsReadError::UnterminatedName { len: end - start });
            }

            let length = self.buffer[pos];
            pos += 1;

            if length == 0 {
                // End of name
                break;
            }

            // Compression not allowed in EDNS qnames
            if length & 0xC0 != 0 {
                return Err(DnsReadError::CompressionNotAllowed { byte: length });
            }

            let label_len = length as usize;

            if pos + label_len > end {
                return Err(DnsReadError::BufferUnderflow {
                    pos,
                    need: label_len,
                    have: end - pos,
                });
            }

            labels.push(self.buffer[pos..pos + label_len].to_vec());
            pos += label_len;
        }

        if pos != end {
            return Err(DnsReadError::TrailingBytes { pos, end });
        }

        self.position = end;

        DomainName::from_labels(labels)
    }

    /// Read a specified number of bytes from the DNS message.
    pub fn read_bytes(&mut self, length: usize) -> ReadResult<&'a [u8]> {
        self.need(length)?;
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
    #[inline]
    pub fn remaining(&self) -> usize {
        self.buffer.len() - self.position
    }
}

/// Trait for types that can be directly parsed from a DNS message.
pub trait DnsReadable: Sized {
    fn read_from(reader: &mut DnsMessageReader) -> Result<Self>;
}

#[cfg(test)]
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

    #[test]
    fn test_read_u8() {
        use super::DnsMessageReader;
        let data = [42u8, 100, 255];
        let mut reader = DnsMessageReader::new(&data);

        assert_eq!(reader.read_u8().unwrap(), 42);
        assert_eq!(reader.read_u8().unwrap(), 100);
        assert_eq!(reader.read_u8().unwrap(), 255);
        assert!(reader.read_u8().is_err()); // Buffer underflow
    }

    #[test]
    fn test_read_u16() {
        use super::DnsMessageReader;
        let data = [0x12, 0x34, 0xFF, 0xFF];
        let mut reader = DnsMessageReader::new(&data);

        assert_eq!(reader.read_u16().unwrap(), 0x1234);
        assert_eq!(reader.read_u16().unwrap(), 0xFFFF);
        assert!(reader.read_u16().is_err()); // Buffer underflow
    }

    #[test]
    fn test_read_u32() {
        use super::DnsMessageReader;
        let data = [0x12, 0x34, 0x56, 0x78, 0xFF, 0xFF, 0xFF, 0xFF];
        let mut reader = DnsMessageReader::new(&data);

        assert_eq!(reader.read_u32().unwrap(), 0x12345678);
        assert_eq!(reader.read_u32().unwrap(), 0xFFFFFFFF);
        assert!(reader.read_u32().is_err()); // Buffer underflow
    }

    #[test]
    fn test_read_bytes() {
        use super::DnsMessageReader;
        let data = [1, 2, 3, 4, 5];
        let mut reader = DnsMessageReader::new(&data);

        let bytes = reader.read_bytes(3).unwrap();
        assert_eq!(bytes, &[1, 2, 3]);
        assert_eq!(reader.position(), 3);

        let bytes = reader.read_bytes(2).unwrap();
        assert_eq!(bytes, &[4, 5]);
        assert_eq!(reader.position(), 5);

        assert!(reader.read_bytes(1).is_err()); // Buffer underflow
    }

    #[test]
    fn test_seek() {
        use super::DnsMessageReader;
        let data = [1, 2, 3, 4, 5];
        let mut reader = DnsMessageReader::new(&data);

        assert_eq!(reader.position(), 0);

        reader.seek(3).unwrap();
        assert_eq!(reader.position(), 3);
        assert_eq!(reader.read_u8().unwrap(), 4);

        reader.seek(0).unwrap();
        assert_eq!(reader.position(), 0);
        assert_eq!(reader.read_u8().unwrap(), 1);

        reader.seek(5).unwrap(); // At end is ok
        assert_eq!(reader.position(), 5);

        assert!(reader.seek(6).is_err()); // Out of bounds
    }

    #[test]
    fn test_remaining() {
        use super::DnsMessageReader;
        let data = [1, 2, 3, 4, 5];
        let mut reader = DnsMessageReader::new(&data);

        assert_eq!(reader.remaining(), 5);

        reader.read_u8().unwrap();
        assert_eq!(reader.remaining(), 4);

        reader.read_u16().unwrap();
        assert_eq!(reader.remaining(), 2);

        reader.seek(5).unwrap();
        assert_eq!(reader.remaining(), 0);
    }

    /// Build wire format bytes from a list of raw labels (adds length prefixes + root terminator).
    fn wire_name(labels: &[&[u8]]) -> Vec<u8> {
        let mut out = Vec::new();
        for label in labels {
            out.push(label.len() as u8);
            out.extend_from_slice(label);
        }
        out.push(0);
        out
    }

    #[test]
    fn test_read_qname_simple() {
        use super::DnsMessageReader;
        let data = wire_name(&[b"example", b"com"]);
        let mut reader = DnsMessageReader::new(&data);

        let name = reader.read_qname().unwrap();
        assert_eq!(name.as_str(), "example.com");
    }

    #[test]
    fn test_read_qname_root() {
        use super::DnsMessageReader;
        // Root domain is just a single 0 byte
        let data = vec![0];
        let mut reader = DnsMessageReader::new(&data);

        let name = reader.read_qname().unwrap();
        assert_eq!(name, DomainName::root());
    }

    #[test]
    fn test_read_qname_with_compression() {
        use super::DnsMessageReader;
        // "example.com" at offset 0, then "www.example.com" with pointer to offset 0
        let mut data = wire_name(&[b"example", b"com"]);
        let first_label_offset = 0;
        // Add "www" label then pointer to "example.com"
        data.push(3);
        data.extend_from_slice(b"www");
        data.push(0xC0); // Compression marker
        data.push(first_label_offset);

        let mut reader = DnsMessageReader::new(&data);

        // Read first name
        let name1 = reader.read_qname().unwrap();
        assert_eq!(name1.as_str(), "example.com");

        // Read second name
        let name2 = reader.read_qname().unwrap();
        assert_eq!(name2.as_str(), "www.example.com");
    }

    #[test]
    fn test_read_qname_compression_loop_detection() {
        use super::DnsMessageReader;
        // Create a compression pointer that points to itself
        let data = vec![0xC0, 0x00]; // Points to offset 0 (itself)
        let mut reader = DnsMessageReader::new(&data);

        // Should detect the loop and return an error
        assert!(reader.read_qname().is_err());
    }

    #[test]
    fn test_read_qname_out_of_bounds() {
        use super::DnsMessageReader;
        // Label length claims more bytes than available
        let data = vec![10, b'a', b'b', b'c']; // length says 10 but only 3 bytes follow
        let mut reader = DnsMessageReader::new(&data);

        assert!(reader.read_qname().is_err());
    }

    #[test]
    fn test_read_qname_compression_out_of_bounds() {
        use super::DnsMessageReader;
        // Compression pointer to invalid offset
        let data = vec![0xC0, 0xFF]; // Points way out of bounds
        let mut reader = DnsMessageReader::new(&data);

        assert!(reader.read_qname().is_err());
    }

    #[test]
    fn test_read_qname_uncompressed_with_compression_error() {
        use super::DnsMessageReader;
        // Try to use compression in uncompressed context
        let data = vec![0xC0, 0x00, 0x00]; // Compression pointer
        let mut reader = DnsMessageReader::new(&data);

        // Should error because compression is not allowed
        assert!(reader.read_qname_uncompressed(3).is_err());
    }

    #[test]
    fn test_read_qname_uncompressed_unterminated() {
        use super::DnsMessageReader;
        // Missing root label (0 byte)
        // "com" label with no root terminator
        let data = [3, b'c', b'o', b'm'];
        let mut reader = DnsMessageReader::new(&data);

        assert!(reader.read_qname_uncompressed(data.len()).is_err());
    }

    #[test]
    fn test_read_qname_uncompressed_extra_bytes() {
        use super::DnsMessageReader;
        // Has proper termination but extra bytes beyond the specified length
        // "com" with root terminator + an extra trailing byte
        let data = vec![3, b'c', b'o', b'm', 0, 99];
        let mut reader = DnsMessageReader::new(&data);

        // Read only the 5-byte qname (should fail because of extra byte check)
        // Actually, this should succeed and then we check extra bytes
        let result = reader.read_qname_uncompressed(6);
        // This should error due to extra bytes
        assert!(result.is_err());
    }

    #[test]
    fn test_position_tracking() {
        use super::DnsMessageReader;
        let data = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut reader = DnsMessageReader::new(&data);

        assert_eq!(reader.position(), 0);

        reader.read_u8().unwrap();
        assert_eq!(reader.position(), 1);

        reader.read_u16().unwrap();
        assert_eq!(reader.position(), 3);

        reader.read_u32().unwrap();
        assert_eq!(reader.position(), 7);

        reader.read_u8().unwrap();
        assert_eq!(reader.position(), 8);
    }

    #[test]
    fn test_buffer_underflow_errors() {
        use super::DnsMessageReader;
        let data = [1, 2];
        let mut reader = DnsMessageReader::new(&data);

        // Try to read more than available
        assert!(reader.read_u32().is_err());

        // Position should not have changed
        assert_eq!(reader.position(), 0);

        // Should still be able to read what's available
        assert_eq!(reader.read_u16().unwrap(), 0x0102);
    }

    #[test]
    fn test_read_bytes_zero_length() {
        use super::DnsMessageReader;
        let data = [1, 2, 3];
        let mut reader = DnsMessageReader::new(&data);

        let bytes = reader.read_bytes(0).unwrap();
        assert_eq!(bytes.len(), 0);
        assert_eq!(reader.position(), 0); // Position should not change
    }

    #[test]
    fn test_read_qname_non_ascii_roundtrip() {
        use super::DnsMessageReader;
        // Wire format name with a non-ASCII label; [0x80, 0xFF, 0x00] . "com"
        let wire = wire_name(&[&[0x80, 0xFF, 0x00], b"com"]);

        let mut reader = DnsMessageReader::new(&wire);
        let name = reader.read_qname().unwrap();

        // The non-ASCII bytes should be escaped
        assert_eq!(name.as_str(), "\\128\\255\\000.com");

        // Write it back and verify identical wire bytes
        let mut writer = DnsMessageWriter::new();
        writer.write_qname_uncompressed(&name).unwrap();
        let written = writer.into_bytes();
        assert_eq!(&wire[..], &written[..]);
    }

    #[test]
    fn test_read_qname_uncompressed_non_ascii_roundtrip() {
        use super::DnsMessageReader;
        // Label with dot-byte (0x2E) and backslash-byte (0x5C) in it
        let wire = wire_name(&[&[0x2E, 0x5C]]);

        let mut reader = DnsMessageReader::new(&wire);
        let name = reader.read_qname_uncompressed(wire.len()).unwrap();

        // Both should be escaped
        assert_eq!(name.as_str(), "\\046\\092");

        let mut writer = DnsMessageWriter::new();
        writer.write_qname_uncompressed(&name).unwrap();
        let written = writer.into_bytes();
        assert_eq!(&wire[..], &written[..]);
    }

    #[test]
    fn test_multiple_labels_in_qname() {
        use super::DnsMessageReader;
        let data = wire_name(&[b"mail", b"example", b"com"]);
        let mut reader = DnsMessageReader::new(&data);

        let name = reader.read_qname().unwrap();
        assert_eq!(name.as_str(), "mail.example.com");
    }
}
