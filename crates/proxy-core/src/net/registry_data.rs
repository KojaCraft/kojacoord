//! Configuration-phase registry data for 1.20.5+ / 1.21 limbo.
//!
//! From 1.20.5 (proto 766) the client CLEARS its registries at the start
//! of the configuration phase and only repopulates them from either
//! (a) a negotiated "known pack" (the `SelectKnownPacks` handshake) or
//! (b) explicit `ClientboundRegistryData` packets. A limbo that sends
//! neither leaves the client with empty `dimension_type` / `worldgen/biome`
//! registries, so the JoinGame `dimension_type = VarInt(0)` reference
//! fails to resolve and the client disconnects. (1.20.2-1.20.4 also need
//! registry data, but in a DIFFERENT wire form — one RegistryData packet
//! carrying the whole codec as a single nameless NBT compound, rather than
//! the per-registry split below. That single-codec form is built by
//! `protocol::config_codec_nameless_for_proto` and sent from
//! `limbo::run_configuration_phase`; `bundle_for_proto` returns None for
//! those protos so this per-registry path stays 766+ only.)
//!
//! We send the full registry set ourselves, captured from PrismarineJS
//! `minecraft-data` `pc/<ver>/loginPacket.json` `dimensionCodec` and
//! converted to per-registry `ClientboundRegistryData` bodies by
//! `tools`/`gen_registries.py`. Each embedded bundle is:
//!
//! ```text
//! [u32 num_registries]
//! repeat num_registries:
//!   [u32 body_len][body]           // body = one RegistryData packet body
//! ```
//!
//! and each `body` is the wire payload of `ClientboundRegistryData`:
//!
//! ```text
//! [String registry_id]
//! [VarInt entry_count]
//! repeat entry_count:
//!   [String entry_key]
//!   [bool has_data]
//!   has_data ? [network NBT: nameless tag id + payload] : ()
//! ```
//!
//! The limbo prepends the proto-correct packet id and frames each body.

/// 1.20.5 / 1.20.6 (proto 766) — 8 registries.
static REGISTRIES_1_20_5: &[u8] =
    include_bytes!("../../../../crates/protocol/data/registries_1_20_5.bin");
/// 1.21 / 1.21.1 (proto 767) — 11 registries (adds painting_variant,
/// enchantment, jukebox_song).
static REGISTRIES_1_21: &[u8] =
    include_bytes!("../../../../crates/protocol/data/registries_1_21.bin");
/// 1.21.2 / 1.21.3 / 1.21.4 (proto 768/769) — 12 registries (adds
/// instrument). 1.21.4 added no synced registries over 1.21.3.
static REGISTRIES_1_21_3: &[u8] =
    include_bytes!("../../../../crates/protocol/data/registries_1_21_3.bin");
/// 1.21.5 (proto 770) — 18 registries: 1.21.3 set + the six mob-variant
/// registries 1.21.5 added (cat/chicken/cow/frog/pig/wolf_sound), per
/// ViaVersion `Protocol1_21_4To1_21_5`. NOTE: dimension_type / worldgen/biome
/// / enchantment / jukebox_song use the 1.21.4 (registries_1_21_3) bodies —
/// 1.21.5 kept that schema (no RegistryDataRewriter addHandler 1.21.4→1.21.5),
/// and the 1.21.11-filtered versions were rejected by the 1.21.5 client
/// (missing has_raids/sky_color, unknown post_piercing_attack, …). biome
/// music is wrapped to the 1.21.4+ weighted list. The six mob-variant bodies
/// keep the 1.21.11-filtered form (schema-stable, validated fine). Rebuilt by
/// `tools/registry-gen/fix_1_21_5.py`.
static REGISTRIES_1_21_5: &[u8] =
    include_bytes!("../../../../crates/protocol/data/registries_1_21_5.bin");
/// 1.21.6 – 1.21.9 (proto 771/772/773) — 19 registries: 1.21.5 set +
/// `dialog` (added 1.21.6 per ViaVersion `Protocol1_21_5To1_21_6`).
/// 1.21.7/1.21.8/1.21.9 added no further synced registries. As with 1.21.5,
/// dimension_type / biome / enchantment / jukebox_song use the correct
/// (1.21.5-rebased) bodies rather than the 1.21.11-filtered ones — 1.21.6 did
/// not change their schema (no addHandler 1.21.5→1.21.6). The 1.21.6 client
/// also requires the `minecraft:dialog` tags (`pause_screen_additions`,
/// `quick_actions`) bound — see [`config_tags_body_for_proto`]. Rebuilt by
/// `tools/registry-gen/rebase_registries.py` from `registries_1_21_5.bin`.
static REGISTRIES_1_21_6: &[u8] =
    include_bytes!("../../../../crates/protocol/data/registries_1_21_6.bin");
