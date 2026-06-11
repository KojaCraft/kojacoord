//! Per-era chunk data types.
//!
//! Chunks changed wire layout four times in Java Edition's history:
//! the 1.13 flattening (numeric block id → varint state id), the 1.14
//! biome rewrite (16×16 → palette per section), and the 1.18 height
//! rework (Y range stretched, sections renumbered). This module just
//! has the parsed structs — the actual repack logic lives in
//! `proxy_core::net::converter::chunk_repack`.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkFormat {
    /// Pre-1.13: 8-bit block ids plus 4-bit metadata nibbles.
    Legacy,
    /// 1.13 – 1.13.x: string palette + bits-per-block packed long array.
    Flattened,
    /// 1.14 – 1.17: adds the per-section biome array.
    ModernBiomes,
    /// 1.18+: world height stretched, section list now spans Y from
    /// -64 to 320 instead of 0–255.
    NewHeight,
}

/// Pre-1.13 chunk section. 16×16×16 cube with flat byte arrays for
/// block ids, packed metadata nibbles, and 4-bit light values. Total
/// per section ≈ 10 KiB.
#[derive(Debug, Clone)]
pub struct LegacyChunkSection {
    /// Block IDs (8 bits per block)
    pub blocks: Vec<u8>,
    /// Block metadata (4 bits per block, packed)
    pub metadata: Vec<u8>,
    /// Block light (4 bits per block, packed)
    pub block_light: Vec<u8>,
    /// Sky light (4 bits per block, packed)
    pub sky_light: Vec<u8>,
}

impl LegacyChunkSection {
    pub fn new() -> Self {
        Self {
            blocks: vec![0; 4096],
            metadata: vec![0; 2048],
            block_light: vec![0; 2048],
            sky_light: vec![0; 2048],
        }
    }

    pub fn size(&self) -> usize {
        self.blocks.len() + self.metadata.len() + self.block_light.len() + self.sky_light.len()
    }
}

impl Default for LegacyChunkSection {
    fn default() -> Self {
        Self::new()
    }
}

/// 1.13+ chunk section. The block grid is now indices into a per-section
/// palette of state strings, packed into a long array with a
/// caller-defined bits-per-block. Empty sections share the air-only
/// default palette.
#[derive(Debug, Clone)]
pub struct FlattenedChunkSection {
    /// Block states as palette indices
    pub block_states: Vec<i64>,
    /// Palette mapping indices to block state strings
    pub palette: Vec<String>,
    /// Block light
    pub block_light: Vec<u8>,
    /// Sky light
    pub sky_light: Vec<u8>,
}

impl FlattenedChunkSection {
    pub fn new() -> Self {
        Self {
            block_states: vec![0; 256],
            palette: vec!["minecraft:air".to_string()],
            block_light: vec![0; 2048],
            sky_light: vec![0; 2048],
        }
    }
}

impl Default for FlattenedChunkSection {
    fn default() -> Self {
        Self::new()
    }
}

/// 1.14 – 1.17 chunk section. Adds a flat biome int array alongside
/// the flattened block grid. The 256-entry array is one biome per
/// (x, z) column.
#[derive(Debug, Clone)]
pub struct ModernBiomeChunkSection {
    /// Flattened block states
    pub block_states: FlattenedChunkSection,
    /// Biome IDs (int array)
    pub biomes: Vec<i32>,
}

impl ModernBiomeChunkSection {
    pub fn new() -> Self {
        Self {
            block_states: FlattenedChunkSection::new(),
            biomes: vec![0; 256],
        }
    }
}

impl Default for ModernBiomeChunkSection {
    fn default() -> Self {
        Self::new()
    }
}

/// 1.18+ chunk section. Same block_states shape as 1.13 but the world
/// is now ‑64 to 320 (24 sections instead of 16) and biomes are a 4×4×4
/// 3-D grid (64 entries per section) rather than a 16×16 column array.
#[derive(Debug, Clone)]
pub struct NewHeightChunkSection {
    pub block_states: FlattenedChunkSection,
    /// 4³ biome volume — 64 entries laid out in (y, z, x) order.
    pub biomes: Vec<i32>,
}

impl NewHeightChunkSection {
    pub fn new() -> Self {
        Self {
            block_states: FlattenedChunkSection::new(),
            biomes: vec![0; 64],
        }
    }
}

impl Default for NewHeightChunkSection {
    fn default() -> Self {
        Self::new()
    }
}

