/// Extracts the transaction ID from a DNS message.
pub fn extract_transaction_id(data: &[u8]) -> Option<u16> {
    if data.len() < 2 {
        return None;
    }
    Some(u16::from_be_bytes([data[0], data[1]]))
}

/// Check if a dns message has a truncated flag set.
pub fn is_truncated(data: &[u8]) -> Option<bool> {
    if data.len() < 4 {
        return None;
    }
    let flags = u16::from_be_bytes([data[2], data[3]]);
    Some((flags & 0x0200) != 0)
}