/// 1.21.10 / 1.21.11 (proto 774) — full 23-registry set (adds
/// test_environment/test_instance/timeline/zombie_nautilus_variant),
/// captured verbatim from minecraft-data `pc/1.21.11`.
static REGISTRIES_1_21_11: &[u8] =
    include_bytes!("../../../../crates/protocol/data/registries_1_21_11.bin");
/// 26.1 (proto 775) — 28 registries: the 1.21.11 set with the
/// fields 26.1 made required injected (dimension_type has_ender_dragon_fight,
/// mob-variant baby_asset_id, wolf_variant baby_assets +"_baby",
/// wolf_sound_variant adult_sounds/baby_sounds with constant
/// step_sound=entity.wolf.step, timeline clock), PLUS the new
/// minecraft:world_clock registry (overworld/the_end, appended last) and the
/// four new entity sound-variant registries (cat/chicken/cow/pig_sound_variant)
/// the 26.1 client rejects unless non-empty. Sound-variant entries captured
/// from ViaVersion `sound-variant-registries-26.1.nbt`; all transforms mirror
/// ViaVersion `Protocol1_21_11To26_1`. Built by
/// `tools/registry-gen/build_26_1.py` from `registries_1_21_11.bin`.
static REGISTRIES_26_1: &[u8] =
    include_bytes!("../../../../crates/protocol/data/registries_26_1.bin");
/// 26.2 (proto 776) — the 26.1 set with the 26.2 entity-predicate migration
/// applied (enchantment `predicate:{type:…}` → `{minecraft:entity_type:…}`,
/// per minecraft.wiki Java_Edition_26.2). 26.2 strictly validates the new
/// entity-predicate codec and rejects the 26.1 form, so 775 and 776 need
/// distinct bundles. Built by `tools/registry-gen/build_26_1.py … 26.2`.
static REGISTRIES_26_2: &[u8] =
    include_bytes!("../../../../crates/protocol/data/registries_26_2.bin");

/// Selects the embedded registry bundle appropriate for a given Minecraft protocol version.
///
/// This returns a static byte slice containing the pre-generated configuration-phase
/// registry bundle for the requested protocol when the protocol uses per-registry
/// configuration (Minecraft 1.20.5 / 1.21.x series). For protocol numbers newer
/// than the newest supported bundle, the newest bundle is returned as a best-effort
/// fallback; for older protocols that do not use per-registry bundles this returns
/// `None`.
///
/// Mapping:
/// - 766 → 1.20.5 bundle
/// - 767 → 1.21.0 / 1.21.1 bundle
/// - 768..=769 → 1.21.2–1.21.4 bundle
/// - 770 → 1.21.5 bundle
/// - 771..=773 → 1.21.6–1.21.9 bundle
/// - 774 → 1.21.10 / 1.21.11 bundle
/// - 775 → 26.1 bundle
/// - 776 → 26.2 bundle (new entity-predicate codec)
/// - p > 776 → newest bundle (best-effort fallback)
///
/// # Examples
///
/// ```ignore
/// assert!(bundle_for_proto(770).is_some()); // 1.21.5
/// assert!(bundle_for_proto(765).is_none()); // pre-1.20.5
/// assert!(bundle_for_proto(800).is_some()); // best-effort: newest embedded bundle
/// ```
pub fn bundle_for_proto(proto: u32) -> Option<&'static [u8]> {
    match proto {
        766 => Some(REGISTRIES_1_20_5),       // 1.20.5 / 1.20.6
        767 => Some(REGISTRIES_1_21),         // 1.21 / 1.21.1
        768..=769 => Some(REGISTRIES_1_21_3), // 1.21.2 / 1.21.3 / 1.21.4
        770 => Some(REGISTRIES_1_21_5),       // 1.21.5
        771..=773 => Some(REGISTRIES_1_21_6), // 1.21.6 – 1.21.9
        774 => Some(REGISTRIES_1_21_11),      // 1.21.10 / 1.21.11
        775 => Some(REGISTRIES_26_1),         // 26.1 / 26.1.1 / 26.1.2
        776 => Some(REGISTRIES_26_2),         // 26.2 (new entity-predicate codec)
        // Anything past the highest protocol we have data for reuses the
        // newest complete set as a logged best-effort.
        p if p > 776 => Some(REGISTRIES_26_2),
        _ => None,
    }
}

