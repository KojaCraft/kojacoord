use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::error::AuthError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileProperty {
    pub name: String,
    pub value: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedProfile {
    pub id: Uuid,
    pub name: String,
    pub properties: Vec<ProfileProperty>,
}

pub fn minecraft_hex_digest(
    server_id: &str,
    shared_secret: &[u8],
    public_key_der: &[u8],
) -> String {
    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    hasher.update(server_id.as_bytes());
    hasher.update(shared_secret);
    hasher.update(public_key_der);
    let hash: [u8; 20] = hasher.finalize().into();
    java_hex(&hash)
}

fn java_hex(bytes: &[u8; 20]) -> String {
    let negative = bytes[0] & 0x80 != 0;
    if negative {
        let mut b = *bytes;
        for x in b.iter_mut() {
            *x = !*x;
        }
        let mut carry = true;
        for x in b.iter_mut().rev() {
            let (v, c) = x.overflowing_add(carry as u8);
            *x = v;
            carry = c;
            if !carry {
                break;
            }
        }
        format!("-{}", hex::encode(b).trim_start_matches('0'))
    } else {
        let s = hex::encode(bytes);
        let t = s.trim_start_matches('0');
        if t.is_empty() {
            "0".into()
        } else {
            t.into()
        }
    }
}

pub async fn verify_session(
    http: &reqwest::Client,
    username: &str,
    server_hash: &str,
    client_ip: Option<IpAddr>,
    rate_limiter: Arc<Semaphore>,
    timeout_secs: u64,
) -> Result<AuthenticatedProfile, AuthError> {
    let _permit = rate_limiter
        .try_acquire()
        .map_err(|_| AuthError::RateLimited)?;

    let mut url = format!(
        "https://sessionserver.mojang.com/session/minecraft/hasJoined?username={}&serverId={}",
        percent_encode(username),
        percent_encode(server_hash),
    );
    if let Some(ip) = client_ip {
        url.push_str(&format!("&ip={}", ip));
    }

    let resp = http
        .get(&url)
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .send()
        .await
        .map_err(AuthError::SessionServerUnreachable)?;

    match resp.status().as_u16() {
        200 => {
            #[derive(Deserialize)]
            struct Raw {
                id: String,
                name: String,
                properties: Vec<ProfileProperty>,
            }

            let body = resp
                .bytes()
                .await
                .map_err(AuthError::SessionServerUnreachable)?;

            let raw: Raw = serde_json::from_slice(&body).map_err(AuthError::MalformedProfile)?;

            let id = parse_uuid(&raw.id)
                .map_err(|_| AuthError::EncryptionSetupFailed("invalid UUID in profile".into()))?;

            Ok(AuthenticatedProfile {
                id,
                name: raw.name,
                properties: raw.properties,
            })
        },
        204 => Err(AuthError::SessionServerRejected),
        code => Err(AuthError::SessionServerError(code)),
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, uuid::Error> {
    if s.len() == 32 {
        Uuid::parse_str(&format!(
            "{}-{}-{}-{}-{}",
            &s[0..8],
            &s[8..12],
            &s[12..16],
            &s[16..20],
            &s[20..32]
        ))
    } else {
        Uuid::parse_str(s)
    }
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => out.push(c),
            other => {
                for b in other.to_string().as_bytes() {
                    out.push_str(&format!("%{:02X}", b));
                }
            },
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_digest_positive_zero() {
        let all_zeros = [0u8; 20];
        assert_eq!(java_hex(&all_zeros), "0");
    }

    #[test]
    fn hex_digest_no_leading_zeros() {
        let mut b = [0u8; 20];
        b[19] = 1;
        assert_eq!(java_hex(&b), "1");
    }

    #[test]
    fn hex_digest_negative() {
        let all_ff = [0xFFu8; 20];

        assert_eq!(java_hex(&all_ff), "-1");
    }
}