/// Tagged union over the four section formats. The parser produces
/// whichever variant matches the source format; the repacker walks
/// the variant on the way out and decides whether/how to translate.
#[derive(Debug, Clone)]
pub enum ChunkData {
    Legacy(Vec<LegacyChunkSection>),
    Flattened(Vec<FlattenedChunkSection>),
    ModernBiomes(Vec<ModernBiomeChunkSection>),
    NewHeight(Vec<NewHeightChunkSection>),
}

impl ChunkData {
    pub fn format(&self) -> ChunkFormat {
        match self {
            ChunkData::Legacy(_) => ChunkFormat::Legacy,
            ChunkData::Flattened(_) => ChunkFormat::Flattened,
            ChunkData::ModernBiomes(_) => ChunkFormat::ModernBiomes,
            ChunkData::NewHeight(_) => ChunkFormat::NewHeight,
        }
    }

    pub fn section_count(&self) -> usize {
        match self {
            ChunkData::Legacy(sections) => sections.len(),
            ChunkData::Flattened(sections) => sections.len(),
            ChunkData::ModernBiomes(sections) => sections.len(),
            ChunkData::NewHeight(sections) => sections.len(),
        }
    }
}

/// Pre-1.13 numeric block id ↔ 1.13+ flattened state-string lookup.
/// Only covers ~60 of the ~300 vanilla blocks; the wider data lives in
/// `types::flattening` and is loaded from TOML. This struct is kept
/// for the chunk repacker's hot path where allocating a fresh
/// `BlockFlatteningTable` lookup per call would dominate.
///
/// Unknown ids fall back to `minecraft:air` on the forward path and
/// `0` (air) on the reverse — desyncing one block is better than
/// dropping the whole chunk.
#[derive(Debug, Clone)]
pub struct BlockStateConverter {
    legacy_to_flattened: Vec<String>,
    flattened_to_legacy: HashMap<String, u16>,
}