/// Indicates whether the registry bundle chosen for `proto` is a best-effort fallback.
///
/// Protocols greater than 776 reuse the newest-known embedded bundle as a best-effort mapping;
/// protocols 776 and below have version-matched bundles.
///
/// # Returns
///
/// `true` if the selection is a best-effort fallback (protocol > 776), `false` otherwise.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(bundle_is_fallback(776), false);
/// assert_eq!(bundle_is_fallback(777), true);
/// ```
pub fn bundle_is_fallback(proto: u32) -> bool {
    proto > 776
}

/// Tag registries the 1.21.2+ client requires to be *bound* in the config
/// phase. From 1.21.2 (proto 768) the data-driven `minecraft:enchantment`
/// registry is validated strictly: each enchantment's `supported_items` /
/// `exclusive_set` reference item/enchantment tags (e.g.
/// `#minecraft:enchantable/sword`), and the client also hard-references a
/// fixed set of entity/block/biome tags. If those tags are never sent the
/// client aborts config with "Errors in registry minecraft:enchantment" /
/// "Unbound tags …" (a Network Protocol Error). Vanilla binds them via a
/// config-phase `UpdateTags`; the limbo sends them here as EMPTY tags —
/// enough to bind every reference (an empty `supported_items` is valid; no
/// enchanting happens in limbo anyway).
///
/// 1.21/1.21.1 (767) ship the same enchantment registry but their client
/// doesn't fatally reject the unbound tags, so we scope this to 768+.
///
/// Tag list captured from the 1.21.2 client's own registry-loading error
/// (the authoritative "missing/unbound" report).
const TAGS_1_21_2: &[(&str, &[&str])] = &[
    (
        "minecraft:item",
        &[
            "minecraft:enchantable/head_armor",
            "minecraft:enchantable/sword",
            "minecraft:enchantable/weapon",
            "minecraft:enchantable/equippable",
            "minecraft:enchantable/armor",
            "minecraft:enchantable/mace",
            "minecraft:enchantable/foot_armor",
            "minecraft:enchantable/mining",
            "minecraft:enchantable/fire_aspect",
            "minecraft:enchantable/bow",
            "minecraft:enchantable/mining_loot",
            "minecraft:enchantable/trident",
            "minecraft:enchantable/crossbow",
            "minecraft:enchantable/fishing",
            "minecraft:enchantable/durability",
            "minecraft:enchantable/sharp_weapon",
            "minecraft:enchantable/leg_armor",
            "minecraft:enchantable/chest_armor",
            "minecraft:enchantable/vanishing",
        ],
    ),
    (
        "minecraft:entity_type",
        &[
            "minecraft:sensitive_to_bane_of_arthropods",
            "minecraft:sensitive_to_impaling",
            "minecraft:sensitive_to_smite",
            "minecraft:arrows",
        ],
    ),
    (
        "minecraft:block",
        &[
            "minecraft:soul_speed_blocks",
            "minecraft:blocks_wind_charge_explosions",
        ],
    ),
    (
        "minecraft:enchantment",
        &[
            "minecraft:exclusive_set/armor",
            "minecraft:exclusive_set/boots",
            "minecraft:exclusive_set/bow",
            "minecraft:exclusive_set/crossbow",
            "minecraft:exclusive_set/damage",
            "minecraft:exclusive_set/mining",
            "minecraft:exclusive_set/riptide",
        ],
    ),
    (
        "minecraft:worldgen/biome",
        &[
            "minecraft:is_badlands",
            "minecraft:is_jungle",
            "minecraft:is_savanna",
        ],
    ),
];

