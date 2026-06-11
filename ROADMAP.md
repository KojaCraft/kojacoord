# Kojacoord Roadmap

This file is the single source of truth for the public roadmap shown on
[kojacoord.net](https://kojacoord.net). The landing page reads it directly from
the default branch, so editing it here updates the site.

**Format:** each `##` heading is a phase. Each list item is a roadmap entry —
use `- [x]` for shipped, `- [ ]` for not-yet-done. Optionally add `— short note`
after the text for extra context.

## Shipped
- [x] Multi-version protocol support — Java Edition 1.7.x through 1.21.x with automatic conversion
- [x] Authentication pipeline — online-mode Mojang session auth + offline-mode support
- [x] Anti-cheat engine at the proxy edge
- [x] WASM plugin runtime — sandboxed, hot-reloadable plugins
- [x] Cluster mode with autoscaling
- [x] Per-player and per-region routing — glob-matched usernames + IPv4/IPv6 CIDR ranges
- [x] Active-passive backend failover groups
- [x] Backend health probes with automatic eviction
- [x] Region-aware lobby auto-selection
- [x] Live config reload — routing rules, servers, MOTD without restart
- [x] Velocity-style + BungeeCord forwarding
- [x] PROXY protocol v2 acceptor — Cloudflare Spectrum, HAProxy compatible
- [x] Connection throttling per IP
- [x] Plugin permissions / capability sandboxing
- [x] Anonymous, opt-out telemetry
- [x] Legacy 0xFE server-list ping — pre-1.7 / 1.6.x MOTD support
- [x] Block-state ↔ legacy-id flattening — 300+ entry conversion table

## In Progress
- [ ] Protocol support: 1.6.x (PreNetty)
- [ ] Protocol support: 1.13.x
- [ ] Protocol support: 1.14.x
- [ ] Protocol support: 1.15.x
- [ ] Protocol support: 1.16.x
- [ ] Protocol support: 1.17.x
- [ ] Protocol support: 1.18.x
- [ ] Protocol support: 1.19.x
- [ ] Protocol support: 1.20.x
- [ ] Protocol support: 1.21.x

## Planned
- [ ] Bedrock edition bridging
- [ ] Plugin marketplace / registry
- [ ] Mojang Realms compatibility layer

## Exploring
- [ ] QUIC / HTTP/3 client transport — once a vanilla client supports it