impl BlockStateConverter {
    pub fn new() -> Self {
        let mut legacy_to_flattened = vec!["minecraft:air".to_string(); 4096];
        let mut flattened_to_legacy = HashMap::new();

        // Initialize with common block mappings
        // Production would have full 300+ entry table from wiki.vg
        legacy_to_flattened[0] = "minecraft:air".to_string();
        legacy_to_flattened[1] = "minecraft:stone".to_string();
        legacy_to_flattened[2] = "minecraft:grass_block".to_string();
        legacy_to_flattened[3] = "minecraft:dirt".to_string();
        legacy_to_flattened[4] = "minecraft:cobblestone".to_string();
        legacy_to_flattened[5] = "minecraft:oak_planks".to_string();
        legacy_to_flattened[6] = "minecraft:spruce_planks".to_string();
        legacy_to_flattened[7] = "minecraft:birch_planks".to_string();
        legacy_to_flattened[8] = "minecraft:jungle_planks".to_string();
        legacy_to_flattened[9] = "minecraft:acacia_planks".to_string();
        legacy_to_flattened[10] = "minecraft:dark_oak_planks".to_string();
        legacy_to_flattened[11] = "minecraft:bedrock".to_string();
        legacy_to_flattened[12] = "minecraft:water".to_string();
        legacy_to_flattened[13] = "minecraft:lava".to_string();
        legacy_to_flattened[14] = "minecraft:sand".to_string();
        legacy_to_flattened[15] = "minecraft:gravel".to_string();
        legacy_to_flattened[16] = "minecraft:gold_ore".to_string();
        legacy_to_flattened[17] = "minecraft:iron_ore".to_string();
        legacy_to_flattened[18] = "minecraft:coal_ore".to_string();
        legacy_to_flattened[19] = "minecraft:oak_log".to_string();
        legacy_to_flattened[20] = "minecraft:spruce_log".to_string();
        legacy_to_flattened[21] = "minecraft:birch_log".to_string();
        legacy_to_flattened[22] = "minecraft:jungle_log".to_string();
        legacy_to_flattened[23] = "minecraft:acacia_log".to_string();
        legacy_to_flattened[24] = "minecraft:dark_oak_log".to_string();
        legacy_to_flattened[25] = "minecraft:oak_leaves".to_string();
        legacy_to_flattened[26] = "minecraft:spruce_leaves".to_string();
        legacy_to_flattened[27] = "minecraft:birch_leaves".to_string();
        legacy_to_flattened[28] = "minecraft:jungle_leaves".to_string();
        legacy_to_flattened[29] = "minecraft:acacia_leaves".to_string();
        legacy_to_flattened[30] = "minecraft:dark_oak_leaves".to_string();
        legacy_to_flattened[31] = "minecraft:glass".to_string();
        legacy_to_flattened[32] = "minecraft:lapis_ore".to_string();
        legacy_to_flattened[33] = "minecraft:lapis_block".to_string();
        legacy_to_flattened[34] = "minecraft:sandstone".to_string();
        legacy_to_flattened[35] = "minecraft:note_block".to_string();
        legacy_to_flattened[36] = "minecraft:bed".to_string();
        legacy_to_flattened[37] = "minecraft:powered_rail".to_string();
        legacy_to_flattened[38] = "minecraft:detector_rail".to_string();
        legacy_to_flattened[39] = "minecraft:sticky_piston".to_string();
        legacy_to_flattened[40] = "minecraft:web".to_string();
        legacy_to_flattened[41] = "minecraft:tall_grass".to_string();
        legacy_to_flattened[42] = "minecraft:dead_bush".to_string();
        legacy_to_flattened[43] = "minecraft:piston".to_string();
        legacy_to_flattened[44] = "minecraft:piston_head".to_string();
        legacy_to_flattened[45] = "minecraft:white_wool".to_string();
        legacy_to_flattened[46] = "minecraft:orange_wool".to_string();
        legacy_to_flattened[47] = "minecraft:magenta_wool".to_string();
        legacy_to_flattened[48] = "minecraft:light_blue_wool".to_string();
        legacy_to_flattened[49] = "minecraft:yellow_wool".to_string();
        legacy_to_flattened[50] = "minecraft:lime_wool".to_string();
        legacy_to_flattened[51] = "minecraft:pink_wool".to_string();
        legacy_to_flattened[52] = "minecraft:gray_wool".to_string();
        legacy_to_flattened[53] = "minecraft:light_gray_wool".to_string();
        legacy_to_flattened[54] = "minecraft:cyan_wool".to_string();
        legacy_to_flattened[55] = "minecraft:purple_wool".to_string();
        legacy_to_flattened[56] = "minecraft:blue_wool".to_string();
        legacy_to_flattened[57] = "minecraft:brown_wool".to_string();
        legacy_to_flattened[58] = "minecraft:green_wool".to_string();
        legacy_to_flattened[59] = "minecraft:red_wool".to_string();
        legacy_to_flattened[60] = "minecraft:black_wool".to_string();

        // Build reverse mapping
        for (id, name) in legacy_to_flattened.iter().enumerate() {
            flattened_to_legacy.insert(name.clone(), id as u16);
        }

        Self {
            legacy_to_flattened,
            flattened_to_legacy,
        }
    }

    /// Legacy id → `"minecraft:<name>"`. Returns air for any id past
    /// the table — see the type docs on degraded behaviour.
    pub fn to_flattened(&self, legacy_id: u16) -> String {
        self.legacy_to_flattened
            .get(legacy_id as usize)
            .cloned()
            .unwrap_or_else(|| "minecraft:air".to_string())
    }

    /// Reverse of [`Self::to_flattened`]. Unknown strings collapse to
    /// air (legacy id 0); this is one of the visible failure modes
    /// the operator should expect when bridging 1.13+ → pre-1.13.
    pub fn to_legacy(&self, flattened_id: &str) -> u16 {
        *self.flattened_to_legacy.get(flattened_id).unwrap_or(&0)
    }
}

impl Default for BlockStateConverter {
    fn default() -> Self {
        Self::new()
    }
}

/// Pre-1.16 numeric biome id ↔ 1.16+ flattened registry id. 1.16 was
/// the bigger break here than 1.14 — that's when biomes moved from a
/// global numeric registry to per-world `minecraft:` namespaced ids,
/// pushed through the dimension codec.
#[derive(Debug, Clone)]
pub struct BiomeConverter {
    legacy_to_modern: Vec<i32>,
    modern_to_legacy: HashMap<i32, u8>,
}

