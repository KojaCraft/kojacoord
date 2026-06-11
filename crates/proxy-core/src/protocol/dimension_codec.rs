//! Dimension codec injection for cross-era JoinGame bridging.
//!
//! 1.16+ clients expect a dimension codec NBT embedded in JoinGame; 1.15
//! and earlier servers don't send one, and 1.20.2+ clients expect it
//! through the configuration phase instead. When the wire format on
//! both ends disagrees, we synthesise a minimal codec / dimension type
//! NBT here and splice it into the relayed packet stream so the client
//! has something to chew on.
//!
//! The synthesised values are deliberately minimal: one overworld
//! dimension, default biome registry. They're enough to keep the
//! client out of an error state until the real backend pushes its own
//! data — the proxy doesn't pretend to host a world.

use bytes::BytesMut;
use kojacoord_protocol::{
    types::{Nbt, NbtTag},
    Encode, ProtocolVersion,
};

/// True for 1.16+ clients that expect a dimension codec NBT on JoinGame.
pub fn uses_dimension_codec(protocol_version: u32) -> bool {
    let canonical = ProtocolVersion::from_id(protocol_version);
    matches!(
        canonical.epoch(),
        kojacoord_protocol::Epoch::V1_16
            | kojacoord_protocol::Epoch::V1_17_To_1_18
            | kojacoord_protocol::Epoch::V1_19
            | kojacoord_protocol::Epoch::V1_20
            | kojacoord_protocol::Epoch::V1_21Plus
    )
}

/// True when only one side of the bridge speaks the codec — we'll need
/// to synthesise it (or drop it) to keep them in sync.
pub fn needs_codec_injection(client_protocol: u32, backend_protocol: u32) -> bool {
    uses_dimension_codec(client_protocol) && !uses_dimension_codec(backend_protocol)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecInjectionMode {
    /// Both sides agree — pass JoinGame through unchanged.
    None,
    /// Client expects a codec the backend never produces; we inject
    /// one on the way to the client.
    ClientSide,
    /// Backend wants a codec from a client that doesn't have one
    /// (rare; mostly happens when bridging snapshots).
    BackendSide,
}

/// Decide which direction (if any) needs codec injection for the given
/// client/backend pair.
pub fn determine_injection_mode(client_protocol: u32, backend_protocol: u32) -> CodecInjectionMode {
    match (
        uses_dimension_codec(client_protocol),
        uses_dimension_codec(backend_protocol),
    ) {
        (true, false) => CodecInjectionMode::ClientSide,
        (false, true) => CodecInjectionMode::BackendSide,
        _ => CodecInjectionMode::None,
    }
}

/// Build a dimension codec NBT for the given protocol.
///
/// Per BungeeCord `protocol/Login.java`, the codec field exists from
/// proto 735 (1.16) onward and was removed when the Configuration
/// phase split out the registry data at proto 764 (1.20.2).
///
/// Strategy: prefer the byte-for-byte PrismarineJS codec for this
/// proto if `crates/protocol/data/dimension_codec_<proto>.nbt.bin`
/// was populated by `gen_dimension_codec`. Otherwise fall back to the
/// synthesised minimal codec which is enough to pass the client's
/// "did the server send a codec?" check but doesn't enumerate the
/// nether/end dimensions or the full biome registry.
pub fn build_dimension_codec_for_proto(proto: u32) -> Result<Vec<u8>, String> {
    if let Some(bytes) = prismarine_codec_for_proto(proto) {
        return Ok(bytes.to_vec());
    }
    build_minimal_dimension_codec()
}

/// Lookup a baked PrismarineJS dimension codec by proto. Returns
/// `Some(bytes)` only for protos where the build embedded a real
/// codec via `include_bytes!`. Empty list today — populate by running
/// `cargo run -p kojacoord-protocol --bin gen_dimension_codec` against
/// a local minecraft-data clone, then add `match` arms here pointing
/// at the resulting `dimension_codec_<proto>.nbt.bin` files. Mirrors
/// the runtime side of the existing `gen_flattening` pattern.
fn prismarine_codec_for_proto(proto: u32) -> Option<&'static [u8]> {
    // Populated by the generator — see file-level comment.
    // Example (uncomment after running the generator):
    //   754 => Some(include_bytes!(
    //       "../../../protocol/data/dimension_codec_754.nbt.bin"
    //   )),
    let _ = proto;
    None
}

