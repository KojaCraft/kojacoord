//! Play-phase packet pump.
//!
//! Once `ClientConnection` finishes the login dance, [`PacketRelay::run`]
//! takes over: three tokio tasks share the same underlying sockets,
//! one per direction (client→server, server→client, and a writer
//! that drains an mpsc of injected packets back to the client).
//! Tasks coordinate shutdown via a single `Notify`; the client
//! socket writer is a `tokio::sync::Mutex` shared between all three.
//!
//! On the hot path every S→C packet runs through:
//!   1. TPS tracker (lock-free atomic ring buffer)
//!   2. Per-player metrics atomic counters (handle cached at session
//!      start, no map lookup per packet)
//!   3. Optional cross-version packet converter
//!   4. Plugin packet hooks (`Forward` / `Drop` / `Modify`)
//!   5. Write to client
//!
//! The reverse direction adds a per-connection exploit guard
//! (`ExploitGuard`) on the way in. The `out_task` exists so plugins
//! and converters can inject packets toward the client without owning
//! the writer mutex themselves.

use bytes::BytesMut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use kojacoord_protocol::{codec::Encode, types::VarInt, Decode};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, Notify};

use crate::{
    chat_signing::{determine_signing_mode, strip_chat_signature},
    commands,
    config_synthesis::{build_cfg_packets, determine_synthesis_mode, SynthesisMode},
    cookies_transfers::supports_cookies_transfers,
    error::ConnectionError,
    exploit_guard::{build_kick_message, check_chat_message, KickReason},
    modloader,
    packet_builder::{
        build_brand_packet, build_disconnect_packet, build_plugin_message_packet,
        build_system_message_packet,
    },
    packet_ids::{
        cb_play, cb_plugin_message_id, chat_packet_ids_for, sb_play, sb_plugin_message_id,
    },
    packet_io::{
        encode_packet, is_pre_netty_proto, read_client_packet, read_packet, write_client_packet,
        write_packet,
    },
    plugin_decoder,
    proxy::ProxyState,
    server_selector,
    session::SharedSession,
    transfer,
};

use kojacoord_protocol::ProtocolVersion;

use kojacoord_plugin_system::{PacketData, PacketDirection, PacketHookResult};

pub struct PacketRelay {
    pub client_stream: crate::connection::McStream,
    pub backend_stream: TcpStream,
    pub session: SharedSession,
    pub state: Arc<ProxyState>,
    pub protocol_version: u32,
    pub client_compression_threshold: i32,
    pub backend_compression_threshold: i32,
    pub ml_kind: modloader::ModloaderKind,
    pub backend_protocol: u32,
}

#[allow(clippy::large_enum_variant)]
pub enum RelayExit {
    Disconnected,

    Switch {
        client_stream: crate::connection::McStream,
        target: String,
    },

    /// Backend sent us a play-state Disconnect. We hand the client
    /// stream back to the outer pipeline so it can drop the player
    /// into limbo (or a fallback server) instead of closing the
    /// socket. `reason` is the JSON the backend gave us — surfaced
    /// to the player as a "you were kicked: …" message.
    BackendKicked {
        client_stream: crate::connection::McStream,
        reason: String,
    },
}

/// Tag an error as having originated from the *backend* socket rather
/// than the client's. `connection.rs`'s `client_gone()` check uses
/// this to tell "client hung up, nobody to notify" apart from
/// "backend died, the client is still here and needs a real kick
/// message" — see [`ConnectionError::Backend`].
fn from_backend(e: ConnectionError) -> ConnectionError {
    ConnectionError::Backend(Box::new(e))
}

/// Encode a packet exactly as [`write_client_packet`] would put it on the
/// wire — raw bytes for pre-netty (1.6.x) clients, varint-length-framed and
/// optionally zlib-compressed otherwise — but return the bytes instead of
/// writing them anywhere. Used by the S→C write-batching path in
/// [`PacketRelay::run`] to accumulate several already-encoded frames before
/// a single `write_all`; kept as its own tiny function (rather than
/// reworking `write_client_packet` itself) so every other caller of that
/// function across the codebase is completely unaffected by this change.
fn encode_for_client(raw: &[u8], proto: u32, threshold: i32) -> BytesMut {
    if is_pre_netty_proto(proto) {
        BytesMut::from(raw)
    } else {
        encode_packet(raw, threshold)
    }
}

macro_rules! kick {
    ($cw:expr, $reason:expr, $proto:expr, $thresh:expr) => {{
        let msg = build_kick_message($reason);
        let pkt = build_disconnect_packet(&msg, $proto);
        // Use the proto-aware client writer so pre-netty (1.6.x) clients
        // get a raw-bytes disconnect frame and modern clients get the
        // varint-length-framed form.
        let _ = write_client_packet(&mut *$cw.lock().await, &pkt, $proto, $thresh).await;
        return Err(ConnectionError::Closed);
    }};
}

