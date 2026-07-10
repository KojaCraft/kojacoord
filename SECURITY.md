# Security Policy

Kojacoord terminates untrusted TCP connections from the public internet and
speaks a binary protocol to them before any authentication happens — that's
the part of this project that actually needs a security policy, so this
document is about that, not a generic template.

## Supported versions

Pre-1.0. Only the latest release gets security fixes; there's no maintained
LTS branch yet.

## Reporting a vulnerability

Don't open a public issue for it. Email
[security@kojacoord.net](mailto:security@kojacoord.net) instead, with:

- What the vulnerability is and where (file/function if you have it)
- Steps to reproduce
- What it lets an attacker actually do
- A patch or mitigation, if you have one — not required

You'll get an acknowledgment within 48 hours and a real response (not just
"still looking") within 7 days. We'll credit you in the advisory unless you
ask us not to. Give us a reasonable window to ship a fix before disclosing
publicly; we're not going to drag that out to avoid embarrassment.

## What's actually in scope

The realistic attack surface here is: the Minecraft listener (anyone can
connect), the packet parsers for every supported protocol version, the
login/encryption handshake, and — if you've enabled them — the TCP
server-management control plane and the gRPC control plane. Those last two
are meant for trusted orchestration tooling, not the public internet; if
you find a way to reach them without the configured auth token, that's a
real bug.

Out of scope: anything that requires an already-compromised backend server,
a plugin the operator chose to install, or physical/OS-level access to the
box the proxy runs on. We'll still take the report, just don't expect it to
be treated as urgent.

## What's actually built to hold up

Rather than a generic checklist, here's what's specific to this codebase:

- **No database, no admin HTTP surface.** The proxy holds no persistent
  state and doesn't expose an inbound REST/admin API — the only thing it
  listens on over HTTP is a read-only Prometheus metrics endpoint. That's
  not a missing feature, it's less attack surface: there's no SQL to inject,
  no admin panel to leave world-readable, no database credentials to leak.
- **Per-connection panic isolation.** Every client connection runs in its
  own task, and the release build does *not* use `panic = "abort"` — a
  panic triggered by one malformed packet from one client tears down that
  one connection, not the whole proxy. (This one's a real footgun in
  network services that get it wrong: `panic = "abort"` turns "attacker
  sends one bad packet" into "every player on the server gets disconnected.")
- **Signed login, signed profiles.** Online-mode login uses RSA key
  exchange + AES-CFB8 encryption per the vanilla protocol, and Mojang's
  profile-property signatures are verified against the configured public
  key before a skin/cape is trusted — so a MITM can't forge one in.
- **Constant-time token comparisons.** Every control-plane auth check
  (TCP server-management, gRPC control plane) compares tokens in constant
  time, not with `==`, so a timing attack can't leak the token
  byte-by-byte.
- **Inbound abuse guards.** A sliding-window packet-rate ceiling and a
  per-packet size cap on the client→backend direction, independent of
  per-IP connection throttling (token-bucket, with automatic temporary
  bans) and a pluggable IP-reputation blocklist (static CIDRs plus an
  optional external provider, fail-open on provider timeout so a slow
  third party can't become a denial-of-service vector).
- **Known, accepted limitation:** legacy BungeeCord-style player-info
  forwarding is unsigned by protocol design — it always has been, in every
  implementation of it. If you enable it, your backend servers must be
  firewalled to only accept connections from the proxy, or a player can
  spoof another player's identity. Use Velocity-style forwarding (HMAC-signed)
  instead unless you specifically need BungeeCord compatibility.

## Dependencies

`cargo audit` and `cargo deny` (advisories/bans/licenses/sources) both run
in CI on every push/PR and on a weekly cron, so a newly disclosed advisory
in a transitive dependency gets caught even with no code changes. Advisories
we've deliberately chosen not to act on (no fix available upstream, or the
affected code path isn't reachable with untrusted input) are listed with a
written justification in [`.cargo/audit.toml`](.cargo/audit.toml) — that
file is the actual source of truth, not this one.

## License

Reported vulnerabilities are handled under the same MIT license as the rest
of the project. We ask for a reasonable disclosure window; we're not asking
for secrecy forever.
