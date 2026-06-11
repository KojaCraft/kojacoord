//! Generator for `data/block_flattening.toml` and `data/item_flattening.toml`.
//!
//! Reads `PrismarineJS/minecraft-data` JSON and emits the compact TOML
//! format that the runtime loader in `crates::types::flattening` expects.
//!
//! ## Matching strategy
//!
//! Earlier revisions of this generator matched legacy and modern blocks
//! by `displayName`. That's brittle — names diverge between releases
//! (typos, renames, "X Block" vs "Block of X") and silently dropped ~400
//! variations. We now match against the stable `name` field (the
//! snake_case registry identifier, e.g. `stone`, `granite`,
//! `coarse_dirt`) which is preserved across releases. For legacy
//! *variations* (which carry only a `displayName` in the 1.12 dataset,
//! no `name`), we normalise the display name into snake_case and look
//! that up in the modern index.
//!
//! The state id we record is `defaultState` rather than `minStateId`:
//! e.g. `oak_log` has `minStateId=72` (axis=x) but `defaultState=73`
//! (axis=y), and the latter is the canonical "place a log" state.
//!
//! ## Source
//!
//! Download `PrismarineJS/minecraft-data` somewhere local and point
//! `--data` at its `data/pc` tree. The generator inherits across patch
//! versions (1.12.2 → 1.12.1 → 1.12, etc.) since minecraft-data only
//! stores the diff per patch.
//!
//! ## Usage
//!
//! ```bash
//!   cargo run -p kojacoord-protocol --bin gen_flattening -- \
//!       --data /path/to/minecraft-data/data/pc \
//!       --out  crates/protocol/data
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(serde::Deserialize)]
struct LegacyBlock {
    id: u32,
    name: String,
    #[serde(rename = "displayName")]
    display_name: String,
    #[serde(default)]
    variations: Vec<Variation>,
}

#[derive(serde::Deserialize)]
struct Variation {
    metadata: u32,
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(serde::Deserialize)]
struct ModernBlock {
    name: String,
    #[serde(rename = "displayName")]
    display_name: String,
    /// Canonical state for "place this block as-is" — preferred over
    /// `minStateId`, which is sometimes a non-default orientation.
    #[serde(rename = "defaultState")]
    default_state: Option<u32>,
    /// Fallback if `defaultState` is missing (older snapshots).
    #[serde(rename = "minStateId")]
    min_state_id: Option<u32>,
}

impl ModernBlock {
    fn state(&self) -> Option<u32> {
        self.default_state.or(self.min_state_id)
    }
}

#[derive(serde::Deserialize)]
struct ItemEntry {
    id: i32,
    name: String,
    #[serde(rename = "displayName")]
    display_name: String,
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut data_dir = PathBuf::from("./minecraft-data");
    let mut out_dir = PathBuf::from("crates/protocol/data");
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--data" => {
                data_dir = PathBuf::from(&args[i + 1]);
                i += 2;
            },
            "--out" => {
                out_dir = PathBuf::from(&args[i + 1]);
                i += 2;
            },
            "-h" | "--help" => {
                println!("usage: gen_flattening [--data DIR] [--out DIR]");
                return Ok(());
            },
            other => anyhow::bail!("unknown argument: {other}"),
        }
    }

    eprintln!("reading from {}", data_dir.display());
    eprintln!("writing to   {}", out_dir.display());

    gen_blocks(&data_dir, &out_dir)?;
    gen_items(&data_dir, &out_dir)?;

    Ok(())
}

/// minecraft-data inherits data across patch versions via `dataPaths.json` —
/// e.g. `1.12.2` reuses `1.12`'s `blocks.json`. We don't parse the index;
/// we just walk the obvious fallbacks.
fn find_data(data_dir: &Path, candidates: &[&str], file: &str) -> anyhow::Result<PathBuf> {
    for v in candidates {
        let p = data_dir.join(v).join(file);
        if p.exists() {
            return Ok(p);
        }
    }
    anyhow::bail!(
        "could not find {file} under any of {candidates:?} in {}",
        data_dir.display()
    )
}

