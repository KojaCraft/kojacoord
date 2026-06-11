//! Generator for `data/dimension_codec_<proto>.nbt.bin` files.
//!
//! Reads a local clone of `PrismarineJS/minecraft-data` and copies each
//! version's `dimension_codec.nbt` (raw Java network-NBT) into per-proto
//! files that the runtime loader in
//! `crates::protocol::dimension_codec::build_dimension_codec_for_proto`
//! will eventually `include_bytes!` at compile time.
//!
//! Mirrors the existing `gen_flattening` pattern:
//!   * generator binary in `src/bin/`,
//!   * baked data files under `crates/protocol/data/`,
//!   * `cargo run -p kojacoord-protocol --bin gen_dimension_codec ...`.
//!
//! Per [[project-prismarine-generator]] (in repo memory): the build.rs
//! versus standalone-binary decision was resolved in favour of "mirror
//! the existing standalone-binary pattern" — both keep the network
//! fetch outside the normal `cargo build` path. No network access is
//! performed by this binary itself; the user is expected to clone
//! minecraft-data first and pass its path with `--data`.
//!
//! ## Source layout expected
//!
//! PrismarineJS/minecraft-data's pc tree puts the codec at:
//!   `data/pc/<version>/dimension_codec.nbt`
//!
//! Proto numbers live in `data/pc/<version>/version.json::version`. We
//! read both, then emit one binary file per unique proto.
//!
//! Older protos (≤ 754) and post-1.20.2 protos (≥ 764, configuration
//! phase split) don't carry a codec in JoinGame and are skipped.
//!
//! ## Usage
//!
//! ```bash
//! cargo run -p kojacoord-protocol --bin gen_dimension_codec -- \
//!     --data /path/to/minecraft-data/data/pc \
//!     --out  crates/protocol/data
//! ```
//!
//! The output is binary NBT (Java named-tag format, no length prefix —
//! self-framing) which matches what every 1.16+ Notchian server writes
//! into the JoinGame packet.

use std::fs;
use std::path::{Path, PathBuf};

#[derive(serde::Deserialize)]
struct VersionMeta {
    version: u32,
}

fn main() {
    let mut data_dir: Option<PathBuf> = None;
    let mut out_dir: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--data" => data_dir = args.next().map(PathBuf::from),
            "--out" => out_dir = args.next().map(PathBuf::from),
            "--help" | "-h" => {
                print_usage();
                return;
            },
            other => {
                eprintln!("unknown argument: {}", other);
                print_usage();
                std::process::exit(2);
            },
        }
    }

    let data_dir = match data_dir {
        Some(p) => p,
        None => {
            eprintln!("missing --data <minecraft-data/data/pc>");
            print_usage();
            std::process::exit(2);
        },
    };
    let out_dir = match out_dir {
        Some(p) => p,
        None => {
            eprintln!("missing --out <crates/protocol/data>");
            print_usage();
            std::process::exit(2);
        },
    };

    if !data_dir.is_dir() {
        eprintln!("--data path is not a directory: {}", data_dir.display());
        std::process::exit(1);
    }
    if !out_dir.is_dir() {
        eprintln!("--out path is not a directory: {}", out_dir.display());
        std::process::exit(1);
    }

    let mut emitted = 0;
    let mut skipped_no_codec = 0;
    let mut skipped_no_version = 0;

    // Walk every `<version>/` subdirectory in data/pc.
    for entry in fs::read_dir(&data_dir).expect("read data dir") {
        let entry = entry.expect("read entry");
        if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            continue;
        }
        let path = entry.path();
        let version_json = path.join("version.json");
        if !version_json.is_file() {
            continue;
        }
        let codec_nbt = path.join("dimension_codec.nbt");
        if !codec_nbt.is_file() {
            skipped_no_codec += 1;
            continue;
        }

        let version_meta: VersionMeta = match read_json(&version_json) {
            Ok(m) => m,
            Err(e) => {
                eprintln!(
                    "warn: skipping {}: failed to parse version.json: {}",
                    path.display(),
                    e
                );
                skipped_no_version += 1;
                continue;
            },
        };
        let proto = version_meta.version;

        // Per minecraft.wiki: dimension codec only lives in JoinGame
        // between proto 735 (1.16) and proto 763 (1.20.1). 1.20.2+
        // moved it to the configuration phase.
        if !(735..=763).contains(&proto) {
            continue;
        }

        let nbt_bytes = fs::read(&codec_nbt).expect("read codec nbt");
        let out_file = out_dir.join(format!("dimension_codec_{}.nbt.bin", proto));
        fs::write(&out_file, &nbt_bytes).expect("write output");
        emitted += 1;
        eprintln!(
            "ok: proto {} ({}) <- {} bytes",
            proto,
            path.file_name().and_then(|s| s.to_str()).unwrap_or("?"),
            nbt_bytes.len()
        );
    }

    eprintln!(
        "\nsummary: {} emitted, {} skipped (no codec), {} skipped (bad version.json)",
        emitted, skipped_no_codec, skipped_no_version
    );
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read: {}", e))?;
    serde_json::from_str(&text).map_err(|e| format!("parse: {}", e))
}

fn print_usage() {
    eprintln!(
        "usage:\n  gen_dimension_codec \\\n    --data /path/to/minecraft-data/data/pc \\\n    --out  crates/protocol/data\n\nReads dimension_codec.nbt + version.json under each subdir of --data,\nemits one binary NBT per unique proto under --out.\nOnly protos 735..=763 (1.16 \u{2014} 1.20.1) carry a codec in JoinGame.\n"
    );
}