/// Build the config-phase `UpdateTags` packet body that binds the tags
/// [`TAGS_1_21_2`] lists, all empty, for proto >= 768. Returns `None` for
/// protocols that don't need it (the enchantment-tag validation is a 1.21.2+
/// thing). Wire format (minecraft.wiki "Update Tags"):
///
/// ```text
/// [VarInt registry_count]
/// repeat: [Identifier registry][VarInt tag_count]
///   repeat: [Identifier tag][VarInt entry_count][VarInt entry_id…]
/// ```
pub fn config_tags_body_for_proto(proto: u32) -> Option<Vec<u8>> {
    use bytes::BufMut;
    if proto < 768 {
        return None;
    }
    let write_str = |buf: &mut Vec<u8>, s: &str| {
        // VarInt length prefix (these strings are all < 128 bytes, but
        // encode a full VarInt to stay correct).
        let mut len = s.len() as u32;
        loop {
            let mut byte = (len & 0x7F) as u8;
            len >>= 7;
            if len != 0 {
                byte |= 0x80;
            }
            buf.put_u8(byte);
            if len == 0 {
                break;
            }
        }
        buf.extend_from_slice(s.as_bytes());
    };
    // Accumulate tags per registry so version-specific additions MERGE into
    // the right registry entry instead of emitting a second (conflicting)
    // entry for e.g. `minecraft:item`. Order is preserved; new registries
    // (dialog, timeline) append.
    let mut registries: Vec<(&str, Vec<&str>)> = TAGS_1_21_2
        .iter()
        .map(|(r, tags)| (*r, tags.to_vec()))
        .collect();
    let mut extend = |reg: &'static str, more: &[&'static str]| {
        if let Some(entry) = registries.iter_mut().find(|(r, _)| *r == reg) {
            entry.1.extend_from_slice(more);
        } else {
            registries.push((reg, more.to_vec()));
        }
    };

    // 1.21.6 (771) added the `minecraft:dialog` registry; bind its tags only
    // once that registry exists (an unknown-registry tag would error < 771).
    if proto >= 771 {
        extend("minecraft:dialog", DIALOG_TAGS_1_21_6);
    }
    // 1.21.11 (774) uses the genuine enchantment registry, which references
    // the newer `enchantable/melee_weapon|lunge|sweeping` item tags and the
    // `lightning_rods` block tag; it also adds the `minecraft:timeline`
    // registry (tags `in_overworld|in_nether|in_end`). 770–773 use the
    // 1.21.4-rebased enchantment that doesn't reference these, so scope to
    // 774+.
    if proto >= 774 {
        extend("minecraft:item", ITEM_TAGS_1_21_11);
        extend("minecraft:block", BLOCK_TAGS_1_21_11);
        extend("minecraft:timeline", TIMELINE_TAGS_1_21_11);
    }
    // 26.1 (775) eagerly builds default item components during config: a
    // `fire_resistant` item resolves `#minecraft:is_fire`, and other defaults
    // reference the rest of the damage_type / banner_pattern tag sets. If those
    // tag keys aren't bound the client aborts FinishConfiguration with
    // "Missing tag TagKey[minecraft:damage_type / minecraft:is_fire]". Vanilla
    // binds them via UpdateTags; ViaVersion `Protocol1_21_11To26_1` adds the
    // same lists empty (`tagRewriter.addEmptyTags`). Bind them here, empty.
    if proto >= 775 {
        extend("minecraft:damage_type", DAMAGE_TYPE_TAGS_26_1);
        extend("minecraft:banner_pattern", BANNER_PATTERN_TAGS_26_1);
        // 26.x dimension_type entries reference `infiniburn:"#minecraft:..."`
        // block tags; 26.2 validates them strictly during registry load
        // ("Missing tag: minecraft:infiniburn_overworld in minecraft:block").
        // Bind them (empty — no infinite-burn blocks needed in limbo).
        extend("minecraft:block", INFINIBURN_TAGS_26);
    }

    let mut body = Vec::new();
    write_varint(&mut body, registries.len() as u32);
    for (registry, tags) in &registries {
        write_str(&mut body, registry);
        write_varint(&mut body, tags.len() as u32);
        for tag in tags {
            write_str(&mut body, tag);
            write_varint(&mut body, 0); // empty tag: zero entries
        }
    }
    Some(body)
}

/// `minecraft:dialog` registry tags the 1.21.6+ (proto 771+) client requires
/// bound. Captured from the 1.21.6 client's "Unbound tags … dialog" report.
const DIALOG_TAGS_1_21_6: &[&str] = &[
    "minecraft:pause_screen_additions",
    "minecraft:quick_actions",
];

/// Extra `minecraft:item` enchantable tags the genuine 1.21.11 enchantment
/// registry references (added 1.21.5+). Captured from the 1.21.11 client's
/// "Missing tag" enchantment errors.
const ITEM_TAGS_1_21_11: &[&str] = &[
    "minecraft:enchantable/melee_weapon",
    "minecraft:enchantable/lunge",
    "minecraft:enchantable/sweeping",
];

/// Extra `minecraft:block` tag the 1.21.11 enchantment registry references
/// (channeling → `lightning_rods`).
const BLOCK_TAGS_1_21_11: &[&str] = &["minecraft:lightning_rods"];

/// `minecraft:timeline` registry tags the 1.21.11 (proto 774) client requires
/// bound (the timeline registry is new in 1.21.10/1.21.11).
const TIMELINE_TAGS_1_21_11: &[&str] = &[
    "minecraft:in_overworld",
    "minecraft:in_nether",
    "minecraft:in_end",
];