impl PacketRelay {
    fn process_packet_hooks(
        state: &Arc<ProxyState>,
        protocol_version: u32,
        packet_id: i32,
        direction: PacketDirection,
        data: bytes::Bytes,
        player_uuid: uuid::Uuid,
    ) -> Result<bytes::Bytes, bytes::Bytes> {
        // Fast path: no plugin registered any packet hooks, so don't
        // touch the global plugin lock — one relaxed atomic load and we
        // forward untouched. This keeps the per-packet relay cost flat
        // when hooks aren't in use (the common case).
        if !state.plugin_activity.has_packet_hooks() {
            return Ok(data);
        }

        let packet_data = PacketData {
            protocol_version,
            packet_id,
            direction,
            data: data.clone(),
            player_uuid: Some(player_uuid),
        };

        // Snapshot the matching hooks while holding the manager lock, then
        // RELEASE the lock before executing plugin code. Running hooks while
        // holding the proxy-level `RwLock<PluginManager>` read lock let a slow
        // or blocking plugin stall every connection and block plugin
        // load/unload (write lock) — a proxy freeze.
        let hooks = state
            .plugin_manager
            .read()
            .unwrap_or_else(|e| {
                tracing::error!(
                    "plugin_manager lock poisoned — recovering with potentially corrupt state"
                );
                e.into_inner()
            })
            .snapshot_matching_hooks(&packet_data);
        let hook_result = kojacoord_plugin_system::PluginManager::run_hooks(&hooks, &packet_data);
        match hook_result {
            PacketHookResult::Forward => Ok(data),
            PacketHookResult::Drop => Err(data),
            PacketHookResult::Modify(new_data) => Ok(new_data),
            PacketHookResult::Replace {
                packet_id: new_id,
                data: new_data,
            } => {
                let mut new_packet = BytesMut::new();
                let _ = VarInt(new_id).encode(&mut new_packet);
                new_packet.extend_from_slice(&new_data);
                Ok(new_packet.freeze())
            },
        }
    }