/// Hand-curated rename table for 1.12 → 1.13 block names that didn't carry
/// through unchanged. Source: minecraft.wiki's per-block "Renamed in 1.13"
/// notes and the Flattening changelog. We try this *after* a direct
/// `name`/`snake(displayName)` lookup, so it only fires when the simple
/// match misses.
fn alias_modern_name(legacy: &str) -> Option<&'static str> {
    Some(match legacy {
        // Direct renames documented on minecraft.wiki/w/Java_Edition_1.13/Flattening.
        "flowing_water" => "water",
        "flowing_lava" => "lava",
        "web" => "cobweb",
        "deadbush" => "dead_bush",
        "yellow_flower" => "dandelion",
        "noteblock" => "note_block",
        "lit_pumpkin" => "jack_o_lantern",
        "magma" => "magma_block",
        "red_nether_brick" => "red_nether_bricks",
        "end_bricks" => "end_stone_bricks",
        "snow_layer" => "snow",
        "snow" => "snow_block",
        "trapdoor" => "oak_trapdoor",
        "sapling" => "oak_sapling",
        "tallgrass" => "grass",
        "leaves" => "oak_leaves",
        "log" => "oak_log",
        // In 1.13 the inverted DSD became a `daylight_detector` blockstate
        // (inverted=true). Without state-level repack we collapse to the
        // base block — better than dropping the cell entirely.
        "daylight_detector_inverted" => "daylight_detector",
        // 1.12 had separate piston_extension/piston_head; 1.13 merged head
        // shapes onto a single block.
        "piston_extension" => "moving_piston",
        // 1.12's "Bed" (id=26) carried color in the tile entity; the meta
        // field only encoded rotation/foot. We can only pick one color
        // for the flat mapping — default to red, the vanilla creative bed.
        "bed" => "red_bed",
        // Banners likewise — color was tile-entity, meta was rotation.
        "standing_banner" => "white_banner",
        "wall_banner" => "white_wall_banner",
        // Skulls/heads similarly were tile-entity coloured.
        "skull" => "skeleton_skull",
        // Pre-1.13 had separate "double" slab blocks for accounting purposes;
        // 1.13 dropped them. Map to the regular slab as a degraded fallback.
        "double_stone_slab" => "smooth_stone_slab",
        "double_stone_slab2" => "red_sandstone_slab",
        "double_wooden_slab" => "oak_slab",
        "purpur_double_slab" => "purpur_slab",
        // Wheat-like crops: "X_seeds" was the planted block in 1.12; 1.13
        // gave each a dedicated name.
        "beetroot_seeds" => "beetroots",
        "carrots" => "carrots",
        "potatoes" => "potatoes",
        "wheat" => "wheat",

        // ── Base-block renames ──
        "brick_block" => "bricks",
        "fence" => "oak_fence",
        "wooden_trapdoor" => "oak_trapdoor",
        "wooden_pressure_plate" => "oak_pressure_plate",
        "button" => "oak_button",
        "stone_brick" => "stone_bricks",
        "chiseled_stone_brick" => "chiseled_stone_bricks",
        "cracked_stone_brick" => "cracked_stone_bricks",
        "mossy_stone_brick" => "mossy_stone_bricks",
        "nether_brick" => "nether_bricks",
        "melon_block" => "melon",
        "reeds" => "sugar_cane",
        "waterlily" => "lily_pad",
        "mob_spawner" => "spawner",
        "pillar_quartz_block" => "quartz_pillar",
        "lit_redstone_lamp" => "redstone_lamp",
        "lit_redstone_ore" => "redstone_ore",
        "quartz_ore" => "nether_quartz_ore",
        "slime" => "slime_block",
        "portal" => "nether_portal",
        "burning_furnace" => "furnace",
        "powered_comparator" => "comparator",
        "daylight_sensor" => "daylight_detector",

        // ── Anvil damage levels ──
        "slightly_damaged_anvil" => "chipped_anvil",
        "very_damaged_anvil" => "damaged_anvil",

        // ── Stained clay → terracotta (1.13's "Stained Hardened Clay" rename) ──
        "white_hardened_clay" => "white_terracotta",
        "orange_hardened_clay" => "orange_terracotta",
        "magenta_hardened_clay" => "magenta_terracotta",
        "light_blue_hardened_clay" => "light_blue_terracotta",
        "yellow_hardened_clay" => "yellow_terracotta",
        "lime_hardened_clay" => "lime_terracotta",
        "pink_hardened_clay" => "pink_terracotta",
        "gray_hardened_clay" => "gray_terracotta",
        "light_gray_hardened_clay" => "light_gray_terracotta",
        "cyan_hardened_clay" => "cyan_terracotta",
        "purple_hardened_clay" => "purple_terracotta",
        "blue_hardened_clay" => "blue_terracotta",
        "brown_hardened_clay" => "brown_terracotta",
        "green_hardened_clay" => "green_terracotta",
        "red_hardened_clay" => "red_terracotta",
        "black_hardened_clay" => "black_terracotta",

        // ── Stone slabs lost the "_stone_" infix in 1.13 ──
        "stone_slab" => "smooth_stone_slab",
        "wooden_slab" => "oak_slab",
        "smooth_double_stone_slab" => "smooth_stone_slab",
        "smooth_double_sandstone_slab" => "smooth_sandstone_slab",
        "upper_red_sandstone_slab" => "red_sandstone_slab",

        // ── Bricks → bricks_slab ──
        "bricks_slab" => "brick_slab",
        "double_bricks_slab" => "brick_slab",

        // ── "X Wood Stairs" → "X Stairs" ──
        "oak_wood_stairs" => "oak_stairs",
        "spruce_wood_stairs" => "spruce_stairs",
        "birch_wood_stairs" => "birch_stairs",
        "jungle_wood_stairs" => "jungle_stairs",
        "acacia_wood_stairs" => "acacia_stairs",
        "dark_oak_wood_stairs" => "dark_oak_stairs",

        // ── "X Wood Slab" / "Double X Wood Slab" → "X Slab" ──
        "oak_wood_slab" | "double_oak_wood_slab" => "oak_slab",
        "spruce_wood_slab" | "double_spruce_wood_slab" => "spruce_slab",
        "birch_wood_slab" | "double_birch_wood_slab" => "birch_slab",
        "jungle_wood_slab" | "double_jungle_wood_slab" => "jungle_slab",
        "acacia_wood_slab" | "double_acacia_wood_slab" => "acacia_slab",
        "dark_oak_wood_slab" | "double_dark_oak_wood_slab" => "dark_oak_slab",

        // ── Other "Tile / Pillar Quartz" double slabs ──
        "tile_double_quartz_slab" => "quartz_slab",

        // ── Weighted pressure plates ──
        "weighted_pressure_plate_light" => "light_weighted_pressure_plate",
        "weighted_pressure_plate_heavy" => "heavy_weighted_pressure_plate",

        // ── Monster eggs (silverfish stones) ──
        "stone_monster_egg" => "infested_stone",
        "cobblestone_monster_egg" => "infested_cobblestone",
        "stone_brick_monster_egg" => "infested_stone_bricks",
        "mossy_stone_brick_monster_egg" => "infested_mossy_stone_bricks",
        "cracked_stone_brick_monster_egg" => "infested_cracked_stone_bricks",
        "chiseled_stone_brick_monster_egg" => "infested_chiseled_stone_bricks",

        // ── Tall plants & signs ──
        "double_tallgrass" => "tall_grass",
        "standing_sign" => "sign", // 1.13 used the bare `sign` until 1.14 split it per wood.
        "head" => "skeleton_skull",
        // Brown/red "Mushroom" base entries fall to the named blocks.
        "mushroom" => "brown_mushroom",
        "potato" => "potatoes",
        "carrot" => "carrots",
        "cauldront" => "cauldron",

        // ── Generic "Fence Gate" w/ no wood prefix ──
        "fence_gate" => "oak_fence_gate",

        // ── 1.12 "double slab" variants → flatten to the regular slab.
        // 1.13 keeps a single block that covers both "single" and "double"
        // placements via the `type` blockstate (top/bottom/double).
        "double_sandstone_slab" => "sandstone_slab",
        "double_stone_wooden_slab" => "petrified_oak_slab",
        "stone_wooden_slab" => "petrified_oak_slab",
        "double_cobblestone_slab" => "cobblestone_slab",
        "double_stone_brick_slab" => "stone_brick_slab",
        "double_nether_brick_slab" => "nether_brick_slab",
        "double_quartz_slab" => "quartz_slab",
        "smooth_double_red_sandstone_slab" => "smooth_red_sandstone_slab",

        // ── Inverted DSD ──
        "inverted_daylight_sensor" => "daylight_detector",

        _ => return None,
    })
}

