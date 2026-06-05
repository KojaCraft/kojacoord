![Koja Banner](../docs/koja_banner.png)

# cargo-kpl

A Cargo subcommand for building Kojacoord plugins (`.kpl` files).

## Installation

Install from the local repository:

```bash
cargo install --path cargo-kpl
```

Or install from crates.io (when published):

```bash
cargo install cargo-kpl
```

## Usage

### Build a Plugin

Build a plugin from the current directory:

```bash
cargo kpl build
```

With custom output path:

```bash
cargo kpl build --output my-plugin.kpl
```

Release build:

```bash
cargo kpl build --release
```

Custom plugin name:

```bash
cargo kpl build --name my-plugin
```

### Package an Existing Library

Package a pre-compiled library into a `.kpl` file:

```bash
cargo kpl package --input target/release/libmy_plugin.dll --output my-plugin.kpl
```

With custom metadata:

```bash
cargo kpl package --input target/release/libmy_plugin.dll --metadata metadata.toml
```

## Plugin Metadata

The tool automatically extracts metadata from `Cargo.toml`. You can also provide a custom `metadata.toml` file:

```toml
name = "my-plugin"
version = "1.0.0"
author = "Your Name"
description = "My Kojacoord plugin"
min_proxy_version = "0.1.0"
dependencies = []
```

## .kpl File Format

A `.kpl` file is a ZIP archive containing:

- `metadata.json` - Plugin metadata (JSON)
- `<library>.dll`/`.so`/`.dylib` - Compiled plugin library
- `plugin.toml` - Optional plugin configuration