    pub async fn run(mut self) -> Result<RelayExit, ConnectionError> {
        let brand_raw = build_brand_packet(self.ml_kind, self.protocol_version);
        write_packet(
            &mut self.client_stream,
            &brand_raw,
            self.client_compression_threshold,
        )
        .await?;

        let (cr, cw) = tokio::io::split(self.client_stream);
        let mut cr = tokio::io::BufReader::with_capacity(8192, cr);
        let (br, mut bw) = self.backend_stream.into_split();
        let mut br = tokio::io::BufReader::with_capacity(8192, br);

        let cw_master = Arc::new(Mutex::new(cw));
        let cw_s2c = Arc::clone(&cw_master);
        let cw_c2s = Arc::clone(&cw_master);

        let stop = Arc::new(Notify::new());
        let stopped = Arc::new(AtomicBool::new(false));
        let switch_target: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        // Set when the backend sends us a play-state Disconnect — we
        // stash the reason and let the outer loop drop the player into
        // limbo instead of closing the client socket.
        let kick_reason: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let kick_reason_s2c = Arc::clone(&kick_reason);

        let stop_s2c_wait = Arc::clone(&stop);
        let stop_s2c_sig = Arc::clone(&stop);
        let stopped_s2c_chk = Arc::clone(&stopped);
        let stopped_s2c_set = Arc::clone(&stopped);

        let stop_c2s_wait = Arc::clone(&stop);
        let stop_c2s_sig = Arc::clone(&stop);
        let stopped_c2s_chk = Arc::clone(&stopped);
        let stopped_c2s_set = Arc::clone(&stopped);
        let switch_c2s = Arc::clone(&switch_target);

        let player_uuid = self.session.read().await.uuid;
        // TODO(H8): Switch to bounded channel (65536) for backpressure.
        // Requires making all senders async or using try_send with drop semantics.
        let (out_tx, mut out_rx) = tokio::sync::mpsc::unbounded_channel::<bytes::Bytes>();
        // Keep a clone in-scope for converter-driven s2c injection from the c2s
        // task (e.g. synthesizing FinishConfiguration after we swallow a
        // LoginAcknowledged from a 1.20.2+ client).
        let inject_s2c_tx = out_tx.clone();
        self.state.outbound.insert(player_uuid, out_tx);

        // TODO(H8): Switch to bounded channel (65536) for backpressure.
        // Requires making all senders async or using try_send with drop semantics.
        let (backend_out_tx, mut backend_out_rx) =
            tokio::sync::mpsc::unbounded_channel::<bytes::Bytes>();
        self.state
            .backend_outbound
            .insert(player_uuid, backend_out_tx.clone());

        // Pending-purchase delivery used to live here (query the DB 200ms
        // after connect, push a plugin message to the backend). Dropped
        // along with the rest of proxy-owned persistence — a plugin/backend
        // that needs to deliver a purchase can do so over its own storage
        // and the existing plugin-message channel.
        let cw_out = Arc::clone(&cw_master);
        let stop_out_wait = Arc::clone(&stop);
        let stop_out_sig = Arc::clone(&stop);
        let stopped_out_set = Arc::clone(&stopped);
        let stopped_out_chk = Arc::clone(&stopped);

        let cr_mut = &mut cr;
        let br_mut = &mut br;
        let bw_mut = &mut bw;

        let proto = self.protocol_version;
        let client_thresh = self.client_compression_threshold;
        let backend_thresh = self.backend_compression_threshold;
        // Cross-version packet conversion has been removed: the proxy now
        // forwards play packets verbatim and relies on ViaVersion (on the
        // backend) for any client↔backend protocol bridging. `backend_proto`
        // is still used for chat-signing and config-synthesis decisions below.
        let backend_proto = self.backend_protocol;

        // Determine chat signing mode if enabled
        let chat_signing_mode = if self.state.config.proxy.chat_signing_translation {
            Some(determine_signing_mode(proto, backend_proto))
        } else {
            None
        };

        // Determine config synthesis mode
        let synthesis_mode = determine_synthesis_mode(proto, backend_proto);

        // Check if cookies/transfers are supported
        let supports_cookies = supports_cookies_transfers(proto);

        let cb_pm_id = cb_plugin_message_id(proto);
        let cb_disc_id = cb_play(proto, "ClientboundDisconnect");
        let sb_pm_id = sb_plugin_message_id(proto);
        let sb_chat_ids = chat_packet_ids_for(proto);
        // The modern `ServerboundChatCommand` packet carries the command text
        // WITHOUT the leading slash, so we can't detect commands by sniffing for
        // '/'. Track its id explicitly so we can route command packets to the
        // command handler (and forward unhandled ones to the backend) instead of
        // mistaking them for chat.
        let sb_chat_cmd_id = sb_play(proto, "ServerboundChatCommand");

        let cb_chunk_id = cb_play(proto, "ClientboundLevelChunkWithLight");
        // Look these up via the central registry so adding a new
        // protocol version (1.21.6+, …) doesn't require editing this
        // hot path.
        let sb_move_pos_id = sb_play(proto, "ServerboundMovePlayerPos");
        let sb_move_pos_rot_id = sb_play(proto, "ServerboundMovePlayerPosRot");

        let state_s2c = Arc::clone(&self.state);
        let state_c2s = Arc::clone(&self.state);
        let session_s2c = self.session.clone();
        let session_c2s = self.session.clone();

        // Lock-free per-player metrics handles. Grabbed once here; the
        // loops below just do `fetch_add` per packet, no locks taken on
        // the hot path. `None` means the player wasn't pre-registered
        // (shouldn't happen, but we skip silently if so).
        let metrics_handle = self.state.player_metrics.get(&player_uuid);
        let metrics_s2c = metrics_handle.clone();
        let metrics_c2s = metrics_handle.clone();
        // Microsecond clock shared by both directions; cheaper than
        // re-reading `Instant::now()` for every packet — we recompute
        // it once per loop iteration.
        let metrics_epoch = std::time::Instant::now();

        let s2c = async move {
            let result: Result<(), ConnectionError> = async move {
            // S→C write batching (see relay.rs module docs / the
            // `batch_s2c_writes` config comment): when several backend
            // packets already arrived together in `br`'s read buffer (a
            // chunk-loading burst, a teleport), accumulate their encoded
            // frames here and flush once instead of once per packet.
            // `PENDING_FLUSH_CAP` is a defensive cap so a change to
            // `BufReader`'s capacity elsewhere can't make this grow
            // unboundedly; in practice `br_mut.buffer().is_empty()` (no
            // more already-buffered backend bytes) is what triggers most
            // flushes, so a lone packet still goes out immediately — this
            // never adds latency versus today's per-packet flush.
            let batch_writes = state_s2c.config.proxy.batch_s2c_writes;
            let mut pending_s2c = BytesMut::new();
            const PENDING_FLUSH_CAP: usize = 32 * 1024;

            let loop_result: Result<(), ConnectionError> = loop {
                if stopped_s2c_chk.load(Ordering::Acquire) {
                    break Ok(());
                }
                let payload = tokio::select! {
                    biased;
                    _ = stop_s2c_wait.notified() => break Ok(()),
                    _ = state_s2c.shutdown_notify.notified() => {
                        // Flush anything already queued so it reaches the
                        // client before the shutdown Disconnect, preserving
                        // wire order.
                        if !pending_s2c.is_empty() {
                            let mut cw = cw_s2c.lock().await;
                            let _ = cw.write_all(&pending_s2c).await;
                            pending_s2c.clear();
                        }
                        // Proxy is shutting down — send the configured
                        // Disconnect packet to the client before we
                        // drop the socket. Without this the client
                        // sees "End of stream".
                        let reason = state_s2c.shutdown_reason.load().as_ref().clone();
                        let raw = crate::packet_builder::build_disconnect_packet(&reason, proto);
                        let mut cw = cw_s2c.lock().await;
                        let _ = write_client_packet(&mut *cw, &raw, proto, client_thresh).await;
                        break Ok(());
                    },
                    r = read_packet(&mut *br_mut, backend_thresh) => r.map_err(from_backend)?,
                };

                state_s2c.metrics.record_packet(payload.len());

                state_s2c.tps_tracker.record_packet();

                if let Some(m) = &metrics_s2c {
                    let now_micros = metrics_epoch.elapsed().as_micros() as u64;
                    crate::metrics_player::PlayerMetricsRegistry::record_sent(
                        m,
                        payload.len(),
                        now_micros,
                    );
                }

                let mut cur = payload.clone();

                let pkt_id = match VarInt::decode(&mut cur) {
                    Ok(v) => {
                        if v.0 < 0 || v.0 > 255 {
                            tracing::warn!(raw_id = v.0, "S→C packet ID out of u8 range — treating as passthrough");
                            // Cold path, bypasses the batching accumulator —
                            // flush it first so wire order still matches.
                            if !pending_s2c.is_empty() {
                                let mut cw = cw_s2c.lock().await;
                                cw.write_all(&pending_s2c).await?;
                                pending_s2c.clear();
                            }
                            let mut cw = cw_s2c.lock().await;
                            write_client_packet(&mut *cw, &payload, proto, client_thresh).await?;
                            continue;
                        }
                        v.0 as u8
                    }
                    Err(_) => {
                        tracing::warn!("S→C failed to decode packet ID — dropping malformed packet");
                        continue;
                    }
                };

                tracing::trace!(
                    direction = "S→C",
                    packet_id = pkt_id,
                    protocol = proto,
                    "packet"
                );

                if pkt_id == cb_chunk_id {
                    if batch_writes {
                        let frame = encode_for_client(&payload, proto, client_thresh);
                        pending_s2c.extend_from_slice(&frame);
                        if pending_s2c.len() >= PENDING_FLUSH_CAP || br_mut.buffer().is_empty() {
                            let mut cw = cw_s2c.lock().await;
                            cw.write_all(&pending_s2c).await?;
                            pending_s2c.clear();
                        }
                    } else {
                        let mut cw = cw_s2c.lock().await;
                        write_client_packet(&mut *cw, &payload, proto, client_thresh).await?;
                    }
                    continue;
                }

                if pkt_id == cb_pm_id {
                    let mut body = payload.clone();
                    let _ = VarInt::decode(&mut body);
                    if let Ok(msg) =
                        plugin_decoder::decode_clientbound_plugin_message(body, proto)
                    {
                        if msg.channel == "minecraft:brand" || msg.channel == "MC|Brand" {
                            tracing::debug!("suppressed backend brand");
                            continue;
                        }
                        if let Some(cmd) = transfer::parse_command(&msg.channel, &msg.data) {
                            if let Some(resp) =
                                handle_transfer_command(cmd, &session_s2c, &state_s2c).await
                            {
                                let pkt_raw = build_plugin_message_packet(
                                    transfer::KOJACOORD_CHANNEL,
                                    &resp,
                                    proto,
                                );
                                // A different packet is about to go out on
                                // this connection — flush anything already
                                // queued first so wire order still matches
                                // logical order.
                                if !pending_s2c.is_empty() {
                                    let mut cw = cw_s2c.lock().await;
                                    cw.write_all(&pending_s2c).await?;
                                    pending_s2c.clear();
                                }
                                write_client_packet(&mut *cw_s2c.lock().await, &pkt_raw, proto, client_thresh).await?;
                            }
                            continue;
                        }
                        if modloader::is_fml1_play_channel(&msg.channel) {
                            modloader::log_fml1_packet(&msg.channel, &msg.data, "S→C", proto);
                        }
                    }
                }

                if pkt_id == cb_disc_id {
                    // Backend kicked the player. Don't forward the
                    // disconnect — stash the reason, signal the relay
                    // to wind down cleanly, and let the outer pipeline
                    // hand the client off to limbo so they don't get
                    // dropped from the proxy.
                    let mut reason_cursor = payload.clone();
                    let _ = VarInt::decode(&mut reason_cursor); // skip the packet id
                    let reason = String::decode(&mut reason_cursor)
                        .unwrap_or_else(|_| "Backend disconnected".to_string());
                    tracing::info!(
                        reason = %reason,
                        "backend sent Disconnect — handing player to limbo"
                    );
                    // Flush anything already queued before we stop writing —
                    // the outer pipeline hands the client to limbo next.
                    if !pending_s2c.is_empty() {
                        let mut cw = cw_s2c.lock().await;
                        let _ = cw.write_all(&pending_s2c).await;
                        pending_s2c.clear();
                    }
                    *kick_reason_s2c.lock().await = Some(reason);
                    // Mirror what the outer post-block does — the
                    // outer scope still owns its own clones, so this
                    // just speeds up the stop signal.
                    break Ok(());
                }

                // Verbatim forward (no cross-version conversion). Plugin
                // packet hooks still run; ViaVersion on the backend handles
                // any protocol bridging.
                match Self::process_packet_hooks(
                    &state_s2c,
                    proto,
                    pkt_id as i32,
                    PacketDirection::Clientbound,
                    payload.clone(),
                    player_uuid,
                ) {
                    Ok(data) => {
                        if batch_writes {
                            let frame = encode_for_client(&data, proto, client_thresh);
                            pending_s2c.extend_from_slice(&frame);
                            if pending_s2c.len() >= PENDING_FLUSH_CAP || br_mut.buffer().is_empty() {
                                let mut cw = cw_s2c.lock().await;
                                cw.write_all(&pending_s2c).await?;
                                pending_s2c.clear();
                            }
                        } else {
                            let mut cw = cw_s2c.lock().await;
                            write_client_packet(&mut *cw, &data, proto, client_thresh).await?;
                        }
                    }
                    Err(_) => {
                        tracing::trace!(pkt_id, "S→C dropped by plugin hook");
                    }
                }
            };

            // Only reached on a clean `break Ok(())` above — any `?`
            // propagation short-circuits this whole `async move` block
            // before we get here, so there's no risk of flushing into an
            // already-broken write half.
            if !pending_s2c.is_empty() {
                let mut cw = cw_s2c.lock().await;
                cw.write_all(&pending_s2c).await?;
            }

            loop_result
            }.await;

            stopped_s2c_set.store(true, Ordering::Release);
            stop_s2c_sig.notify_waiters();
            result
        };

        let c2s = async move {
            let result: Result<(), ConnectionError> = async move {
                loop {
                    if stopped_c2s_chk.load(Ordering::Acquire) {
                        return Ok(());
                    }
                    let payload = tokio::select! {
                        biased;
                        _ = stop_c2s_wait.notified() => return Ok(()),
                        _ = state_c2s.shutdown_notify.notified() => {
                            // Proxy is shutting down — s2c loop is
                            // already writing the Disconnect packet,
                            // so we just stop reading from the client
                            // and unwind. Returning here releases our
                            // half of the read/write split and lets
                            // the s2c task drop the socket cleanly.
                            return Ok(());
                        },
                        backend_pkt = backend_out_rx.recv() => {
                            match backend_pkt {
                                Some(raw) => {
                                    write_packet(&mut *bw_mut, &raw, backend_thresh).await.map_err(from_backend)?;
                                    continue;
                                }
                                None => return Ok(()),
                            }
                        }
                        r = read_client_packet(&mut *cr_mut, proto, client_thresh) => r?,
                    };

                    // Inbound abuse guard removed — the backend Minecraft server
                    // already enforces packet-rate/size limits.

                    if let Some(m) = &metrics_c2s {
                        let now_micros = metrics_epoch.elapsed().as_micros() as u64;
                        crate::metrics_player::PlayerMetricsRegistry::record_received(
                            m,
                            payload.len(),
                            now_micros,
                        );
                    }

                    let mut cur = payload.clone();

                    let pkt_id = match VarInt::decode(&mut cur) {
                        Ok(v) => {
                            if v.0 < 0 || v.0 > 255 {
                                tracing::warn!(raw_id = v.0, "C→S packet ID out of u8 range — treating as passthrough");
                                write_packet(&mut *bw_mut, &payload, backend_thresh).await.map_err(from_backend)?;
                                continue;
                            }
                            v.0 as u8
                        }
                        Err(_) => {
                            tracing::warn!("exploit_guard: failed to decode packet id — kicking");
                            kick!(
                                cw_c2s,
                                crate::exploit_guard::KickReason::MalformedPacket,
                                proto,
                                client_thresh
                            );
                        },
                    };

                    tracing::trace!(
                        direction = "C→S",
                        packet_id = pkt_id,
                        protocol = proto,
                        "packet"
                    );

                    state_c2s.metrics.record_packet(payload.len());

                    // `player_uuid` is captured once at relay start and never
                    // changes for the life of the connection. Re-reading it from
                    // the session RwLock on every C→S packet was a needless
                    // per-packet lock acquisition on the hot path.
                    let uuid = player_uuid;

                    // Dispatch movement events to plugin system (anticheat, etc.).
                    // Movement is the highest-frequency C→S packet, so gate the
                    // whole decode + plugin fan-out behind a lock-free atomic: if
                    // no loaded plugin subscribes to PlayerMove we skip it
                    // entirely and never touch the global plugin lock. This is the
                    // fix for the movement rubber-banding under load — previously
                    // every move packet from every player contended on that lock.
                    if (pkt_id == sb_move_pos_rot_id || pkt_id == sb_move_pos_id)
                        && state_c2s
                            .plugin_activity
                            .subscribes(kojacoord_plugin_system::PluginEventKind::PlayerMove)
                    {
                        let mut body = payload.clone();
                        let _ = VarInt::decode(&mut body);
                        if let (Ok(x), Ok(y), Ok(z)) = (
                            f64::decode(&mut body),
                            f64::decode(&mut body),
                            f64::decode(&mut body),
                        ) {
                            if pkt_id == sb_move_pos_rot_id {
                                let _ = f32::decode(&mut body);
                                let _ = f32::decode(&mut body);
                            }
                            let on_ground = bool::decode(&mut body).unwrap_or(false);
                            let responses = state_c2s
                                .plugin_manager
                                .read()
                                .unwrap_or_else(|e| {
                                    tracing::error!("plugin_manager lock poisoned — recovering with potentially corrupt state");
                                    e.into_inner()
                                })
                                .broadcast_event(
                                &kojacoord_plugin_system::PluginEvent::PlayerMove {
                                    uuid,
                                    x,
                                    y,
                                    z,
                                    on_ground,
                                },
                            );
                            for resp in responses {
                                if let kojacoord_plugin_system::PluginResponse::KickPlayer { uuid: kicked_uuid, reason } = resp {
                                    if kicked_uuid == uuid {
                                        kick!(
                                            cw_c2s,
                                            KickReason::Custom(
                                                "Anticheat Violation".to_string(),
                                                reason,
                                            ),
                                            proto,
                                            client_thresh
                                        );
                                    }
                                }
                            }
                        }
                    }

                    if pkt_id == sb_pm_id {
                        let mut body = payload.clone();
                        let _ = VarInt::decode(&mut body);
                        if let Ok(msg) =
                            plugin_decoder::decode_serverbound_plugin_message(body, proto)
                        {
                            if modloader::is_fml1_play_channel(&msg.channel) {
                                modloader::log_fml1_packet(&msg.channel, &msg.data, "C→S", proto);
                            }

                            // Cookies & Transfers passthrough handling
                            if supports_cookies && state_c2s.config.proxy.cookies_transfers_passthrough
                                && (msg.channel == "minecraft:cookie_response" || msg.channel == "minecraft:transfer") {
                                    tracing::trace!(channel = %msg.channel, "Passthrough: relaying cookie/transfer packet");
                                    // Store cookie data in session if needed
                                    let mut session = session_c2s.write().await;
                                    if msg.channel == "minecraft:cookie_response" {
                                        session.cookies.store("default".to_string(), msg.data.clone());
                                    }
                                    drop(session);
                                }

                            if server_selector::is_serverlist_channel(&msg.channel) {
                                let payload =
                                    server_selector::build_serverlist_payload(&state_c2s, player_uuid).await;

                                let pkt_raw =
                                    build_plugin_message_packet(&msg.channel, &payload, proto);
                                write_client_packet(
                                    &mut *cw_c2s.lock().await,
                                    &pkt_raw,
                                    proto,
                                    client_thresh,
                                )
                                .await?;
                                tracing::debug!(
                                    channel = %msg.channel,
                                    "server-selector: answered server-list request"
                                );
                                continue;
                            }
                            if server_selector::is_connect_channel(&msg.channel) {
                                if let Some(server) =
                                    server_selector::parse_connect_payload(&msg.data)
                                {
                                    if request_switch(
                                        &server,
                                        &state_c2s,
                                        &switch_c2s,
                                        &cw_c2s,
                                        proto,
                                        client_thresh,
                                    )
                                    .await?
                                    {
                                        return Ok(());
                                    }
                                } else {
                                    tracing::warn!(
                                        channel = %msg.channel,
                                        "server-selector: ignoring connect with empty server name"
                                    );
                                }
                                continue;
                            }
                            if server_selector::is_modpack_channel(&msg.channel) {
                                tracing::debug!(
                                    channel = %msg.channel,
                                    bytes = msg.data.len(),
                                    "server-selector: received modpack info"
                                );
                                // Hand the client's modpack report to plugins so
                                // the orchestrator plugin can cache it per-player
                                // and accept/reject backend connections against
                                // each server's required modpack. Delivered as a
                                // generic plugin message; the orchestrator plugin
                                // parses the `kojacoord:modpack` payload.
                                state_c2s
                                    .plugin_manager
                                    .read()
                                    .unwrap_or_else(|e| {
                                        tracing::error!("plugin_manager lock poisoned — recovering with potentially corrupt state");
                                        e.into_inner()
                                    })
                                    .broadcast_event(
                                        &kojacoord_plugin_system::PluginEvent::PluginMessage {
                                            uuid,
                                            channel: msg.channel.clone(),
                                            data: msg.data.clone(),
                                        },
                                    );
                                continue;
                            }

                            if let Some(cmd) = transfer::parse_command(&msg.channel, &msg.data) {
                                if let transfer::TransferCommand::Connect { server } = &cmd {
                                    if request_switch(
                                        server,
                                        &state_c2s,
                                        &switch_c2s,
                                        &cw_c2s,
                                        proto,
                                        client_thresh,
                                    )
                                    .await?
                                    {
                                        return Ok(());
                                    }
                                    continue;
                                }
                                if let Some(resp) =
                                    handle_transfer_command(cmd, &session_c2s, &state_c2s).await
                                {
                                    let pkt_raw = build_plugin_message_packet(
                                        transfer::KOJACOORD_CHANNEL,
                                        &resp,
                                        proto,
                                    );
                                    write_packet(&mut *bw_mut, &pkt_raw, backend_thresh).await.map_err(from_backend)?;
                                }
                                continue;
                            }
                        }
                    }

                    // Chat signing translation: strip signatures if needed
                    let mut modified_payload = payload.clone();
                    if sb_chat_ids.contains(&pkt_id) {
                        let mut body = payload.clone();
                        let _ = VarInt::decode(&mut body);
                        if let Ok(text) = String::decode(&mut body) {
                            // A `ServerboundChatCommand` packet IS a command even
                            // though its text has no leading slash; chat-message
                            // packets are commands only when the player typed '/'.
                            let is_command_packet =
                                sb_chat_cmd_id != 0xFF && pkt_id == sb_chat_cmd_id;
                            let treat_as_command = is_command_packet || text.starts_with('/');
                            // Normalised command line (always slash-prefixed) for
                            // the proxy command handler.
                            let command_line = if treat_as_command && !text.starts_with('/') {
                                format!("/{}", text)
                            } else {
                                text.clone()
                            };

                            if let Err(reason) = check_chat_message(&text) {
                                let username = session_c2s.read().await.username.clone();
                                tracing::warn!(
                                    username = %username,
                                    "exploit_guard: illegal chat — kicking"
                                );
                                kick!(cw_c2s, reason, proto, client_thresh);
                            }

                            if let Some(mode) = chat_signing_mode {
                                use crate::chat_signing::ChatSigningMode;
                                if mode == ChatSigningMode::Unsigned {
                                    match strip_chat_signature(&payload, proto) {
                                        Ok(stripped) => {
                                            modified_payload = stripped.into();
                                            tracing::trace!("Stripped chat signature for unsigned mode");
                                        },
                                        Err(e) => {
                                            tracing::warn!(error = %e, "Failed to strip chat signature, using original");
                                        },
                                    }
                                }
                            }

                            if treat_as_command {
                                let mut messages: Vec<String> = Vec::new();
                                let result = commands::handle_command(
                                    &command_line,
                                    session_c2s.clone(),
                                    Arc::clone(&state_c2s),
                                    &mut |msg| messages.push(msg),
                                )
                                .await;

                                if !messages.is_empty() {
                                    let mut cw = cw_c2s.lock().await;
                                    for msg in &messages {
                                        let encoded_raw = build_system_message_packet(msg, proto);
                                        if let Err(e) = write_client_packet(
                                            &mut *cw,
                                            &encoded_raw,
                                            proto,
                                            client_thresh,
                                        )
                                                .await
                                        {
                                            tracing::warn!(
                                                error = %e,
                                                "failed to send command response"
                                            );
                                        }
                                    }
                                }

                                // Proxy handled it — swallow. Otherwise fall
                                // through so the original command packet reaches
                                // the backend for server-side execution.
                                match result {
                                    // A `/server`/`/hub` switch request: perform
                                    // the live switch via the same path the
                                    // server-selector connect channel uses.
                                    commands::CommandResult::Switch(target) => {
                                        if request_switch(
                                            &target,
                                            &state_c2s,
                                            &switch_c2s,
                                            &cw_c2s,
                                            proto,
                                            client_thresh,
                                        )
                                        .await?
                                        {
                                            return Ok(());
                                        }
                                        continue;
                                    },
                                    commands::CommandResult::Handled => continue,
                                    // NotACommand / Error: fall through to backend.
                                    _ => {},
                                }
                            } else {
                                // Plain chat. Hand to plugins (a plugin may kick),
                                // then broadcast across the network.
                                if state_c2s
                                    .dispatch_plugin_event(kojacoord_plugin_system::PluginEvent::PlayerChat {
                                        uuid,
                                        message: text.clone(),
                                    })
                                    .await
                                {
                                    return Ok(());
                                }
                                // Rank-prefixed formatting used to come from the
                                // (now-removed) role registry; plain "name: text" is
                                // all the proxy itself knows how to render.
                                let name = session_c2s.read().await.username.clone();
                                let line = format!("{}: {}", name, text);
                                state_c2s.broadcast_system_message(&line).await;
                                continue;
                            }
                        }
                    }

                    // Verbatim forward (no cross-version conversion). ViaVersion
                    // on the backend handles any protocol bridging.
                    // Config synthesis: inject RegistryData (766+) + FinishConfiguration
                    if synthesis_mode == SynthesisMode::ClientSide {
                        let canonical = ProtocolVersion::from_id(proto);
                        if pkt_id == 0x03 && canonical.has_configuration_phase() {
                            if let Ok(cfg_packets) = build_cfg_packets(proto) {
                                for pkt in cfg_packets {
                                    tracing::trace!("Injecting synthetic config-phase packet ({} bytes)", pkt.len());
                                    let _ = inject_s2c_tx.send(pkt.into());
                                }
                            }
                        }
                    }

                    // Use modified payload if signature was stripped
                    let payload_to_send = if modified_payload != payload {
                        modified_payload.clone()
                    } else {
                        payload.clone()
                    };
                    write_packet(&mut *bw_mut, &payload_to_send, backend_thresh).await.map_err(from_backend)?;
                }

                #[allow(unreachable_code)]
                Ok::<(), ConnectionError>(())
            }
            .await;

            stopped_c2s_set.store(true, Ordering::Release);
            stop_c2s_sig.notify_waiters();
            result
        };

        let out_task = async move {
            loop {
                if stopped_out_chk.load(Ordering::Acquire) {
                    break;
                }
                tokio::select! {
                    biased;
                    _ = stop_out_wait.notified() => break,
                    msg = out_rx.recv() => match msg {
                        Some(raw) => {
                            if write_client_packet(&mut *cw_out.lock().await, &raw, proto, client_thresh).await.is_err() {
                                break;
                            }
                        }
                        None => break,
                    },
                }
            }
            stopped_out_set.store(true, Ordering::Release);
            stop_out_sig.notify_waiters();
        };

        let (c2s_res, s2c_res, _) = tokio::join!(c2s, s2c, out_task);

        self.state.outbound.remove(&player_uuid);
        self.state.backend_outbound.remove(&player_uuid);

        let transferred = self.session.read().await.transferred;
        if let Some(srv_name) = self.session.read().await.current_server.clone() {
            if !transferred {
                if let Some(srv) = self.state.server_registry.get(&srv_name) {
                    srv.player_count.fetch_sub(1, Ordering::Relaxed);
                }
            }
        }
        self.session.write().await.current_server = None;

        let target = switch_target.lock().await.take();
        if let (Some(target), Ok(())) = (target, &c2s_res) {
            match Arc::try_unwrap(cw_master) {
                Ok(mutex) => {
                    let cw = mutex.into_inner();
                    let client_stream = cr.into_inner().unsplit(cw);
                    tracing::info!(target = %target, "relay: performing live server switch");
                    return Ok(RelayExit::Switch {
                        client_stream,
                        target,
                    });
                },
                Err(_) => {
                    tracing::error!("relay: could not reunite client stream for switch");
                    return Err(ConnectionError::Closed);
                },
            }
        }

        // Did the backend kick us mid-play? Hand the stream back to
        // the outer pipeline so it can drop the player into limbo.
        // The s2c task that detected the kick also set the stop
        // signal, so by here c2s has wound down naturally.
        let kick = kick_reason.lock().await.take();
        if let Some(reason) = kick {
            match Arc::try_unwrap(cw_master) {
                Ok(mutex) => {
                    let cw = mutex.into_inner();
                    let client_stream = cr.into_inner().unsplit(cw);
                    return Ok(RelayExit::BackendKicked {
                        client_stream,
                        reason,
                    });
                },
                Err(_) => {
                    tracing::error!("relay: could not reunite client stream after backend kick");
                    return Err(ConnectionError::Closed);
                },
            }
        }

        c2s_res?;
        s2c_res?;
        Ok(RelayExit::Disconnected)
    }
}

