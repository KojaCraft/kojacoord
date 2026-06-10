# Kojacoord Roadmap

This file is the single source of truth for the public roadmap shown on
[kojacoord.net](https://kojacoord.net). The landing page reads it directly from
the default branch, so editing it here updates the site.

**Format:** each `##` heading is a phase. Each list item is a roadmap entry —
use `- [x]` for shipped, `- [ ]` for not-yet-done. Optionally add `— short note`
after the text for extra context.

## Shipped
- [x] Multi-version protocol support (1.8 → latest)
- [x] Authentication pipeline (online + offline) — Mojang session auth
- [x] Anti-cheat engine at the proxy edge
- [x] Native plugin system + `cargo-kpl` plugin builder
- [x] crates.io publishing with docs.rs documentation
- [x] Signed, multi-platform releases — cosign + SHA256SUMS
- [x] Anonymous, opt-out telemetry
- [x] Cross-platform `.kpl` packaging — bundle Windows/Linux/macOS plugin libs
- [x] Plugin signing tied to the integrity allowlist
- [x] Unified plugin API surface across crates
- [x] Public global metrics dashboard
- [x] Cluster mode with autoscaling
- [x] Per-player and per-region routing rules — glob-matched names + IPv4/IPv6 CIDR client ranges
- [x] Hot-reload of plugins without restart — polling mtime watcher, opt-in via `plugins.hot_reload`
- [x] Legacy 0xFE server-list ping — let pre-1.7 / 1.6.x clients see the MOTD
- [x] Block-state ↔ legacy-id flattening table — expanded to 300+ entries covering common blocks
- [x] Velocity-style modern forwarding — HMAC, in addition to BungeeCord and IP-Forward
- [x] Connection throttling per IP and per ASN — token-bucket with temp-ban, ASN lookup placeholder
- [x] Plugin event-bus priorities + cancellation propagation — priority-based execution, Cancel response
- [x] Live config reload (SIGHUP / inotify) — routing rules, server entries, motd without restart
- [x] Per-server compression threshold + cipher pinning — per-server override, TLS cipher suite pinning
- [x] Plugin permissions / capability sandboxing — declared in config, enforced at load
- [x] Player-list / sample customisation via plugin hook — ServerListPing event, UpdatePlayerSample response
- [x] Rate-limited plugin-channel routing (anti-spam) — token-bucket per player, 10 msg/sec default
- [x] Built-in proxy-protocol v2 acceptor (Cloudflare Spectrum, HAProxy) — optional mode for direct/LB hybrid
- [x] Per-player metrics + structured packet trace dumps — packet counters, latency tracking, trace buffer
- [x] Backend health probes with automatic eviction and re-entry — TCP probes, configurable thresholds
- [x] Active-passive backend failover groups — primary/standby with auto-failback
- [x] Resource-pack pinning + signed bundle injection — forced resource packs on join
- [x] Region-aware lobby auto-selection — pick the nearest lobby on first connect
- [x] Cookies & Transfers passthrough (1.20.5+) — preserve server-driven reconnects
- [x] Chat-signing translation (1.19+) — sign/strip ServerboundChatMessage cleanly when bridging versions
- [x] Configuration-phase synthesis — bridge modern clients to pre-1.20.2 backends without stalling
- [x] Dimension codec / registry NBT injection for cross-era JoinGame
- [x] WASM plugin runtime — sandboxed, portable plugins
- [x] Full 1.7.x ↔ 1.21 protocol coverage — fill out the remaining wiki-correct converters per family
- [x] Chunk repack across the 1.13 flattening + 1.14 biome/storage rewrites
- [x] Pluggable encryption (post-quantum KEM exploration)
- [x] gRPC control plane for external orchestration

## In Progress

## Planned

## Exploring
- [ ] Bedrock edition bridging
- [ ] Plugin marketplace / registry
- [ ] QUIC / HTTP/3 client transport — once a vanilla client supports it
- [ ] Cross-proxy player-state replication (BungeeMessaging-compatible)
- [ ] Mojang Realms compatibility layer