/// `minecraft:damage_type` tags the 26.1 (proto 775) client requires bound
/// before FinishConfiguration: default item-component construction resolves
/// `#minecraft:is_fire` (fire-resistant items) and the rest of these. Verbatim
/// from ViaVersion `Protocol1_21_11To26_1.onMappingDataLoaded`
/// (`addEmptyTags(RegistryType.DAMAGE_TYPE, …)`).
const DAMAGE_TYPE_TAGS_26_1: &[&str] = &[
    "minecraft:damages_helmet",
    "minecraft:bypasses_armor",
    "minecraft:bypasses_shield",
    "minecraft:bypasses_invulnerability",
    "minecraft:bypasses_cooldown",
    "minecraft:bypasses_effects",
    "minecraft:bypasses_resistance",
    "minecraft:bypasses_enchantments",
    "minecraft:is_fire",
    "minecraft:is_projectile",
    "minecraft:witch_resistant_to",
    "minecraft:is_explosion",
    "minecraft:is_fall",
    "minecraft:is_drowning",
    "minecraft:is_freezing",
    "minecraft:is_lightning",
    "minecraft:no_anger",
    "minecraft:no_impact",
    "minecraft:always_most_significant_fall",
    "minecraft:wither_immune_to",
    "minecraft:ignites_armor_stands",
    "minecraft:burns_armor_stands",
    "minecraft:avoids_guardian_thorns",
    "minecraft:always_triggers_silverfish",
    "minecraft:always_hurts_ender_dragons",
    "minecraft:no_knockback",
    "minecraft:always_kills_armor_stands",
    "minecraft:can_break_armor_stand",
    "minecraft:bypasses_wolf_armor",
    "minecraft:is_player_attack",
    "minecraft:burn_from_stepping",
    "minecraft:panic_causes",
    "minecraft:panic_environmental_causes",
    "minecraft:mace_smash",
];

/// `minecraft:block` infiniburn tags referenced by 26.x dimension_type
/// entries (`infiniburn:"#minecraft:infiniburn_<dim>"`). 26.2 validates these
/// strictly at registry-load time.
const INFINIBURN_TAGS_26: &[&str] = &[
    "minecraft:infiniburn_overworld",
    "minecraft:infiniburn_nether",
    "minecraft:infiniburn_end",
];

/// `minecraft:banner_pattern` tags the 26.1 client requires bound (referenced
/// by default item components). Verbatim from ViaVersion
/// `addEmptyTags(RegistryType.BANNER_PATTERN, …)`.
const BANNER_PATTERN_TAGS_26_1: &[&str] = &[
    "minecraft:no_item_required",
    "minecraft:pattern_item/flower",
    "minecraft:pattern_item/creeper",
    "minecraft:pattern_item/skull",
    "minecraft:pattern_item/mojang",
    "minecraft:pattern_item/globe",
    "minecraft:pattern_item/piglin",
    "minecraft:pattern_item/flow",
    "minecraft:pattern_item/guster",
    "minecraft:pattern_item/field_masoned",
    "minecraft:pattern_item/bordure_indented",
];

