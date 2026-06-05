use uuid::Uuid;

pub fn offline_uuid(username: &str) -> Uuid {
    use md5::{Digest, Md5};
    let input = format!("OfflinePlayer:{}", username);
    let mut bytes: [u8; 16] = Md5::digest(input.as_bytes()).into();
    bytes[6] = (bytes[6] & 0x0F) | 0x30;
    bytes[8] = (bytes[8] & 0x3F) | 0x80;
    Uuid::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_uuid_is_version_3() {
        let u = offline_uuid("TestPlayer");
        assert_eq!(u.get_version_num(), 3);
    }

    #[test]
    fn offline_uuid_deterministic() {
        assert_eq!(offline_uuid("Steve"), offline_uuid("Steve"));
        assert_ne!(offline_uuid("Steve"), offline_uuid("Alex"));
    }
}
