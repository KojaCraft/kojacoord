use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const KOJACOORD_CHANNEL: &str = "kojacoord:send";
pub const BUNGEECORD_CHANNEL: &str = "BungeeCord";

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum TransferCommand {
    Connect { server: String },
    ConnectOther { server: String, uuid: Uuid },
    GetServer,
    GetPlayers,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TransferResponse {
    CurrentServer { name: String },
    PlayerList { count: usize, players: Vec<String> },
    Error { message: String },
    Ok,
}

impl TransferResponse {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    pub fn error(msg: impl Into<String>) -> Self {
        TransferResponse::Error {
            message: msg.into(),
        }
    }
}

pub fn parse_command(channel: &str, data: &[u8]) -> Option<TransferCommand> {
    match channel {
        KOJACOORD_CHANNEL => parse_kojacoord(data),
        BUNGEECORD_CHANNEL => parse_bungeecord(data),
        _ => None,
    }
}

fn parse_kojacoord(data: &[u8]) -> Option<TransferCommand> {
    if data.len() > 65_536 {
        tracing::warn!(
            len = data.len(),
            "transfer: oversized kojacoord payload — ignoring"
        );
        return None;
    }
    match serde_json::from_slice::<TransferCommand>(data) {
        Ok(cmd) => {
            tracing::debug!(command = ?cmd, "transfer: parsed kojacoord command");
            Some(cmd)
        },
        Err(e) => {
            tracing::debug!(error = %e, "transfer: failed to parse kojacoord command");
            None
        },
    }
}

fn parse_bungeecord(data: &[u8]) -> Option<TransferCommand> {
    let (sub_cmd, rest) = read_bc_string(data)?;

    tracing::debug!(sub_cmd = sub_cmd, "transfer: parsing BungeeCord command");

    match sub_cmd {
        "Connect" => {
            let (server, _) = read_bc_string(rest)?;
            if server.is_empty() {
                tracing::warn!("transfer: Connect command with empty server name");
                return None;
            }
            tracing::debug!(server = server, "transfer: parsed Connect command");
            Some(TransferCommand::Connect {
                server: server.to_owned(),
            })
        },
        "ConnectOther" => {
            let (server, rest) = read_bc_string(rest)?;
            let (uuid_str, _) = read_bc_string(rest)?;
            if server.is_empty() || uuid_str.is_empty() {
                tracing::warn!("transfer: ConnectOther command with empty server or uuid");
                return None;
            }
            let uuid = Uuid::parse_str(uuid_str)
                .map_err(|e| tracing::debug!(error = %e, "transfer: bad uuid in ConnectOther"))
                .ok()?;
            tracing::debug!(server = server, uuid = %uuid_str, "transfer: parsed ConnectOther command");
            Some(TransferCommand::ConnectOther {
                server: server.to_owned(),
                uuid,
            })
        },
        "GetServer" => {
            tracing::debug!("transfer: parsed GetServer command");
            Some(TransferCommand::GetServer)
        },
        "GetPlayers" => {
            tracing::debug!("transfer: parsed GetPlayers command");
            Some(TransferCommand::GetPlayers)
        },
        other => {
            tracing::debug!(sub_cmd = other, "transfer: unknown BungeeCord subcmd");
            None
        },
    }
}

fn read_bc_string(data: &[u8]) -> Option<(&str, &[u8])> {
    if data.len() < 2 {
        return None;
    }
    let len = u16::from_be_bytes([data[0], data[1]]) as usize;
    let data = &data[2..];
    if data.len() < len {
        return None;
    }
    let s = std::str::from_utf8(&data[..len]).ok()?;
    let rest = &data[len..];
    Some((s, rest))
}

pub fn encode_bc_string(s: &str) -> Vec<u8> {
    let bytes = s.as_bytes();
    let len = bytes.len().min(u16::MAX as usize);
    let mut out = Vec::with_capacity(2 + len);
    out.extend_from_slice(&(len as u16).to_be_bytes());
    out.extend_from_slice(&bytes[..len]);
    out
}

pub fn build_bc_get_server_response(server_name: &str) -> Vec<u8> {
    encode_bc_string(server_name)
}

pub fn build_bc_get_players_response(players: &[String]) -> Vec<u8> {
    let joined = players.join("\0");
    encode_bc_string(&joined)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_kojacoord_connect() {
        let payload = br#"{"action":"connect","server":"lobby"}"#;
        let cmd = parse_command(KOJACOORD_CHANNEL, payload).unwrap();
        assert!(matches!(cmd, TransferCommand::Connect { server } if server == "lobby"));
    }

    #[test]
    fn parse_kojacoord_connect_other() {
        let payload = br#"{"action":"connect_other","server":"lobby","uuid":"550e8400-e29b-41d4-a716-446655440000"}"#;
        let cmd = parse_command(KOJACOORD_CHANNEL, payload).unwrap();
        assert!(matches!(cmd, TransferCommand::ConnectOther { server, .. } if server == "lobby"));
    }

    #[test]
    fn parse_kojacoord_get_server() {
        let payload = br#"{"action":"get_server"}"#;
        let cmd = parse_command(KOJACOORD_CHANNEL, payload).unwrap();
        assert!(matches!(cmd, TransferCommand::GetServer));
    }

    #[test]
    fn parse_kojacoord_get_players() {
        let payload = br#"{"action":"get_players"}"#;
        let cmd = parse_command(KOJACOORD_CHANNEL, payload).unwrap();
        assert!(matches!(cmd, TransferCommand::GetPlayers));
    }

    #[test]
    fn kojacoord_oversized_payload_returns_none() {
        let payload = vec![b'x'; 65_537];
        assert!(parse_command(KOJACOORD_CHANNEL, &payload).is_none());
    }

    #[test]
    fn malformed_json_returns_none() {
        assert!(parse_command(KOJACOORD_CHANNEL, b"not json").is_none());
    }

    fn bc_payload(parts: &[&[u8]]) -> Vec<u8> {
        let mut out = Vec::new();
        for part in parts {
            out.extend_from_slice(&(part.len() as u16).to_be_bytes());
            out.extend_from_slice(part);
        }
        out
    }

    #[test]
    fn parse_bungeecord_connect() {
        let data = bc_payload(&[b"Connect", b"lobby"]);
        let cmd = parse_command(BUNGEECORD_CHANNEL, &data).unwrap();
        assert!(matches!(cmd, TransferCommand::Connect { server } if server == "lobby"));
    }

    #[test]
    fn parse_bungeecord_connect_other() {
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let data = bc_payload(&[b"ConnectOther", b"lobby", uuid.as_bytes()]);
        let cmd = parse_command(BUNGEECORD_CHANNEL, &data).unwrap();
        assert!(matches!(
            cmd,
            TransferCommand::ConnectOther { server, .. } if server == "lobby"
        ));
    }

    #[test]
    fn parse_bungeecord_get_server() {
        let data = bc_payload(&[b"GetServer"]);
        let cmd = parse_command(BUNGEECORD_CHANNEL, &data).unwrap();
        assert!(matches!(cmd, TransferCommand::GetServer));
    }

    #[test]
    fn parse_bungeecord_get_players() {
        let data = bc_payload(&[b"GetPlayers"]);
        let cmd = parse_command(BUNGEECORD_CHANNEL, &data).unwrap();
        assert!(matches!(cmd, TransferCommand::GetPlayers));
    }

    #[test]
    fn bungeecord_unknown_subcmd_returns_none() {
        let data = bc_payload(&[b"ForwardToPlayer"]);
        assert!(parse_command(BUNGEECORD_CHANNEL, &data).is_none());
    }

    #[test]
    fn bungeecord_truncated_returns_none() {
        assert!(parse_command(BUNGEECORD_CHANNEL, &[0x00]).is_none());
        assert!(parse_command(BUNGEECORD_CHANNEL, &[]).is_none());
    }

    #[test]
    fn bungeecord_connect_empty_server_returns_none() {
        let data = bc_payload(&[b"Connect", b""]);
        assert!(parse_command(BUNGEECORD_CHANNEL, &data).is_none());
    }

    #[test]
    fn unknown_channel_returns_none() {
        assert!(parse_command("minecraft:brand", b"{}").is_none());
    }

    #[test]
    fn response_current_server_serializes() {
        let r = TransferResponse::CurrentServer {
            name: "lobby".into(),
        };
        let j: serde_json::Value = serde_json::from_slice(&r.to_bytes()).unwrap();
        assert_eq!(j["type"], "current_server");
        assert_eq!(j["name"], "lobby");
    }

    #[test]
    fn response_player_list_serializes() {
        let r = TransferResponse::PlayerList {
            count: 2,
            players: vec!["Alice".into(), "Bob".into()],
        };
        let j: serde_json::Value = serde_json::from_slice(&r.to_bytes()).unwrap();
        assert_eq!(j["type"], "player_list");
        assert_eq!(j["count"], 2);
    }

    #[test]
    fn bc_get_server_response_roundtrip() {
        let payload = build_bc_get_server_response("lobby");
        let (s, _) = super::read_bc_string(&payload).unwrap();
        assert_eq!(s, "lobby");
    }

    #[test]
    fn bc_get_players_response_roundtrip() {
        let players = vec!["Alice".to_owned(), "Bob".to_owned()];
        let payload = build_bc_get_players_response(&players);
        let (s, _) = super::read_bc_string(&payload).unwrap();
        assert_eq!(s, "Alice\0Bob");
    }
}