/// Transform a `ClientboundRegistryData` body so each biome's
/// `effects.music` (a single music compound) becomes the 1.21.4 weighted
/// list `[{data: <music>, weight: 1}]`. Returns `Some(new_body)` only for
/// proto 769 (1.21.4) acting on the `minecraft:worldgen/biome` registry;
/// `None` otherwise (caller sends the original body unchanged).
///
/// 1.21.4 (proto 769) reworked biome `music` from `Optional<Music>` to a
/// `WeightedList<Music>`. Our 768/769 bundle (`REGISTRIES_1_21_3`) carries
/// the 1.21.2/1.21.3 object form, which a 1.21.4 client rejects with
/// "Not a list: {…music…}" / "Unbound values in registry worldgen/biome".
/// 1.21.2/1.21.3 (768) still want the object form, so this is scoped to 769.
/// 1.21.5+ (770+) use different captured bundles that already ship the list
/// form. Mirrors ViaVersion `EntityPacketRewriter1_21_4`'s biome handler.
pub fn biome_music_list_transform(proto: u32, body: &[u8]) -> Option<Vec<u8>> {
    use bytes::{Buf, Bytes};
    use kojacoord_protocol::codec::{Decode, Encode};
    use kojacoord_protocol::types::nbt::Nbt;
    use kojacoord_protocol::types::VarInt;

    if proto != 769 {
        return None;
    }

    // --- read the leading registry id; bail unless it's the biome registry.
    let mut pos = 0usize;
    let reg = read_mc_string(body, &mut pos)?;
    if reg != "minecraft:worldgen/biome" {
        return None;
    }

    let mut out = Vec::with_capacity(body.len() + 64);
    write_mc_string(&mut out, &reg);

    // entry count
    let mut cur = Bytes::copy_from_slice(&body[pos..]);
    let count = VarInt::decode(&mut cur).ok()?.0;
    pos = body.len() - cur.remaining();
    write_varint(&mut out, count as u32);

    for _ in 0..count {
        let key = read_mc_string(body, &mut pos)?;
        write_mc_string(&mut out, &key);
        // has_data bool
        let has_data = *body.get(pos)? != 0;
        out.push(body[pos]);
        pos += 1;
        if !has_data {
            continue;
        }
        // Nameless network NBT (`0x0a <payload>`). Inject an empty name
        // (`0x00 0x00`) after the tag id so the named `Nbt::decode` can
        // parse it; strip it again on re-encode.
        if *body.get(pos)? != 0x0a {
            return None;
        }
        let mut named = Vec::with_capacity(3 + (body.len() - pos - 1));
        named.push(0x0a);
        named.push(0);
        named.push(0);
        named.extend_from_slice(&body[pos + 1..]);
        let mut nb = Bytes::copy_from_slice(&named);
        let before = nb.remaining();
        let mut nbt = Nbt::decode(&mut nb).ok()?;
        let consumed_named = before - nb.remaining();
        pos += consumed_named - 2; // advance over the original (nameless) bytes

        wrap_biome_music(&mut nbt.root);

        let mut buf = bytes::BytesMut::new();
        nbt.encode(&mut buf).ok()?;
        // Re-frame named (`0x0a 0x00 0x00 …`) back to nameless (`0x0a …`).
        let b = buf.as_ref();
        if b.len() < 3 || b[0] != 0x0a {
            return None;
        }
        out.push(0x0a);
        out.extend_from_slice(&b[3..]);
    }
    Some(out)
}

/// Wrap `root.effects.music` (a compound) into the 1.21.4 weighted-list form
/// `[{data: <music>, weight: 1}]`. No-op when music is absent or already a
/// list.
fn wrap_biome_music(
    root: &mut std::collections::HashMap<String, kojacoord_protocol::types::nbt::NbtTag>,
) {
    use kojacoord_protocol::types::nbt::NbtTag;
    use std::collections::HashMap;

    let Some(NbtTag::Compound(effects)) = root.get_mut("effects") else {
        return;
    };
    if let Some(NbtTag::Compound(_)) = effects.get("music") {
        let music = effects.remove("music").unwrap();
        let mut weighted = HashMap::new();
        weighted.insert("data".to_string(), music);
        weighted.insert("weight".to_string(), NbtTag::Int(1));
        effects.insert(
            "music".to_string(),
            NbtTag::List(vec![NbtTag::Compound(weighted)]),
        );
    }
}