/// Strip the legacy `_wood_planks` suffix → `_planks`. Affects all six wood
/// types (oak, spruce, birch, jungle, acacia, dark_oak).
fn strip_wood_planks(s: &str) -> Option<String> {
    s.strip_suffix("_wood_planks")
        .map(|p| format!("{p}_planks"))
}

/// Normalize a human-readable display name into the snake_case form
/// minecraft-data uses for registry names. "Light blue Wool" → "light_blue_wool".
fn snake_from_display(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_underscore = true;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            for d in c.to_lowercase() {
                out.push(d);
            }
            prev_underscore = false;
        } else if !prev_underscore {
            out.push('_');
            prev_underscore = true;
        }
        // skip other punctuation entirely (apostrophes, parens, etc.)
    }
    while out.ends_with('_') {
        out.pop();
    }
    out
}

fn gen_blocks(data_dir: &Path, out_dir: &Path) -> anyhow::Result<()> {
    let legacy_path = find_data(data_dir, &["1.12.2", "1.12.1", "1.12"], "blocks.json")?;
    let modern_path = find_data(data_dir, &["1.13.2", "1.13.1", "1.13"], "blocks.json")?;
    eprintln!("  legacy blocks: {}", legacy_path.display());
    eprintln!("  modern blocks: {}", modern_path.display());

    let legacy: Vec<LegacyBlock> = serde_json::from_str(&std::fs::read_to_string(&legacy_path)?)?;
    let modern: Vec<ModernBlock> = serde_json::from_str(&std::fs::read_to_string(&modern_path)?)?;

    // Index modern by stable registry name. Carry a secondary index from
    // display-name → name so we can recover when a 1.12 variant's name
    // didn't carry through (e.g. "Polished Andesite" → polished_andesite).
    let by_name: BTreeMap<String, &ModernBlock> =
        modern.iter().map(|m| (m.name.clone(), m)).collect();
    let by_display_snake: BTreeMap<String, &ModernBlock> = modern
        .iter()
        .map(|m| (snake_from_display(&m.display_name), m))
        .collect();

    // Resolution order: registry name → snake(displayName) → hand-curated
    // alias (`alias_modern_name`) → `_wood_planks` suffix rewrite. Each
    // fallback only fires if the previous ones missed.
    let lookup = |key: &str| -> Option<&ModernBlock> {
        if let Some(m) = by_name.get(key).copied() {
            return Some(m);
        }
        if let Some(m) = by_display_snake.get(key).copied() {
            return Some(m);
        }
        if let Some(alias) = alias_modern_name(key) {
            if let Some(m) = by_name.get(alias).copied() {
                return Some(m);
            }
        }
        if let Some(rewritten) = strip_wood_planks(key) {
            if let Some(m) = by_name.get(&rewritten).copied() {
                return Some(m);
            }
        }
        None
    };

    let mut out = String::new();
    out.push_str(
        "# Block flattening table — bidirectional mapping between pre-1.13\n\
         # (legacy_id, meta) pairs and 1.13+ flattened state IDs.\n\
         #\n\
         # Each entry is [legacy_id, legacy_meta, modern_state_id, optional_name].\n\
         # AUTO-GENERATED by `cargo run -p kojacoord-protocol --bin gen_flattening`.\n\
         # Hand edits will be overwritten — change the upstream data or the\n\
         # generator if you need different mappings.\n\n\
         blocks = [\n",
    );

    let mut written = 0usize;
    let mut skipped: Vec<String> = Vec::new();

    for b in &legacy {
        if b.variations.is_empty() {
            // No metadata variants — match the base block by registry name.
            match lookup(&b.name).and_then(|m| m.state()) {
                Some(state) => {
                    out.push_str(&format!(
                        "  [{}, 0, {}, \"{}\"],\n",
                        b.id,
                        state,
                        escape(&b.display_name)
                    ));
                    written += 1;
                },
                None => skipped.push(format!("{} (id={}, no variants)", b.name, b.id)),
            }
            continue;
        }

        for v in &b.variations {
            if v.metadata >= 16 {
                // Meta is a 4-bit field on the wire; anything ≥16 would
                // collide with the block id when packed.
                skipped.push(format!(
                    "{} meta={} (out of range)",
                    v.display_name, v.metadata
                ));
                continue;
            }

            // Try in order: snake(displayName) → modern name; bare base
            // name + meta-0 (skips trivial duplicates).
            let key = snake_from_display(&v.display_name);
            let modern = lookup(&key).or_else(|| {
                // Last-resort: the base block itself, only if meta=0
                // (the "default" variant).
                if v.metadata == 0 {
                    lookup(&b.name)
                } else {
                    None
                }
            });

            match modern.and_then(|m| m.state()) {
                Some(state) => {
                    out.push_str(&format!(
                        "  [{}, {}, {}, \"{}\"],\n",
                        b.id,
                        v.metadata,
                        state,
                        escape(&v.display_name),
                    ));
                    written += 1;
                },
                None => skipped.push(format!(
                    "{} meta={} (no modern match for `{key}`)",
                    v.display_name, v.metadata
                )),
            }
        }
    }
    out.push_str("]\n");

    let path = out_dir.join("block_flattening.toml");
    std::fs::write(&path, out)?;
    eprintln!(
        "wrote {} ({} entries, {} skipped)",
        path.display(),
        written,
        skipped.len()
    );
    if !skipped.is_empty() && std::env::var("KOJA_GEN_VERBOSE").is_ok() {
        for s in &skipped {
            eprintln!("  skipped: {s}");
        }
    } else if !skipped.is_empty() {
        eprintln!("  (set KOJA_GEN_VERBOSE=1 to see the skipped list)");
    }
    Ok(())
}