/// Build a minimal `minecraft:dimension_type` + `minecraft:worldgen/biome`
/// registry NBT — one overworld entry, default biome. Just enough to
/// pass the client's "did the server send a codec?" check.
pub fn build_minimal_dimension_codec() -> Result<Vec<u8>, String> {
    let mut codec_nbt = Nbt::empty("");

    // Build dimension_type registry
    let mut dimension_type = NbtTag::compound();
    dimension_type.as_compound_mut().unwrap().insert(
        "type".to_string(),
        NbtTag::string("minecraft:dimension_type"),
    );

    let mut value_list = NbtTag::list();
    if let Some(list) = value_list.as_list_mut() {
        // Overworld dimension
        let mut overworld = NbtTag::compound();
        if let Some(compound) = overworld.as_compound_mut() {
            compound.insert("name".to_string(), NbtTag::string("minecraft:overworld"));
            compound.insert("id".to_string(), NbtTag::int(0));

            let mut element = NbtTag::compound();
            if let Some(elem) = element.as_compound_mut() {
                elem.insert("height".to_string(), NbtTag::int(256));
                elem.insert("min_y".to_string(), NbtTag::int(0));
                elem.insert("has_ceiling".to_string(), NbtTag::byte(0));
                elem.insert("has_skylight".to_string(), NbtTag::byte(1));
                elem.insert("natural".to_string(), NbtTag::byte(1));
                elem.insert("ambient_light".to_string(), NbtTag::float(0.0));
                elem.insert(
                    "infiniburn".to_string(),
                    NbtTag::string("minecraft:infiniburn_overworld"),
                );
                elem.insert("respawn_anchor_works".to_string(), NbtTag::byte(0));
                elem.insert("ultrawarm".to_string(), NbtTag::byte(0));
                elem.insert("bed_works".to_string(), NbtTag::byte(1));
            }
            compound.insert("element".to_string(), element);
        }
        list.push(overworld);
    }
    dimension_type
        .as_compound_mut()
        .unwrap()
        .insert("value".to_string(), value_list);

    codec_nbt
        .root
        .insert("minecraft:dimension_type".to_string(), dimension_type);

    // Build biome registry
    let mut biome = NbtTag::compound();
    biome.as_compound_mut().unwrap().insert(
        "type".to_string(),
        NbtTag::string("minecraft:worldgen/biome"),
    );

    let mut biome_value_list = NbtTag::list();
    if let Some(list) = biome_value_list.as_list_mut() {
        // Plains biome
        let mut plains = NbtTag::compound();
        if let Some(compound) = plains.as_compound_mut() {
            compound.insert("name".to_string(), NbtTag::string("minecraft:plains"));
            compound.insert("id".to_string(), NbtTag::int(1));

            let mut element = NbtTag::compound();
            if let Some(elem) = element.as_compound_mut() {
                elem.insert("precipitation".to_string(), NbtTag::string("rain"));
                elem.insert("depth".to_string(), NbtTag::float(0.125));
                elem.insert("temperature".to_string(), NbtTag::float(0.8));
                elem.insert("scale".to_string(), NbtTag::float(0.05));
                elem.insert("downfall".to_string(), NbtTag::float(0.4));
                elem.insert("category".to_string(), NbtTag::string("plains"));
            }
            compound.insert("element".to_string(), element);
        }
        list.push(plains);
    }
    biome
        .as_compound_mut()
        .unwrap()
        .insert("value".to_string(), biome_value_list);

    codec_nbt
        .root
        .insert("minecraft:worldgen/biome".to_string(), biome);

    // Encode to bytes
    let mut buffer = BytesMut::new();
    codec_nbt
        .encode(&mut buffer)
        .map_err(|e| format!("Failed to encode dimension codec NBT: {}", e))?;

    Ok(buffer.to_vec())
}

