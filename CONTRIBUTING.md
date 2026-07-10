# Contributing to Kojacoord

Thanks for considering it. This is a fairly technical codebase (a Minecraft
protocol proxy with per-version packet handling), so a bit of context up
front will save you time.

Found a security issue instead of a bug? Don't open an issue for it — see
[SECURITY.md](SECURITY.md).

## Before you start

- Rust 1.92+ and `protoc` (needed for the gRPC control plane). See the
  [README](README.md#building) for the full build setup.
- For anything non-trivial, open an issue first, especially for new
  protocol version support or anything touching packet framing/compression
  — those are the most fragile parts of the codebase and it's easy to build
  something that works for the version you tested and breaks three others.
- Check [ROADMAP.md](ROADMAP.md) and existing issues so you're not
  duplicating work already in flight.

## Workflow

1. Fork the repo, branch off `main`.
2. Make your change. Keep it scoped — a bug fix doesn't need a refactor
   riding along with it, and vice versa.
3. Add tests for new behavior and rustdoc (`///`) for anything public.
4. Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`,
   `cargo test --workspace`. CI runs the same checks (plus `cargo audit` /
   `cargo deny`) and will fail the same way it does for you locally.
5. Open a PR describing what changed and why — the *why* matters more than
   a mechanical diff summary. Reference any related issue.

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/)
(`feat:`, `fix:`, `refactor:`, `chore:`, etc.) — that's what drives the
generated changelog, so it's not just a style preference.

## Reporting bugs

Include the proxy version, Rust version, OS, the client and backend
Minecraft versions involved, relevant logs, and a config with secrets
redacted. "It doesn't work" without a repro is hard to act on in a codebase
that behaves differently per protocol version.

## Adding a dependency

New dependencies go through `cargo deny` (license, advisory, and source
checks — see `deny.toml`) in CI. If you're adding something with an
unusual license or pulling from a non-crates.io source, expect that check
to flag it and be ready to justify it in the PR.

## Project layout

```
kojacoord-proxy/
├── crates/
│   ├── protocol/        # Packet types, codecs, per-version registries
│   ├── netty/           # Framing, compression, encryption codec layer
│   ├── auth/            # Session authentication, login-phase encryption
│   ├── proxy-core/      # The proxy itself: relay, routing, limbo, control planes
│   ├── config/          # Config schema, loading, validation
│   ├── api/             # Public API surface for plugin development
│   ├── plugin-abi/      # Wire types shared by the plugin host and guest SDK
│   ├── plugin-sdk/      # Guest SDK for writing WASM plugins
│   ├── plugin-system/   # Plugin loading, lifecycle, host API
│   ├── cluster/         # Redis-backed cluster coordination
│   └── metrics/         # Prometheus metrics collection and exporter
├── src/                 # Binary entry point
├── docs/                # Usage docs, brand assets
├── kr8s/                # Kubernetes manifests
└── Cargo.toml           # Workspace definition
```

If you're touching `proxy-core::net` (connection/relay/limbo), read the
module docs there first — a lot of the trickier behavior (per-protocol-era
framing, the shared client-writer mutex, live server switching) is
documented inline because it's not obvious from the code alone.

## Code of conduct

Covered in [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## License

Contributions are licensed under the project's [MIT License](LICENSE).