/// Read a length-prefixed (VarInt) UTF-8 Minecraft string at `*pos`,
/// advancing `*pos`. Returns `None` on truncation / invalid UTF-8.
fn read_mc_string(body: &[u8], pos: &mut usize) -> Option<String> {
    let mut len = 0u32;
    let mut shift = 0u32;
    loop {
        let byte = *body.get(*pos)?;
        *pos += 1;
        len |= ((byte & 0x7F) as u32) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    let end = pos.checked_add(len as usize)?;
    let s = body.get(*pos..end)?;
    *pos = end;
    String::from_utf8(s.to_vec()).ok()
}

/// Write a Minecraft string (VarInt length prefix + UTF-8 bytes).
fn write_mc_string(buf: &mut Vec<u8>, s: &str) {
    write_varint(buf, s.len() as u32);
    buf.extend_from_slice(s.as_bytes());
}

/// Minimal unsigned VarInt writer for the tags body.
fn write_varint(buf: &mut Vec<u8>, mut v: u32) {
    use bytes::BufMut;
    loop {
        let mut byte = (v & 0x7F) as u8;
        v >>= 7;
        if v != 0 {
            byte |= 0x80;
        }
        buf.put_u8(byte);
        if v == 0 {
            break;
        }
    }
}

/// Split a registry bundle blob into its contained `ClientboundRegistryData` packet bodies.
///
/// The bundle format is: big-endian `u32` registry count, followed by that many entries each
/// encoded as a big-endian `u32` body length and then `body` bytes. This function returns
/// slices that borrow from the provided `bundle`.
///
/// On malformed input this returns an `Err` with one of the exact messages produced by the
/// parser:
/// - `"registry bundle truncated"` when a required u32 read would run past the end.
/// - `"registry bundle body overruns bundle"` when a declared body length extends beyond the bundle.
///
/// # Returns
///
/// `Ok(Vec<&[u8]>)` with one slice per registry-data body on success, `Err(String)` with a
/// descriptive message on malformed data.
///
/// # Examples
///
/// ```ignore
/// let mut bytes = Vec::new();
/// // num = 1
/// bytes.extend(&1u32.to_be_bytes());
/// // len = 3
/// bytes.extend(&3u32.to_be_bytes());
/// // body = [1,2,3]
/// bytes.extend(&[1u8, 2, 3]);
///
/// let parts = crate::net::registry_data::parse_bundle(&bytes).unwrap();
/// assert_eq!(parts.len(), 1);
/// assert_eq!(parts[0], &[1u8, 2, 3]);
/// ```
pub fn parse_bundle(bundle: &[u8]) -> Result<Vec<&[u8]>, String> {
    let mut off = 0usize;
    let read_u32 = |b: &[u8], off: &mut usize| -> Result<u32, String> {
        if *off + 4 > b.len() {
            return Err("registry bundle truncated".into());
        }
        let v = u32::from_be_bytes([b[*off], b[*off + 1], b[*off + 2], b[*off + 3]]);
        *off += 4;
        Ok(v)
    };
    let num = read_u32(bundle, &mut off)?;
    let mut out = Vec::with_capacity(num as usize);
    for _ in 0..num {
        let len = read_u32(bundle, &mut off)? as usize;
        if off + len > bundle.len() {
            return Err("registry bundle body overruns bundle".into());
        }
        out.push(&bundle[off..off + len]);
        off += len;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Extracts registry identifier strings from a registry bundle.
    ///
    /// Parses the provided bundle framing and returns the list of registry IDs found in each embedded
    /// registry-data body. Each body is expected to start with a Minecraft string (VarInt length
    /// followed by UTF-8 bytes); this function decodes that leading string for every body.
    ///
    /// # Parameters
    ///
    /// - `bundle` — byte slice containing a registry bundle (u32 count, then repeated u32 body_len + body).
    ///
    /// # Returns
    ///
    /// A `Vec<String>` containing the registry id decoded from the start of each body, in bundle order.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let bundle: &[u8] = &[
    ///     0, 0, 0, 1,      // num_registries = 1
    ///     0, 0, 0, 5,      // body_len = 5
    ///     0x04, b't', b'e', b's', b't', // body: VarInt(4) + "test"
    /// ];
    /// let ids = registry_ids(bundle);
    /// assert_eq!(ids, vec!["test".to_string()]);
    /// ```
    fn registry_ids(bundle: &[u8]) -> Vec<String> {
        let bodies = parse_bundle(bundle).expect("parse");
        bodies
            .iter()
            .map(|b| {
                // body starts with a Minecraft String: VarInt len + utf8.
                let mut i = 0usize;
                let mut len = 0u32;
                let mut shift = 0;
                loop {
                    let byte = b[i];
                    i += 1;
                    len |= ((byte & 0x7F) as u32) << shift;
                    if byte & 0x80 == 0 {
                        break;
                    }
                    shift += 7;
                }
                String::from_utf8(b[i..i + len as usize].to_vec()).unwrap()
            })
            .collect()
    }

    #[test]
    fn bundles_parse_and_contain_core_registries() {
        // Per-version registry counts (ViaVersion-derived): 1.21.5 adds
        // 6 mob-variant registries, 1.21.6 adds dialog, 1.21.10/.11 add 4.
        for (proto, expect_n) in [
            (766u32, 8usize), // 1.20.5/.6
            (767, 11),        // 1.21/.1
            (768, 12),        // 1.21.2/.3
            (769, 12),        // 1.21.4 (no additions over 1.21.3)
            (770, 18),        // 1.21.5
            (771, 19),        // 1.21.6
            (772, 19),        // 1.21.7/.8
            (773, 19),        // 1.21.9
            (774, 23),        // 1.21.10/.11
        ] {
            let bundle = bundle_for_proto(proto).expect("bundle present");
            let ids = registry_ids(bundle);
            assert_eq!(ids.len(), expect_n, "proto {proto} registry count");
            // dimension_type + biome are always required to join a world.
            for required in ["minecraft:dimension_type", "minecraft:worldgen/biome"] {
                assert!(
                    ids.iter().any(|s| s == required),
                    "proto {proto} bundle missing {required}"
                );
            }
        }
        // 1.21.5+ must carry the new mob-variant registries.
        let ids = registry_ids(bundle_for_proto(770).unwrap());
        for v in [
            "minecraft:cat_variant",
            "minecraft:pig_variant",
            "minecraft:wolf_sound_variant",
        ] {
            assert!(ids.iter().any(|s| s == v), "1.21.5 missing {v}");
        }
        // dialog only from 1.21.6.
        assert!(!registry_ids(bundle_for_proto(770).unwrap())
            .iter()
            .any(|s| s == "minecraft:dialog"));
        assert!(registry_ids(bundle_for_proto(771).unwrap())
            .iter()
            .any(|s| s == "minecraft:dialog"));
    }

    #[test]
    fn fallback_mapping() {
        // Every protocol through 776 has a version-matched set (775 = 26.1,
        // 776 = 26.2 with its distinct entity-predicate codec).
        for p in 766..=776 {
            assert!(bundle_for_proto(p).is_some(), "proto {p} bundle");
            assert!(!bundle_is_fallback(p), "proto {p} should be exact");
        }
        // Only future/unknown protocols are best-effort.
        assert!(bundle_for_proto(777).is_some());
        assert!(bundle_is_fallback(777));
        assert!(bundle_for_proto(765).is_none());
    }

    #[test]
    fn entity_predicate_migration_26_2_only() {
        // 26.2 (776) renames the enchantment entity-predicate `type` key to
        // `minecraft:entity_type`; 26.1 (775) keeps the old form.
        let has = |b: &[u8], needle: &str| b.windows(needle.len()).any(|w| w == needle.as_bytes());
        let b775 = bundle_for_proto(775).unwrap();
        let b776 = bundle_for_proto(776).unwrap();
        assert!(
            !has(b775, "minecraft:entity_type"),
            "775 must keep old `type`"
        );
        assert!(
            has(b776, "minecraft:entity_type"),
            "776 must rename to entity_type"
        );
    }

    #[test]
    fn biome_music_transform_769_only() {
        // Grab the real biome registry body from the 768/769 bundle.
        let bodies = parse_bundle(bundle_for_proto(769).unwrap()).unwrap();
        let biome = bodies
            .iter()
            .copied()
            .find(|b| {
                let mut p = 0;
                read_mc_string(b, &mut p).as_deref() == Some("minecraft:worldgen/biome")
            })
            .expect("biome registry present");

        // 768 (1.21.2/1.21.3) must NOT be transformed; 769 (1.21.4) must.
        assert!(biome_music_list_transform(768, biome).is_none());
        let out = biome_music_list_transform(769, biome).expect("769 transform");

        // Output still starts with the biome registry id and grew (each
        // music object gains a `data`/`weight` wrapper).
        let mut p = 0;
        assert_eq!(
            read_mc_string(&out, &mut p).as_deref(),
            Some("minecraft:worldgen/biome")
        );
        assert!(out.len() > biome.len(), "wrapping music should add bytes");

        // Re-parse one biome entry's NBT and confirm `effects.music` is now a
        // List (was a Compound).
        use bytes::{Buf, Bytes};
        use kojacoord_protocol::codec::Decode;
        use kojacoord_protocol::types::nbt::{Nbt, NbtTag};
        use kojacoord_protocol::types::VarInt;
        let mut cur = Bytes::copy_from_slice(&out[p..]);
        let count = VarInt::decode(&mut cur).unwrap().0;
        assert!(count > 0);
        let mut pos = out.len() - cur.remaining();
        let mut saw_music_list = false;
        for _ in 0..count {
            let _key = read_mc_string(&out, &mut pos).unwrap();
            let has_data = out[pos] != 0;
            pos += 1;
            if !has_data {
                continue;
            }
            let mut named = vec![0x0a, 0, 0];
            named.extend_from_slice(&out[pos + 1..]);
            let mut nb = Bytes::copy_from_slice(&named);
            let before = nb.remaining();
            let nbt = Nbt::decode(&mut nb).unwrap();
            pos += (before - nb.remaining()) - 2;
            if let Some(NbtTag::Compound(effects)) = nbt.root.get("effects") {
                if let Some(music) = effects.get("music") {
                    assert!(
                        matches!(music, NbtTag::List(_)),
                        "music must be a list after transform"
                    );
                    saw_music_list = true;
                }
            }
        }
        assert!(saw_music_list, "at least one biome carries music");
    }
}
