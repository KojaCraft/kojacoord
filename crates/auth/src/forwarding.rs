use crate::session::AuthenticatedProfile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForwardingMode {
    Bungeecord,
    Velocity { secret: String },
    None,
}

pub fn bungeecord_suffix(
    client_ip: &std::net::IpAddr,
    profile: &AuthenticatedProfile,
) -> Result<String, serde_json::Error> {
    let uuid = profile.id.hyphenated().to_string();
    let props = serde_json::to_string(&profile.properties)?;
    Ok(format!("\0{}\0{}\0{}", client_ip, uuid, props))
}

pub fn velocity_header(
    secret: &str,
    client_ip: &std::net::IpAddr,
    profile: &AuthenticatedProfile,
) -> Vec<u8> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let mut payload = Vec::new();

    payload.extend_from_slice(&1i32.to_be_bytes());
    write_utf8_str(&client_ip.to_string(), &mut payload);
    payload.extend_from_slice(profile.id.as_bytes());
    write_utf8_str(&profile.name, &mut payload);
    write_varint(profile.properties.len() as i32, &mut payload);
    for p in &profile.properties {
        write_utf8_str(&p.name, &mut payload);
        write_utf8_str(&p.value, &mut payload);
        if let Some(sig) = &p.signature {
            payload.push(1);
            write_utf8_str(sig, &mut payload);
        } else {
            payload.push(0);
        }
    }

    type HmacSha256 = Hmac<Sha256>;
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(&payload);
    let sig = mac.finalize().into_bytes();

    let mut out = Vec::new();
    write_varint(sig.len() as i32, &mut out);
    out.extend_from_slice(&sig);
    out.extend_from_slice(&payload);
    out
}

fn write_utf8_str(s: &str, out: &mut Vec<u8>) {
    write_varint(s.len() as i32, out);
    out.extend_from_slice(s.as_bytes());
}

fn write_varint(v: i32, out: &mut Vec<u8>) {
    let mut u = v as u32;
    loop {
        let b = (u & 0x7F) as u8;
        u >>= 7;
        if u != 0 {
            out.push(b | 0x80);
        } else {
            out.push(b);
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bungeecord_suffix_format() {
        let profile = AuthenticatedProfile {
            id: uuid::Uuid::nil(),
            name: "TestPlayer".into(),
            properties: vec![],
        };
        let ip: std::net::IpAddr = "127.0.0.1".parse().unwrap();
        let suffix = bungeecord_suffix(&ip, &profile).unwrap();
        assert!(suffix.starts_with('\0'));
        assert!(suffix.contains("TestPlayer") || suffix.contains("00000000"));
    }
}
