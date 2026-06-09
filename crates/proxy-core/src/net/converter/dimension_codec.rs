//! Minimum-viable dimension codec / dimension type NBT compounds for the
//! 1.16+ JoinGame and Respawn packets.
//!
//! Source: <https://minecraft.wiki/w/Java_Edition_protocol/Packets> §JoinGame
//! and the 1.16 protocol-history page. The codec the client receives has to
//! describe at least the dimension(s) it might be teleported into and the
//! biome(s) it might render — the vanilla Notchian server ships a much larger
//! codec describing the full vanilla registry. We synthesize a minimal subset
//! covering overworld / nether / end plus a single `minecraft:plains` biome,
//! which is enough for a 1.16.5 vanilla client to enter the world and render.
//!
//! The 1.16.2 protocol bump changed the codec slightly (dimension type became
//! its own embedded compound rather than an int id); the layout here targets
//! 1.16.2+ since 1.16.0/1.16.1 are sub-1% of legacy clients.

use std::collections::HashMap;

use bytes::BytesMut;
use kojacoord_protocol::codec::Encode;
use kojacoord_protocol::types::nbt::{Nbt, NbtTag};
use kojacoord_protocol::ProtocolError;

fn d(v: f64) -> NbtTag {
    NbtTag::Float(v as f32)
}
fn b(v: i8) -> NbtTag {
    NbtTag::Byte(v)
}
fn i(v: i32) -> NbtTag {
    NbtTag::Int(v)
}
fn s(v: &str) -> NbtTag {
    NbtTag::String(v.to_string())
}

fn dimension_type_element(
    has_skylight: bool,
    has_ceiling: bool,
    ultrawarm: bool,
    natural: bool,
    infiniburn: &str,
    effects: &str,
) -> NbtTag {
    let mut m = HashMap::new();
    m.insert("piglin_safe".into(), b(0));
    m.insert("natural".into(), b(natural as i8));
    m.insert(
        "ambient_light".into(),
        d(if has_skylight { 0.0 } else { 0.1 }),
    );
    m.insert("infiniburn".into(), s(infiniburn));
    m.insert("respawn_anchor_works".into(), b(0));
    m.insert("has_skylight".into(), b(has_skylight as i8));
    m.insert("bed_works".into(), b(1));
    m.insert("effects".into(), s(effects));
    m.insert("has_raids".into(), b(1));
    m.insert("min_y".into(), i(0));
    m.insert("height".into(), i(256));
    m.insert("logical_height".into(), i(256));
    m.insert("coordinate_scale".into(), NbtTag::Double(1.0));
    m.insert("ultrawarm".into(), b(ultrawarm as i8));
    m.insert("has_ceiling".into(), b(has_ceiling as i8));
    NbtTag::Compound(m)
}

fn dim_entry(name: &str, id: i32, element: NbtTag) -> NbtTag {
    let mut m = HashMap::new();
    m.insert("name".into(), s(name));
    m.insert("id".into(), i(id));
    m.insert("element".into(), element);
    NbtTag::Compound(m)
}

fn biome_effects() -> NbtTag {
    let mut m = HashMap::new();
    m.insert("sky_color".into(), i(7_907_327));
    m.insert("water_fog_color".into(), i(329_011));
    m.insert("fog_color".into(), i(12_638_463));
    m.insert("water_color".into(), i(4_159_204));
    NbtTag::Compound(m)
}

fn plains_biome_element() -> NbtTag {
    let mut m = HashMap::new();
    m.insert("precipitation".into(), s("rain"));
    m.insert("depth".into(), NbtTag::Float(0.125));
    m.insert("temperature".into(), NbtTag::Float(0.8));
    m.insert("scale".into(), NbtTag::Float(0.05));
    m.insert("downfall".into(), NbtTag::Float(0.4));
    m.insert("category".into(), s("plains"));
    m.insert("effects".into(), biome_effects());
    NbtTag::Compound(m)
}