impl BiomeConverter {
    pub fn new() -> Self {
        let mut legacy_to_modern = vec![0; 256];
        let mut modern_to_legacy = HashMap::new();

        // Biome mappings from wiki.vg
        legacy_to_modern[0] = 0; // Ocean → ocean
        legacy_to_modern[1] = 5; // Plains → plains
        legacy_to_modern[2] = 3; // Desert → desert
        legacy_to_modern[3] = 6; // Extreme Hills → windswept_hills
        legacy_to_modern[4] = 2; // Forest → forest
        legacy_to_modern[5] = 4; // Taiga → taiga
        legacy_to_modern[6] = 1; // Swampland → swamp
        legacy_to_modern[7] = 7; // River → river
        legacy_to_modern[8] = 8; // Hell → nether
        legacy_to_modern[9] = 9; // Sky → the_end
        legacy_to_modern[10] = 10; // Frozen Ocean → frozen_ocean
        legacy_to_modern[11] = 11; // Frozen River → frozen_river
        legacy_to_modern[12] = 12; // Ice Plains → snowy_plains
        legacy_to_modern[13] = 13; // Ice Mountains → snowy_slopes
        legacy_to_modern[14] = 14; // Mushroom Fields → mushroom_fields
        legacy_to_modern[15] = 15; // Mushroom Field Shore → mushroom_field_shore
        legacy_to_modern[16] = 16; // Beach → beach
        legacy_to_modern[17] = 17; // Desert Hills → desert_hills
        legacy_to_modern[18] = 18; // Forest Hills → wooded_hills
        legacy_to_modern[19] = 19; // Taiga Hills → taiga_hills
        legacy_to_modern[20] = 20; // Extreme Hills Edge → mountain_edge
        legacy_to_modern[21] = 21; // Jungle → jungle
        legacy_to_modern[22] = 22; // Jungle Hills → jungle_hills
        legacy_to_modern[23] = 23; // Jungle Edge → sparse_jungle
        legacy_to_modern[24] = 24; // Deep Ocean → deep_ocean
        legacy_to_modern[25] = 25; // Stone Beach → stony_shore
        legacy_to_modern[26] = 26; // Cold Beach → snowy_beach
        legacy_to_modern[27] = 27; // Birch Forest → birch_forest
        legacy_to_modern[28] = 28; // Birch Forest Hills → birch_forest_hills
        legacy_to_modern[29] = 29; // Roofed Forest → dark_forest
        legacy_to_modern[30] = 30; // Cold Taiga → snowy_taiga
        legacy_to_modern[31] = 31; // Cold Taiga Hills → snowy_taiga_hills
        legacy_to_modern[32] = 32; // Mega Taiga → old_growth_pine_taiga
        legacy_to_modern[33] = 33; // Mega Taiga Hills → old_growth_pine_taiga_hills
        legacy_to_modern[34] = 34; // Extreme Hills+ → windswept_gravelly_hills
        legacy_to_modern[35] = 35; // Savanna → savanna
        legacy_to_modern[36] = 36; // Savanna Plateau → savanna_plateau
        legacy_to_modern[37] = 37; // Mesa → badlands
        legacy_to_modern[38] = 38; // Mesa Plateau F → badlands_plateau
        legacy_to_modern[39] = 39; // Mesa Plateau → wooded_badlands_plateau

        // Build reverse mapping
        for (legacy_id, modern_id) in legacy_to_modern.iter().enumerate() {
            modern_to_legacy.insert(*modern_id, legacy_id as u8);
        }

        Self {
            legacy_to_modern,
            modern_to_legacy,
        }
    }

    /// Convert legacy biome ID to modern biome ID
    pub fn to_modern(&self, legacy_id: u8) -> i32 {
        *self.legacy_to_modern.get(legacy_id as usize).unwrap_or(&0)
    }

    /// Convert modern biome ID to legacy biome ID
    pub fn to_legacy(&self, modern_id: i32) -> u8 {
        *self.modern_to_legacy.get(&modern_id).unwrap_or(&0)
    }
}

impl Default for BiomeConverter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_state_conversion() {
        let converter = BlockStateConverter::new();

        // Test legacy to flattened
        let flattened = converter.to_flattened(1);
        assert_eq!(flattened, "minecraft:stone");

        // Test flattened to legacy
        let legacy = converter.to_legacy("minecraft:stone");
        assert_eq!(legacy, 1);
    }

    #[test]
    fn biome_conversion() {
        let converter = BiomeConverter::new();

        // Test legacy to modern
        let modern = converter.to_modern(1);
        assert_eq!(modern, 5); // Plains

        // Test modern to legacy
        let legacy = converter.to_legacy(5);
        assert_eq!(legacy, 1);
    }
}
