# Kojacoord Proxy Usage Guide

A deeper walkthrough than the [README](../README.md#configuration) — full
config sections, the TCP management protocol, and troubleshooting. If
something here and the code disagree, the code wins; the generated default
`config.toml` has per-field comments that are the actual source of truth.

## Table of Contents

1. [Installation](#installation)
2. [Configuration basics](#configuration-basics)
3. [Multi-server setup](#multi-server-setup)
4. [Regional routing and backend pools](#regional-routing-and-backend-pools)
5. [Authentication](#authentication)
6. [Connection protection](#connection-protection)
7. [Maintenance mode and queueing](#maintenance-mode-and-queueing)
8. [Server management protocol](#server-management-protocol)
9. [Monitoring](#monitoring)
10. [Troubleshooting](#troubleshooting)

## Installation

```bash
git clone https://github.com/KojaCraft/kojacoord.git
cd kojacoord-proxy
cargo build --release
# Binary at target/release/kojacoord-proxy
```

**Requirements:** Rust 1.92+, `protoc` (Protocol Buffers compiler, needed
for the gRPC control plane), 2+ CPU cores recommended, 512MB RAM minimum.
No database — Kojacoord holds no persistent state of its own.

## Configuration basics

On first run Kojacoord writes a default `config.toml` next to the binary,
generates strong tokens for anything enabled that needs one, and prompts
you to accept the Minecraft EULA. A representative excerpt:

```toml
[proxy]
bind = "0.0.0.0:25565"
online_mode = true
compression_threshold = 256
max_players = 1000
session_timeout_secs = 30
prevent_proxy_connections = true

[[servers]]
name = "lobby"
address = "localhost:25566"
backend_type = "vanilla"       # vanilla | spigot | forge | hybrid

[[servers]]
name = "survival"
address = "localhost:25567"
backend_type = "vanilla"

[server_management]
enabled = true
bind = "127.0.0.1:8080"
auth_token = "change-this-token"
```

- **bind** — where the proxy listens for players.
- **online_mode** — Mojang authentication on/off (see
  [Authentication](#authentication)).
- **compression_threshold** — packet size (bytes) above which the modern
  wire format compresses; `-1` disables compression entirely.
- **prevent_proxy_connections** — reject connections coming through another
  proxy (Mojang session-server check), independent of the IP-reputation
  blocklist below.

Each backend under `[[servers]]` also accepts `display_name`, `motd`,
`modpack`/`modpack_version`, `game_type`, a per-server
`compression_threshold` override, `health_probe_interval_secs` (TCP health
checks; `0` disables them for that server), `max_players` (enables
[queueing](#maintenance-mode-and-queueing) once it's reached), `region`
(for [GeoIP routing](#regional-routing-and-backend-pools)), and `weight`
(for weighted selection in a `[[server_groups]]` pool).

Most of this hot-reloads on file save or `SIGHUP` (Unix); a few fields need
a restart — the generated config comments say which.

## Multi-server setup

Players land on `routing.default_server` (or the first `[[servers]]` entry
if that's unset) unless a rule matches first:

```toml
[routing]
default_server = "lobby"

[[routing.rules]]
label = "vip-lobby"
name_glob = "vip_*"
target = "vip-lobby"

[[routing.rules]]
label = "internal-network"
client_cidrs = ["10.0.0.0/8"]
target = "staging-lobby"
```

Rules are evaluated in order; the first match wins. `name_glob` is a
case-insensitive username glob (`*` wildcards); `client_cidrs` matches the
connecting IP (IPv4/IPv6). A rule with neither set matches everyone, so put
a catch-all last if you want one.

## Regional routing and backend pools

Point `[geoip].database_path` at a MaxMind GeoLite2 Country or City `.mmdb`
file (free account required at maxmind.com) and lobby selection prefers the
server whose `region` matches the connecting player's, falling back through
a fixed priority order (same continent first, then the rest) to any healthy
server. With no database configured, every player buckets to `"global"` —
the same as omitting this section entirely.

```toml
[geoip]
database_path = "/etc/kojacoord/GeoLite2-City.mmdb"
```

For load-balancing across interchangeable backends (several "survival-N"
instances behind one logical name), define a group and point a routing rule
or `default_server` at its name instead of a literal server:

```toml
[[server_groups]]
name = "survival-pool"
members = ["survival-1", "survival-2"]
strategy = "least_connections"   # least_connections | weighted | latency
```

`weighted` uses each member's `weight`; `latency` prefers the member with
the lowest measured TCP-connect time from the health prober (members
without `health_probe_interval_secs > 0` never get preferred, since there's
no measurement to prefer them on).

## Authentication

```toml
[proxy]
online_mode = true
```

In online mode, the proxy runs the full Mojang `hasJoined` session check,
RSA key exchange, and AES-CFB8 encryption, and verifies profile-property
signatures against `proxy.mojang_public_key` before trusting a skin/cape.

```toml
[proxy]
online_mode = false
```

In offline mode, no Mojang round-trip happens, there's no encryption, and
UUIDs are derived deterministically from the username (the same
`OfflinePlayer:<name>` MD5 scheme vanilla/Bukkit offline servers use, so
they're stable across restarts and match what any other offline server
would derive for the same name).

There's no pluggable custom-auth trait to implement — if you need different
authentication behavior, it goes through the plugin system's `PreLogin`
hook instead (a plugin can accept or reject a connection before the login
handshake completes).

## Connection protection

Two independent layers run before a connection reaches any per-player
logic, both unconditional (not something you toggle a subsystem for):

- A sliding-window packet-rate ceiling and per-packet size cap on the
  client→backend direction (catches floods and the classic oversized-varint
  crash), and a per-IP token-bucket connection throttle with automatic
  temporary bans (`proxy.max_connections_per_ip`, `0` disables it).
- An optional IP-reputation blocklist:

```toml
[ip_reputation]
blocklist_cidrs = ["203.0.113.0/24"]
provider_url = "https://your-reputation-service/check"
api_key = ""
cache_ttl_secs = 3600
timeout_ms = 300
```

`blocklist_cidrs` is checked with zero I/O. `provider_url`, if set, gets an
outbound `GET ?ip=<addr>` expecting `{"blocked": bool}` back; a slow or
unreachable provider **fails open** (connection allowed) rather than
stalling new connections, and results are cached per-IP for
`cache_ttl_secs`.

Actual anti-cheat (movement/combat analysis) isn't something the proxy
does itself — it dispatches a `PlayerMove` event to plugins (only when one
is actually subscribed, so it costs nothing when unused) and a plugin
decides what to do with it, including kicking the player.

## Maintenance mode and queueing

```toml
[maintenance]
enabled = false
kick_message = "The network is down for maintenance."
bypass_uuids = ["11111111-1111-1111-1111-111111111111"]

[queue]
enabled = true
max_queue_size = 0   # 0 = unlimited
```

`maintenance.enabled` is only the boot-time default — flip it live over the
[server management protocol](#server-management-protocol) without a
restart or file edit; `bypass_uuids` still comes from the config file.

When a target server's `max_players` is reached, a connecting player is
held in limbo with a live "Position in queue: N" message instead of being
rejected, and let through in arrival order as slots free up.

## Server management protocol

A small newline-delimited-JSON protocol over raw TCP, meant for trusted
orchestration tooling (not players, not the public internet) —
register/deregister backends, transfer players, evacuate a server, and
toggle maintenance mode live.

```toml
[server_management]
enabled = true
bind = "127.0.0.1:8080"
auth_token = "change-this-token"
```

Every message is a single JSON line ending in `\n`; the message type is
inferred from which fields are present. `auth_token` is required on every
message and compared in constant time. Examples (via `nc localhost 8080`):

```
{"name":"survival-2","address":"10.0.0.5","port":25565,"max_players":100,"auth_token":"change-this-token"}
```
Registers (or updates, if the name already exists) a backend.

```
{"uuid":"...","server":"survival","auth_token":"change-this-token"}
```
Transfers an online player to `server` by UUID.

```
{"enabled":true,"auth_token":"change-this-token"}
```
Toggles maintenance mode live.

The server replies with a single `OK: ...` or `ERROR: ...` line per
message.

## Monitoring

```bash
RUST_LOG=info ./kojacoord-proxy    # or debug for more detail
```

Enable the Prometheus endpoint for scraping instead of parsing logs:

```toml
[metrics]
enabled = true
bind = "127.0.0.1:9090"
```

There's no other inbound HTTP surface — no admin API, no dashboard. See the
README's [Metrics](../README.md#metrics) section for why that's deliberate.

## Troubleshooting

**Players can't connect**
- Check the firewall for the proxy's `bind` port.
- Confirm `bind` in the config matches what you expect (default
  `0.0.0.0:25565`).
- Confirm the backend server(s) are reachable from the proxy's host.

**Authentication failures**
- Confirm `online_mode` matches what you actually want (`true` needs
  outbound access to Mojang's session servers).
- Check for a rejected profile signature in the logs — that means Mojang's
  public key in config doesn't match what's actually expected, or someone
  tried to forge a profile.

**High latency / lag**
- Raise `compression_threshold` if CPU-bound on small packets, lower it if
  bandwidth-bound.
- Check `[metrics]` for connection counts and packet rates before assuming
  it's the proxy and not a backend server.

**A specific client version misbehaves**
- Check the [version support table](../README.md#version-support) — not
  every version has been verified to the same depth.
- The proxy forwards play packets verbatim and relies on ViaVersion running
  on the backend for gameplay-packet translation between versions; if a
  specific block/entity/item doesn't render right, that's usually a
  ViaVersion/ViaBackwards configuration question on the backend, not
  something to fix in the proxy config.

## Additional resources

- [README.md](../README.md) — project overview, feature list, security model
- [ROADMAP.md](../ROADMAP.md) — what's shipped and what's planned
- [GitHub Issues](https://github.com/KojaCraft/kojacoord/issues) — bug reports and feature requests
- [Discord](https://discord.gg/Xp6wFH3nM6) — community chat