async fn request_switch<W>(
    server: &str,
    state: &Arc<ProxyState>,
    switch_target: &Mutex<Option<String>>,
    cw: &Mutex<W>,
    proto: u32,
    client_thresh: i32,
) -> Result<bool, ConnectionError>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    // `server` is client-controlled (server-selector / transfer plugin
    // message payload) and gets embedded in this message below — build
    // it with `serde_json` rather than hand-rolled string formatting so
    // a server name containing `"` or `\` can't break out of the chat
    // JSON and corrupt or inject into the rendered component.
    let reject = |reason: &str| serde_json::json!({ "text": reason, "color": "red" }).to_string();

    let message = match state.server_registry.get(server) {
        Some(b) if b.is_online() => {
            *switch_target.lock().await = Some(server.to_owned());
            tracing::info!(server = %server, "relay: live switch requested");
            return Ok(true);
        },
        Some(_) => reject(&format!("Server '{}' is currently offline.", server)),
        None => reject(&format!("Unknown server '{}'.", server)),
    };

    let raw = build_system_message_packet(&message, proto);
    write_client_packet(&mut *cw.lock().await, &raw, proto, client_thresh).await?;
    Ok(false)
}

async fn handle_transfer_command(
    cmd: transfer::TransferCommand,
    session: &SharedSession,
    state: &Arc<ProxyState>,
) -> Option<Vec<u8>> {
    match cmd {
        transfer::TransferCommand::Connect { server } => {
            match state.server_registry.get(&server) {
                Some(backend) => {
                    if let Some(old_name) = session.read().await.current_server.clone() {
                        if let Some(old) = state.server_registry.get(&old_name) {
                            old.player_count.fetch_sub(1, Ordering::Relaxed);
                        }
                    }
                    backend.player_count.fetch_add(1, Ordering::Relaxed);
                    session.write().await.current_server = Some(server.clone());
                    session.write().await.transferred = true;
                    tracing::info!(server = %server, "relay: transfer requested");
                },
                None => tracing::warn!(%server, "relay: connect to unknown server ignored"),
            }
            None
        },
        transfer::TransferCommand::ConnectOther { server, uuid } => {
            if let Some(target) = state.sessions.get(&uuid) {
                if let Some(old_name) = target.read().await.current_server.clone() {
                    if let Some(old) = state.server_registry.get(&old_name) {
                        old.player_count.fetch_sub(1, Ordering::Relaxed);
                    }
                }
                if let Some(new_srv) = state.server_registry.get(&server) {
                    new_srv.player_count.fetch_add(1, Ordering::Relaxed);
                }
                target.write().await.current_server = Some(server.clone());
                target.write().await.transferred = true;
                tracing::info!(%uuid, %server, "relay: ConnectOther transferred");
            } else {
                tracing::warn!(%uuid, %server, "relay: ConnectOther player not found");
            }
            None
        },
        transfer::TransferCommand::GetServer => {
            let name = session
                .read()
                .await
                .current_server
                .clone()
                .unwrap_or_else(|| "unknown".to_owned());
            serde_json::to_vec(&transfer::TransferResponse::CurrentServer { name }).ok()
        },
        transfer::TransferCommand::GetPlayers => {
            let players: Vec<String> = {
                state
                    .sessions
                    .iter()
                    .filter_map(|entry| entry.value().try_read().ok().map(|g| g.username.clone()))
                    .filter(|s| !s.is_empty())
                    .collect()
            };
            serde_json::to_vec(&transfer::TransferResponse::PlayerList {
                count: players.len(),
                players,
            })
            .ok()
        },
    }
}