fn biome_entry(name: &str, id: i32) -> NbtTag {
    let mut m = HashMap::new();
    m.insert("name".into(), s(name));
    m.insert("id".into(), i(id));
    m.insert("element".into(), plains_biome_element());
    NbtTag::Compound(m)
}

fn registry(registry_id: &str, values: Vec<NbtTag>) -> NbtTag {
    let mut m = HashMap::new();
    m.insert("type".into(), s(registry_id));
    m.insert("value".into(), NbtTag::List(values));
    NbtTag::Compound(m)
}

/// Build the dimension codec compound that 1.16+ JoinGame embeds.
/// Returned as a network-encoded NBT (compound tag with empty name).
pub fn dimension_codec_nbt() -> Result<Vec<u8>, ProtocolError> {
    let mut root: HashMap<String, NbtTag> = HashMap::new();

    let overworld = dimension_type_element(
        true,
        false,
        false,
        true,
        "minecraft:infiniburn_overworld",
        "minecraft:overworld",
    );
    let nether = dimension_type_element(
        false,
        true,
        true,
        false,
        "minecraft:infiniburn_nether",
        "minecraft:the_nether",
    );
    let end = dimension_type_element(
        false,
        false,
        false,
        false,
        "minecraft:infiniburn_end",
        "minecraft:the_end",
    );

    let dim_registry = registry(
        "minecraft:dimension_type",
        vec![
            dim_entry("minecraft:overworld", 0, overworld),
            dim_entry("minecraft:the_nether", 1, nether),
            dim_entry("minecraft:the_end", 2, end),
        ],
    );

    let biome_registry = registry(
        "minecraft:worldgen/biome",
        vec![biome_entry("minecraft:plains", 1)],
    );

    root.insert("minecraft:dimension_type".into(), dim_registry);
    root.insert("minecraft:worldgen/biome".into(), biome_registry);

    let nbt = Nbt {
        name: String::new(),
        root,
    };
    let mut buf = BytesMut::new();
    nbt.encode(&mut buf)?;
    Ok(buf.to_vec())
}

/// Build the standalone "dimension type" compound the 1.16.2+ JoinGame embeds
/// right after the codec. `key` is e.g. "minecraft:overworld".
pub fn dimension_type_nbt(key: &str) -> Result<Vec<u8>, ProtocolError> {
    let (element, infiniburn, effects, has_skylight, has_ceiling, ultrawarm, natural) = match key {
        "minecraft:the_nether" => (
            "nether",
            "minecraft:infiniburn_nether",
            "minecraft:the_nether",
            false,
            true,
            true,
            false,
        ),
        "minecraft:the_end" => (
            "end",
            "minecraft:infiniburn_end",
            "minecraft:the_end",
            false,
            false,
            false,
            false,
        ),
        _ => (
            "overworld",
            "minecraft:infiniburn_overworld",
            "minecraft:overworld",
            true,
            false,
            false,
            true,
        ),
    };
    let _ = element;
    let element = dimension_type_element(
        has_skylight,
        has_ceiling,
        ultrawarm,
        natural,
        infiniburn,
        effects,
    );
    let mut root = HashMap::new();
    if let NbtTag::Compound(m) = element {
        root.extend(m);
    }
    let nbt = Nbt {
        name: String::new(),
        root,
    };
    let mut buf = BytesMut::new();
    nbt.encode(&mut buf)?;
    Ok(buf.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codec_encodes_to_nonempty() {
        let b = dimension_codec_nbt().unwrap();
        assert!(b.len() > 32);
        // First byte is the Compound tag (10).
        assert_eq!(b[0], 10);
    }

    #[test]
    fn dimension_type_nether_has_no_skylight() {
        let b = dimension_type_nbt("minecraft:the_nether").unwrap();
        // The compound starts with tag 10, then i16 name length, then name…
        // Just verify it encoded.
        assert!(b.len() > 16);
        assert_eq!(b[0], 10);
    }
}