/// Build the registry NBT 1.19+ clients expect alongside the dimension
/// codec — currently only `minecraft:chat_type` (translation key +
/// `sender`/`content` parameters), which is all the client checks during
/// JoinGame.
pub fn build_minimal_registry() -> Result<Vec<u8>, String> {
    let mut registry_nbt = Nbt::empty("");

    // Build chat_type registry
    let mut chat_type = NbtTag::compound();
    chat_type
        .as_compound_mut()
        .unwrap()
        .insert("type".to_string(), NbtTag::string("minecraft:chat_type"));

    let mut value_list = NbtTag::list();
    if let Some(list) = value_list.as_list_mut() {
        // Chat type
        let mut chat = NbtTag::compound();
        if let Some(compound) = chat.as_compound_mut() {
            compound.insert("name".to_string(), NbtTag::string("minecraft:chat"));
            compound.insert("id".to_string(), NbtTag::int(0));

            let mut element = NbtTag::compound();
            if let Some(elem) = element.as_compound_mut() {
                let mut chat_elem = NbtTag::compound();
                if let Some(c) = chat_elem.as_compound_mut() {
                    c.insert(
                        "translation_key".to_string(),
                        NbtTag::string("chat.type.text"),
                    );

                    let mut params = NbtTag::list();
                    if let Some(p) = params.as_list_mut() {
                        p.push(NbtTag::string("sender"));
                        p.push(NbtTag::string("content"));
                    }
                    c.insert("parameters".to_string(), params);
                }
                elem.insert("chat".to_string(), chat_elem);

                let mut narration_elem = NbtTag::compound();
                if let Some(n) = narration_elem.as_compound_mut() {
                    n.insert(
                        "translation_key".to_string(),
                        NbtTag::string("chat.type.text.narrate"),
                    );

                    let mut params = NbtTag::list();
                    if let Some(p) = params.as_list_mut() {
                        p.push(NbtTag::string("sender"));
                        p.push(NbtTag::string("content"));
                    }
                    n.insert("parameters".to_string(), params);
                }
                elem.insert("narration".to_string(), narration_elem);
            }
            compound.insert("element".to_string(), element);
        }
        list.push(chat);
    }
    chat_type
        .as_compound_mut()
        .unwrap()
        .insert("value".to_string(), value_list);

    registry_nbt
        .root
        .insert("minecraft:chat_type".to_string(), chat_type);

    // Encode to bytes
    let mut buffer = BytesMut::new();
    registry_nbt
        .encode(&mut buffer)
        .map_err(|e| format!("Failed to encode registry NBT: {}", e))?;

    Ok(buffer.to_vec())
}

/// Convenience alias for [`build_minimal_dimension_codec`]; kept for
/// call-site readability where "codec NBT" reads more naturally than
/// "minimal codec".
pub fn dimension_codec_nbt() -> Result<Vec<u8>, String> {
    build_minimal_dimension_codec()
}

/// Standalone dimension-type NBT (the inner element 1.16.2+ JoinGame
/// carries separately from the codec). `_dim_key` is reserved — the
/// minimal builder always emits an overworld-shaped element regardless,
/// since we don't synthesise nether/end worlds at the proxy.
pub fn dimension_type_nbt(_dim_key: &str) -> Result<Vec<u8>, String> {
    let mut dim_type = Nbt::empty("");

    let mut element = NbtTag::compound();
    if let Some(elem) = element.as_compound_mut() {
        elem.insert("height".to_string(), NbtTag::int(256));
        elem.insert("min_y".to_string(), NbtTag::int(0));
        elem.insert("has_ceiling".to_string(), NbtTag::byte(0));
        elem.insert("has_skylight".to_string(), NbtTag::byte(1));
        elem.insert("natural".to_string(), NbtTag::byte(1));
        elem.insert("ambient_light".to_string(), NbtTag::float(0.0));
        elem.insert(
            "infiniburn".to_string(),
            NbtTag::string("minecraft:infiniburn_overworld"),
        );
        elem.insert("respawn_anchor_works".to_string(), NbtTag::byte(0));
        elem.insert("ultrawarm".to_string(), NbtTag::byte(0));
        elem.insert("bed_works".to_string(), NbtTag::byte(1));
    }

    dim_type.root.insert("element".to_string(), element);

    let mut buffer = BytesMut::new();
    dim_type
        .encode(&mut buffer)
        .map_err(|e| format!("Failed to encode dimension type NBT: {}", e))?;

    Ok(buffer.to_vec())
}