/// Item-specific 1.12 → 1.13 renames. Most overlap with the block table
/// (since block-placement items share the block id), but a handful of
/// inventory-only items (spawn eggs, music discs, etc.) need their own
/// entries.
fn alias_modern_item(legacy: &str) -> Option<&'static str> {
    // First try the block alias table — most placeable items map the
    // same way (e.g. `noteblock` item → `note_block` item).
    if let Some(via_block) = alias_modern_name(legacy) {
        return Some(via_block);
    }
    Some(match legacy {
        // Items that were renamed without changing the block id.
        "record_13" => "music_disc_13",
        "record_cat" => "music_disc_cat",
        "record_blocks" => "music_disc_blocks",
        "record_chirp" => "music_disc_chirp",
        "record_far" => "music_disc_far",
        "record_mall" => "music_disc_mall",
        "record_mellohi" => "music_disc_mellohi",
        "record_stal" => "music_disc_stal",
        "record_strad" => "music_disc_strad",
        "record_ward" => "music_disc_ward",
        "record_11" => "music_disc_11",
        "record_wait" => "music_disc_wait",
        // Inventory-only renames documented in the Flattening notes.
        "fish" => "cod",
        "cooked_fish" => "cooked_cod",
        "experience_bottle" => "experience_bottle",
        "fireworks" => "firework_rocket",
        "firework_charge" => "firework_star",
        "speckled_melon" => "glistering_melon_slice",
        "melon" => "melon_slice",
        "netherbrick" => "nether_brick",
        "leather_helmet" => "leather_helmet",
        "skull" => "skeleton_skull",
        // Spawn eggs collapsed into one item id in 1.13; pick the
        // generic pig egg as a safe default for the bare `spawn_egg`.
        "spawn_egg" => "pig_spawn_egg",
        // 1.12 had two leaf items differing only by metadata; the second
        // (id=162 / "leaves2") covers acacia/dark_oak. Without a meta
        // round-trip we collapse both to oak_leaves as the default.
        "log2" | "leaves2" => "oak_leaves",
        "stained_hardened_clay" => "white_terracotta",
        _ => return None,
    })
}

