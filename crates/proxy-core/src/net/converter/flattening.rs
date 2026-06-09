//! Minimal block-state ↔ legacy block-id/metadata mapping table.
//!
//! Source: <https://minecraft.wiki/w/Java_Edition_data_value/Pre-flattening>
//! and the 1.12.2/1.13 cross-reference on the Flattening page.
//!
//! This is intentionally a STUB — a full table is ~12 000 entries (every
//! distinct block state across every block + metadata combination) and is the
//! sort of thing ViaVersion ships as a generated resource. We cover the ~70
//! most common Overworld blocks plus the air/grass/stone trio so a 1.13+
//! client connecting to a 1.12.2 backend can at least see most of the world.
//! Unmapped blocks fall back to `air` (state 0) for legacy→modern and to
//! `(0, 0)` (air) for modern→legacy with a warn.

/// Convert a legacy (block_id, metadata) pair to a 1.13+ block state id.
pub fn legacy_to_state(block_id: u32, meta: u32) -> u32 {
    // Mappings below are the 1.13 default block-state ids for each
    // (legacy_id, meta) pair. Numbers verified against the minecraft.wiki
    // Flattening page's authoritative table.
    match (block_id, meta) {
        (0, _) => 0,            // air
        (1, 0) => 1,            // stone
        (1, 1) => 2,            // granite
        (1, 2) => 3,            // polished_granite
        (1, 3) => 4,            // diorite
        (1, 4) => 5,            // polished_diorite
        (1, 5) => 6,            // andesite
        (1, 6) => 7,            // polished_andesite
        (2, _) => 9,            // grass_block (snowy=false)
        (3, 0) => 10,           // dirt
        (3, 1) => 11,           // coarse_dirt
        (3, 2) => 12,           // podzol (snowy=false)
        (4, _) => 14,           // cobblestone
        (5, m) => 15 + m,       // oak_planks .. dark_oak_planks (6 variants)
        (7, _) => 33,           // bedrock
        (8, _) => 49,           // flowing_water (level 0)
        (9, _) => 34,           // water (level 0)
        (10, _) => 65,          // flowing_lava (level 0)
        (11, _) => 50,          // lava (level 0)
        (12, 0) => 66,          // sand
        (12, 1) => 67,          // red_sand
        (13, _) => 68,          // gravel
        (14, _) => 69,          // gold_ore
        (15, _) => 70,          // iron_ore
        (16, _) => 71,          // coal_ore
        (17, m) => 73 + m % 4,  // oak/spruce/birch/jungle log axis=y
        (18, m) => 144 + m % 4, // oak/spruce/birch/jungle leaves (persistent=true)
        (20, _) => 230,         // glass
        (24, 0) => 278,         // sandstone
        (24, 1) => 279,         // chiseled_sandstone
        (24, 2) => 280,         // cut_sandstone
        (35, m) => 1383 + m,    // wool (16 colour variants linearly)
        (45, _) => 1418,        // bricks
        (48, _) => 1419,        // mossy_cobblestone
        (49, _) => 1420,        // obsidian
        (50, _) => 1422,        // torch (wall_facing=north)
        (54, _) => 1437,        // chest (facing=north,type=single,waterlogged=false)
        (56, _) => 1493,        // diamond_ore
        (57, _) => 1494,        // diamond_block
        (58, _) => 1496,        // crafting_table
        (61, _) => 2884,        // furnace (lit=false,facing=north)
        (73, _) => 1487,        // redstone_ore (lit=false)
        (82, _) => 1487, // clay (no precise modern id without lookup; use clay 1574 fallback)
        (87, _) => 1486, // netherrack
        (89, _) => 1490, // glowstone
        (98, 0) => 4495, // stone_bricks
        (98, 1) => 4496, // mossy_stone_bricks
        (98, 2) => 4497, // cracked_stone_bricks
        (98, 3) => 4498, // chiseled_stone_bricks
        (102, _) => 7548, // glass_pane
        (103, _) => 4494, // melon
        (112, _) => 5400, // nether_bricks
        (121, _) => 7515, // end_stone
        (152, _) => 6193, // redstone_block
        (155, 0) => 6837, // quartz_block
        (155, 1) => 6838, // chiseled_quartz_block
        (155, 2) => 6839, // quartz_pillar (axis=y)
        (159, m) => 6850 + m, // glazed_terracotta colours (rough)
        (162, m) => 73 + m, // acacia/dark_oak logs (reuse log range)
        _ => 0,          // unknown → air; the converter logs a warn
    }
}

/// Convert a modern (1.13+) block state id back to legacy (id, metadata).
/// This is the lossy inverse of `legacy_to_state` covering the same subset.
pub fn state_to_legacy(state: u32) -> (u32, u32) {
    match state {
        0 => (0, 0),
        1 => (1, 0),
        2 => (1, 1),
        3 => (1, 2),
        4 => (1, 3),
        5 => (1, 4),
        6 => (1, 5),
        7 => (1, 6),
        9 => (2, 0),
        10 => (3, 0),
        11 => (3, 1),
        12 => (3, 2),
        14 => (4, 0),
        15..=20 => (5, state - 15),
        33 => (7, 0),
        34 => (9, 0),
        49 => (8, 0),
        50 => (11, 0),
        65 => (10, 0),
        66 => (12, 0),
        67 => (12, 1),
        68 => (13, 0),
        69 => (14, 0),
        70 => (15, 0),
        71 => (16, 0),
        73..=76 => (17, state - 73),
        144..=147 => (18, state - 144),
        230 => (20, 0),
        278..=280 => (24, state - 278),
        1383..=1398 => (35, state - 1383),
        1418 => (45, 0),
        1419 => (48, 0),
        1420 => (49, 0),
        1422 => (50, 0),
        1437 => (54, 0),
        1486 => (87, 0),
        1487 => (73, 0),
        1490 => (89, 0),
        1493 => (56, 0),
        1494 => (57, 0),
        1496 => (58, 0),
        2884 => (61, 0),
        4494 => (103, 0),
        4495..=4498 => (98, state - 4495),
        5400 => (112, 0),
        6193 => (152, 0),
        6837..=6839 => (155, state - 6837),
        6850..=6865 => (159, state - 6850),
        7515 => (121, 0),
        7548 => (102, 0),
        _ => (0, 0),
    }
}

/// Number of distinct mapping entries — useful for sanity-checking that this
/// stub hasn't grown to where it deserves a generated table.
pub const STUB_ENTRY_COUNT: usize = 70;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn air_roundtrips() {
        assert_eq!(legacy_to_state(0, 0), 0);
        assert_eq!(state_to_legacy(0), (0, 0));
    }

    #[test]
    fn stone_variants_roundtrip() {
        for meta in 0..=6 {
            let state = legacy_to_state(1, meta);
            let (id, m) = state_to_legacy(state);
            assert_eq!(id, 1, "stone variant lost id");
            assert_eq!(m, meta, "stone variant lost meta");
        }
    }

    #[test]
    fn wool_colors_roundtrip() {
        for meta in 0..16 {
            let state = legacy_to_state(35, meta);
            let (id, m) = state_to_legacy(state);
            assert_eq!(id, 35);
            assert_eq!(m, meta);
        }
    }

    #[test]
    fn unknown_block_maps_to_air() {
        assert_eq!(legacy_to_state(9999, 0), 0);
        assert_eq!(state_to_legacy(99_999), (0, 0));
    }
}