fn gen_items(data_dir: &Path, out_dir: &Path) -> anyhow::Result<()> {
    let legacy_path = find_data(data_dir, &["1.12.2", "1.12.1", "1.12"], "items.json")?;
    let modern_path = find_data(data_dir, &["1.13.2", "1.13.1", "1.13"], "items.json")?;
    eprintln!("  legacy items: {}", legacy_path.display());
    eprintln!("  modern items: {}", modern_path.display());

    let legacy: Vec<ItemEntry> = serde_json::from_str(&std::fs::read_to_string(&legacy_path)?)?;
    let modern: Vec<ItemEntry> = serde_json::from_str(&std::fs::read_to_string(&modern_path)?)?;

    let modern_by_name: BTreeMap<String, &ItemEntry> =
        modern.iter().map(|m| (m.name.clone(), m)).collect();
    let modern_by_display: BTreeMap<String, &ItemEntry> = modern
        .iter()
        .map(|m| (snake_from_display(&m.display_name), m))
        .collect();

    // Resolution chain mirrors the block lookup: direct name →
    // snake(display) → curated alias table.
    let lookup = |name: &str, display: &str| -> Option<&ItemEntry> {
        if let Some(m) = modern_by_name.get(name).copied() {
            return Some(m);
        }
        let dkey = snake_from_display(display);
        if let Some(m) = modern_by_display.get(&dkey).copied() {
            return Some(m);
        }
        if let Some(alias) = alias_modern_item(name) {
            if let Some(m) = modern_by_name.get(alias).copied() {
                return Some(m);
            }
        }
        None
    };

    let mut out = String::new();
    out.push_str(
        "# Item flattening table — pre-1.13 → 1.13+ item ID mapping.\n\
         # AUTO-GENERATED by `cargo run -p kojacoord-protocol --bin gen_flattening`.\n\n",
    );

    let mut entries = String::new();
    let mut written = 0usize;
    let mut skipped: Vec<String> = Vec::new();
    for l in &legacy {
        match lookup(&l.name, &l.display_name) {
            Some(m) if l.id >= i16::MIN as i32 && l.id <= i16::MAX as i32 && m.id >= 0 => {
                entries.push_str(&format!(
                    "  [{}, {}, \"{}\"],\n",
                    l.id,
                    m.id,
                    escape(&l.display_name),
                ));
                written += 1;
            },
            _ => skipped.push(format!("{} (id={})", l.name, l.id)),
        }
    }

    out.push_str("items = [\n");
    out.push_str(&entries);
    out.push_str("]\n");

    let path = out_dir.join("item_flattening.toml");
    std::fs::write(&path, out)?;
    eprintln!(
        "wrote {} ({} entries, {} skipped)",
        path.display(),
        written,
        skipped.len()
    );
    if !skipped.is_empty() && std::env::var("KOJA_GEN_VERBOSE").is_ok() {
        for s in &skipped {
            eprintln!("  item skipped: {s}");
        }
    }
    Ok(())
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
