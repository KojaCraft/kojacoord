//! Modern (1.9 → 1.21) → 1.8 clientbound converter.
//!
//! The proxy's reference target on the server side is 1.12.2 (protocol 340)
//! and the V1_9_To_1_12 epoch is the primary path. The 1.16 / 1.19 / 1.20 /
//! 1.21 dispatchers below are best-effort partial coverage carried over from
//! the previous implementation and not the focus of this rewrite.
//!
//! Authoritative sources used to derive the tables and field transforms below:
//!   * Java Edition protocol — minecraft.wiki/w/Java_Edition_protocol/Packets
//!   * Protocol history — minecraft.wiki/w/Java_Edition_protocol_history
//!   * PrismarineJS minecraft-data pc/1.8/protocol.json and
//!     pc/1.12.2/protocol.json (authoritative packet shapes).
//!
//! ── Clientbound (server → client) packet-id mapping table ────────────────
//!   name                        1.12.2  1.8     field xform?
//!   SpawnObject                 0x00    0x0E    none (passthrough id)
//!   SpawnExperienceOrb          0x01    0x11    none
//!   SpawnGlobalEntity           0x02    0x2C    none
//!   SpawnMob                    0x03    0x0F    none (best-effort; metadata differs)
//!   SpawnPainting               0x04    0x10    string title format unchanged
//!   SpawnPlayer                 0x05    0x0C    drop trailing metadata
//!   Animation                   0x06    0x0B    none
//!   Statistics                  0x07    0x37    none (string keys differ but shape same)
//!   BlockBreakAnim              0x08    0x25    none
//!   BlockEntityData             0x09    0x35    none
//!   BlockAction                 0x0A    0x24    none
//!   BlockChange                 0x0B    0x23    Position → x/u8 y/z; block_state → id+meta
//!   BossBar                     0x0C    drop    no 1.8 equivalent
//!   ServerDifficulty            0x0D    0x41    none
//!   TabComplete                 0x0E    0x3A    none
//!   ChatMessage                 0x0F    0x02    none
//!   MultiBlockChange            0x10    0x22    none (records unchanged)
//!   ConfirmTransaction          0x11    0x32    none
//!   CloseWindow                 0x12    0x2E    none
//!   OpenWindow                  0x13    0x2D    none
//!   WindowItems                 0x14    0x30    legacy slot identical
//!   WindowProperty              0x15    0x31    none
//!   SetSlot                     0x16    0x2F    legacy slot identical
//!   SetCooldown                 0x17    drop    no 1.8 equivalent
//!   PluginMessage               0x18    0x3F    none
//!   NamedSoundEffect            0x19    0x29    f32 volume + u8 pitch differ — convert
//!   Disconnect                  0x1A    0x40    none
//!   EntityStatus                0x1B    0x1A    none
//!   Explosion                   0x1C    0x27    none
//!   UnloadChunk                 0x1D    drop    1.8 has no explicit unload
//!   ChangeGameState             0x1E    0x2B    none
//!   KeepAlive                   0x1F    0x00    i64 → VarInt
//!   ChunkData                   0x20    0x21    1.9+ adds no_skylight bool; shape compatible
//!   Effect                      0x21    0x28    none
//!   Particle                    0x22    0x2A    1.9+ has different particle id semantics; passthrough
//!   JoinGame                    0x23    0x01    dimension i32 → i8; rebuild
//!   Map                         0x24    0x34    1.9 has tracking marker icons; passthrough
//!   EntityRelMove               0x25    0x15    i16 deltas (1/4096) → i8 (1/32); divide by 128
//!   EntityLookAndRelMove        0x26    0x17    same i16→i8 division
//!   EntityLook                  0x27    0x16    none
//!   Entity                      0x28    0x14    none (just entity id)
//!   VehicleMove                 0x29    drop    no 1.8 equivalent
//!   OpenSignEditor              0x2A    0x36    Position is identical wire shape
//!   CraftRecipeResponse         0x2B    drop    no 1.8 equivalent
//!   PlayerAbilities             0x2C    0x39    none
//!   CombatEvent                 0x2D    0x42    none
//!   PlayerListItem              0x2E    0x38    body shape mostly compatible
//!   PlayerPosLook               0x2F    0x08    strip trailing VarInt teleport_id
//!   UseBed                      0x30    0x0A    none
//!   UnlockRecipes               0x31    drop    no 1.8 equivalent
//!   DestroyEntities             0x32    0x13    none
//!   RemoveEntityEffect          0x33    0x1E    none
//!   ResourcePackSend            0x34    0x48    none
//!   Respawn                     0x35    0x07    dimension i32 → i8
//!   EntityHeadLook              0x36    0x19    none
//!   SelectAdvancementTab        0x37    drop    no 1.8 equivalent
//!   WorldBorder                 0x38    0x44    none
//!   Camera                      0x39    0x43    none
//!   HeldItemChange              0x3A    0x09    none
//!   DisplayScoreboard           0x3B    0x3D    none
//!   EntityMetadata              0x3C    0x1C    none (best-effort)
//!   AttachEntity                0x3D    0x1B    none
//!   EntityVelocity              0x3E    0x12    none
//!   EntityEquipment             0x3F    0x04    legacy slot identical
//!   SetExperience               0x40    0x1F    none
//!   UpdateHealth                0x41    0x06    none
//!   ScoreboardObjective         0x42    0x3B    none
//!   SetPassengers               0x43    drop    no 1.8 equivalent
//!   Teams                       0x44    0x3E    none
//!   UpdateScore                 0x45    0x3C    none
//!   SpawnPosition               0x46    0x05    none
//!   TimeUpdate                  0x47    0x03    none
//!   Title                       0x48    0x45    none
//!   SoundEffect                 0x49    0x29    rough mapping; passthrough id, not name-resolved
//!   PlayerListHeaderFooter      0x4A    0x47    none
//!   CollectItem                 0x4B    0x0D    add count i32 (1.8 has it; 1.12 doesn't include count)
//!   EntityTeleport              0x4C    0x18    f64 abs → i32 fixed-point (x*32)
//!   Advancements                0x4D    drop    no 1.8 equivalent (replaced statistics tab)
//!   EntityProperties            0x4E    0x20    none
//!   EntityEffect                0x4F    0x1D    none

#![allow(dead_code)] // packet-id constants below are kept as a reference table

use bytes::{BufMut, Bytes, BytesMut};
use kojacoord_protocol::codec::Encode;
use kojacoord_protocol::types::VarInt;
use kojacoord_protocol::Epoch;

use crate::converter::ConversionResult;

use super::{build_payload, nearest, split_id};

// ── 1.12.2 clientbound IDs ────────────────────────────────────────────────

const V112_S2C_SPAWN_OBJECT: u8 = 0x00;
const V112_S2C_SPAWN_EXP_ORB: u8 = 0x01;
const V112_S2C_SPAWN_GLOBAL: u8 = 0x02;
const V112_S2C_SPAWN_MOB: u8 = 0x03;
const V112_S2C_SPAWN_PAINTING: u8 = 0x04;
const V112_S2C_SPAWN_PLAYER: u8 = 0x05;
const V112_S2C_ANIMATION: u8 = 0x06;
const V112_S2C_STATISTICS: u8 = 0x07;
const V112_S2C_BLOCK_BREAK_ANIM: u8 = 0x08;
const V112_S2C_BLOCK_ENTITY_DATA: u8 = 0x09;
const V112_S2C_BLOCK_ACTION: u8 = 0x0A;
const V112_S2C_BLOCK_CHANGE: u8 = 0x0B;
const V112_S2C_BOSS_BAR: u8 = 0x0C;
const V112_S2C_SERVER_DIFFICULTY: u8 = 0x0D;
const V112_S2C_TAB_COMPLETE: u8 = 0x0E;
const V112_S2C_CHAT: u8 = 0x0F;
const V112_S2C_MULTI_BLOCK_CHANGE: u8 = 0x10;
const V112_S2C_CONFIRM_TRANSACTION: u8 = 0x11;
const V112_S2C_CLOSE_WINDOW: u8 = 0x12;
const V112_S2C_OPEN_WINDOW: u8 = 0x13;
const V112_S2C_WINDOW_ITEMS: u8 = 0x14;
const V112_S2C_WINDOW_PROPERTY: u8 = 0x15;
const V112_S2C_SET_SLOT: u8 = 0x16;
const V112_S2C_SET_COOLDOWN: u8 = 0x17;
const V112_S2C_PLUGIN_MESSAGE: u8 = 0x18;
const V112_S2C_NAMED_SOUND: u8 = 0x19;
const V112_S2C_DISCONNECT: u8 = 0x1A;
const V112_S2C_ENTITY_STATUS: u8 = 0x1B;
const V112_S2C_EXPLOSION: u8 = 0x1C;
const V112_S2C_UNLOAD_CHUNK: u8 = 0x1D;
const V112_S2C_CHANGE_GAME_STATE: u8 = 0x1E;
const V112_S2C_KEEP_ALIVE: u8 = 0x1F;
const V112_S2C_CHUNK_DATA: u8 = 0x20;
const V112_S2C_EFFECT: u8 = 0x21;
const V112_S2C_PARTICLE: u8 = 0x22;
const V112_S2C_JOIN_GAME: u8 = 0x23;
const V112_S2C_MAP: u8 = 0x24;
const V112_S2C_ENTITY_REL_MOVE: u8 = 0x25;
const V112_S2C_ENTITY_LOOK_REL_MOVE: u8 = 0x26;
const V112_S2C_ENTITY_LOOK: u8 = 0x27;
const V112_S2C_ENTITY: u8 = 0x28;
const V112_S2C_VEHICLE_MOVE: u8 = 0x29;
const V112_S2C_OPEN_SIGN_EDITOR: u8 = 0x2A;
const V112_S2C_CRAFT_RECIPE_RESP: u8 = 0x2B;
const V112_S2C_PLAYER_ABILITIES: u8 = 0x2C;
const V112_S2C_COMBAT_EVENT: u8 = 0x2D;
const V112_S2C_PLAYER_LIST_ITEM: u8 = 0x2E;
const V112_S2C_PLAYER_POS_LOOK: u8 = 0x2F;
const V112_S2C_USE_BED: u8 = 0x30;
const V112_S2C_UNLOCK_RECIPES: u8 = 0x31;
const V112_S2C_DESTROY_ENTITIES: u8 = 0x32;
const V112_S2C_REMOVE_ENTITY_EFFECT: u8 = 0x33;
const V112_S2C_RESOURCE_PACK: u8 = 0x34;
const V112_S2C_RESPAWN: u8 = 0x35;
const V112_S2C_ENTITY_HEAD_LOOK: u8 = 0x36;
const V112_S2C_SELECT_ADVANCEMENT_TAB: u8 = 0x37;
const V112_S2C_WORLD_BORDER: u8 = 0x38;
const V112_S2C_CAMERA: u8 = 0x39;
const V112_S2C_HELD_ITEM_CHANGE: u8 = 0x3A;
const V112_S2C_DISPLAY_SCOREBOARD: u8 = 0x3B;
const V112_S2C_ENTITY_METADATA: u8 = 0x3C;
const V112_S2C_ATTACH_ENTITY: u8 = 0x3D;
const V112_S2C_ENTITY_VELOCITY: u8 = 0x3E;
const V112_S2C_ENTITY_EQUIPMENT: u8 = 0x3F;
const V112_S2C_SET_EXPERIENCE: u8 = 0x40;
const V112_S2C_UPDATE_HEALTH: u8 = 0x41;
const V112_S2C_SCOREBOARD_OBJ: u8 = 0x42;
const V112_S2C_SET_PASSENGERS: u8 = 0x43;
const V112_S2C_TEAMS: u8 = 0x44;
const V112_S2C_UPDATE_SCORE: u8 = 0x45;
const V112_S2C_SPAWN_POSITION: u8 = 0x46;
const V112_S2C_TIME_UPDATE: u8 = 0x47;
const V112_S2C_TITLE: u8 = 0x48;
const V112_S2C_SOUND_EFFECT: u8 = 0x49;
const V112_S2C_PLAYER_LIST_HEADER_FOOTER: u8 = 0x4A;
const V112_S2C_COLLECT_ITEM: u8 = 0x4B;
const V112_S2C_ENTITY_TELEPORT: u8 = 0x4C;
const V112_S2C_ADVANCEMENTS: u8 = 0x4D;
const V112_S2C_ENTITY_PROPERTIES: u8 = 0x4E;
const V112_S2C_ENTITY_EFFECT: u8 = 0x4F;

// ── 1.8 clientbound IDs ───────────────────────────────────────────────────

const V18_S2C_KEEP_ALIVE: u8 = 0x00;
const V18_S2C_JOIN_GAME: u8 = 0x01;
const V18_S2C_CHAT: u8 = 0x02;
const V18_S2C_TIME_UPDATE: u8 = 0x03;
const V18_S2C_ENTITY_EQUIPMENT: u8 = 0x04;
const V18_S2C_SPAWN_POSITION: u8 = 0x05;
const V18_S2C_UPDATE_HEALTH: u8 = 0x06;
const V18_S2C_RESPAWN: u8 = 0x07;
const V18_S2C_PLAYER_POS_LOOK: u8 = 0x08;
const V18_S2C_HELD_ITEM_CHANGE: u8 = 0x09;
const V18_S2C_USE_BED: u8 = 0x0A;
const V18_S2C_ANIMATION: u8 = 0x0B;
const V18_S2C_SPAWN_PLAYER: u8 = 0x0C;
const V18_S2C_COLLECT_ITEM: u8 = 0x0D;
const V18_S2C_SPAWN_OBJECT: u8 = 0x0E;
const V18_S2C_SPAWN_MOB: u8 = 0x0F;
const V18_S2C_SPAWN_PAINTING: u8 = 0x10;
const V18_S2C_SPAWN_EXP_ORB: u8 = 0x11;
const V18_S2C_ENTITY_VELOCITY: u8 = 0x12;
const V18_S2C_DESTROY_ENTITIES: u8 = 0x13;
const V18_S2C_ENTITY: u8 = 0x14;
const V18_S2C_ENTITY_REL_MOVE: u8 = 0x15;
const V18_S2C_ENTITY_LOOK: u8 = 0x16;
const V18_S2C_ENTITY_LOOK_REL_MOVE: u8 = 0x17;
const V18_S2C_ENTITY_TELEPORT: u8 = 0x18;
const V18_S2C_ENTITY_HEAD_LOOK: u8 = 0x19;
const V18_S2C_ENTITY_STATUS: u8 = 0x1A;
const V18_S2C_ATTACH_ENTITY: u8 = 0x1B;
const V18_S2C_ENTITY_METADATA: u8 = 0x1C;
const V18_S2C_ENTITY_EFFECT: u8 = 0x1D;
const V18_S2C_REMOVE_ENTITY_EFFECT: u8 = 0x1E;
const V18_S2C_SET_EXPERIENCE: u8 = 0x1F;
const V18_S2C_ENTITY_PROPERTIES: u8 = 0x20;
const V18_S2C_CHUNK_DATA: u8 = 0x21;
const V18_S2C_MULTI_BLOCK_CHANGE: u8 = 0x22;
const V18_S2C_BLOCK_CHANGE: u8 = 0x23;
const V18_S2C_BLOCK_ACTION: u8 = 0x24;
const V18_S2C_BLOCK_BREAK_ANIM: u8 = 0x25;
const V18_S2C_EXPLOSION: u8 = 0x27;
const V18_S2C_EFFECT: u8 = 0x28;
const V18_S2C_SOUND_EFFECT: u8 = 0x29;
const V18_S2C_PARTICLE: u8 = 0x2A;
const V18_S2C_CHANGE_GAME_STATE: u8 = 0x2B;
const V18_S2C_SPAWN_GLOBAL: u8 = 0x2C;
const V18_S2C_OPEN_WINDOW: u8 = 0x2D;
const V18_S2C_CLOSE_WINDOW: u8 = 0x2E;
const V18_S2C_SET_SLOT: u8 = 0x2F;
const V18_S2C_WINDOW_ITEMS: u8 = 0x30;
const V18_S2C_WINDOW_PROPERTY: u8 = 0x31;
const V18_S2C_CONFIRM_TRANSACTION: u8 = 0x32;
const V18_S2C_UPDATE_BLOCK_ENTITY: u8 = 0x35;
const V18_S2C_OPEN_SIGN_EDITOR: u8 = 0x36;
const V18_S2C_STATISTICS: u8 = 0x37;
const V18_S2C_PLAYER_LIST_ITEM: u8 = 0x38;
const V18_S2C_PLAYER_ABILITIES: u8 = 0x39;
const V18_S2C_TAB_COMPLETE: u8 = 0x3A;
const V18_S2C_SCOREBOARD_OBJ: u8 = 0x3B;
const V18_S2C_UPDATE_SCORE: u8 = 0x3C;
const V18_S2C_DISPLAY_SCOREBOARD: u8 = 0x3D;
const V18_S2C_TEAMS: u8 = 0x3E;
const V18_S2C_PLUGIN_MESSAGE: u8 = 0x3F;
const V18_S2C_DISCONNECT: u8 = 0x40;
const V18_S2C_SERVER_DIFFICULTY: u8 = 0x41;
const V18_S2C_COMBAT_EVENT: u8 = 0x42;
const V18_S2C_CAMERA: u8 = 0x43;
const V18_S2C_WORLD_BORDER: u8 = 0x44;
const V18_S2C_TITLE: u8 = 0x45;
const V18_S2C_PLAYER_LIST_HEADER_FOOTER: u8 = 0x47;
const V18_S2C_RESOURCE_PACK: u8 = 0x48;

pub fn convert_s2c(payload: Bytes, server_proto: u32) -> ConversionResult {
    let ver = nearest(server_proto);
    let Some((id, body)) = split_id(payload.clone()) else {
        return ConversionResult::Passthrough;
    };

    match ver.epoch() {
        Epoch::V1_9_To_1_12 => dispatch_1_12(id, body),
        Epoch::V1_16 => dispatch_1_16(id, body),
        Epoch::V1_19 | Epoch::V1_20 | Epoch::V1_21Plus => dispatch_modern(id, body, server_proto),
        _ => dispatch_1_12(id, body),
    }
}

fn dispatch_1_12(id: u8, body: Bytes) -> ConversionResult {
    match id {
        V112_S2C_SPAWN_OBJECT => rewrap(body, V18_S2C_SPAWN_OBJECT),
        V112_S2C_SPAWN_EXP_ORB => rewrap(body, V18_S2C_SPAWN_EXP_ORB),
        V112_S2C_SPAWN_GLOBAL => rewrap(body, V18_S2C_SPAWN_GLOBAL),
        V112_S2C_SPAWN_MOB => rewrap(body, V18_S2C_SPAWN_MOB),
        V112_S2C_SPAWN_PAINTING => rewrap(body, V18_S2C_SPAWN_PAINTING),
        V112_S2C_SPAWN_PLAYER => rewrap(body, V18_S2C_SPAWN_PLAYER),
        V112_S2C_ANIMATION => rewrap(body, V18_S2C_ANIMATION),
        V112_S2C_STATISTICS => rewrap(body, V18_S2C_STATISTICS),
        V112_S2C_BLOCK_BREAK_ANIM => rewrap(body, V18_S2C_BLOCK_BREAK_ANIM),
        V112_S2C_BLOCK_ENTITY_DATA => rewrap(body, V18_S2C_UPDATE_BLOCK_ENTITY),
        V112_S2C_BLOCK_ACTION => rewrap(body, V18_S2C_BLOCK_ACTION),
        V112_S2C_BLOCK_CHANGE => s2c_block_change(body),
        V112_S2C_BOSS_BAR => {
            tracing::debug!(target: "converter", "dropping BossBar (no 1.8 equivalent)");
            ConversionResult::Drop
        },
        V112_S2C_SERVER_DIFFICULTY => rewrap(body, V18_S2C_SERVER_DIFFICULTY),
        V112_S2C_TAB_COMPLETE => rewrap(body, V18_S2C_TAB_COMPLETE),
        V112_S2C_CHAT => rewrap(body, V18_S2C_CHAT),
        V112_S2C_MULTI_BLOCK_CHANGE => rewrap(body, V18_S2C_MULTI_BLOCK_CHANGE),
        V112_S2C_CONFIRM_TRANSACTION => rewrap(body, V18_S2C_CONFIRM_TRANSACTION),
        V112_S2C_CLOSE_WINDOW => rewrap(body, V18_S2C_CLOSE_WINDOW),
        V112_S2C_OPEN_WINDOW => rewrap(body, V18_S2C_OPEN_WINDOW),
        V112_S2C_WINDOW_ITEMS => rewrap(body, V18_S2C_WINDOW_ITEMS),
        V112_S2C_WINDOW_PROPERTY => rewrap(body, V18_S2C_WINDOW_PROPERTY),
        V112_S2C_SET_SLOT => rewrap(body, V18_S2C_SET_SLOT),
        V112_S2C_SET_COOLDOWN => {
            tracing::debug!(target: "converter", "dropping SetCooldown (no 1.8 equivalent)");
            ConversionResult::Drop
        },
        V112_S2C_PLUGIN_MESSAGE => rewrap(body, V18_S2C_PLUGIN_MESSAGE),
        V112_S2C_NAMED_SOUND => rewrap(body, V18_S2C_SOUND_EFFECT),
        V112_S2C_DISCONNECT => rewrap(body, V18_S2C_DISCONNECT),
        V112_S2C_ENTITY_STATUS => rewrap(body, V18_S2C_ENTITY_STATUS),
        V112_S2C_EXPLOSION => rewrap(body, V18_S2C_EXPLOSION),
        V112_S2C_UNLOAD_CHUNK => {
            tracing::debug!(target: "converter", "dropping UnloadChunk (1.8 has no explicit unload packet)");
            ConversionResult::Drop
        },
        V112_S2C_CHANGE_GAME_STATE => rewrap(body, V18_S2C_CHANGE_GAME_STATE),
        V112_S2C_KEEP_ALIVE => s2c_keep_alive_long_to_varint(body),
        V112_S2C_CHUNK_DATA => rewrap(body, V18_S2C_CHUNK_DATA),
        V112_S2C_EFFECT => rewrap(body, V18_S2C_EFFECT),
        V112_S2C_PARTICLE => rewrap(body, V18_S2C_PARTICLE),
        V112_S2C_JOIN_GAME => s2c_join_game(body),
        V112_S2C_MAP => rewrap(body, V18_S2C_MAP_UNMAPPED),
        V112_S2C_ENTITY_REL_MOVE => s2c_entity_rel_move(body, false),
        V112_S2C_ENTITY_LOOK_REL_MOVE => s2c_entity_rel_move(body, true),
        V112_S2C_ENTITY_LOOK => rewrap(body, V18_S2C_ENTITY_LOOK),
        V112_S2C_ENTITY => rewrap(body, V18_S2C_ENTITY),
        V112_S2C_VEHICLE_MOVE => {
            tracing::debug!(target: "converter", "dropping VehicleMove (no 1.8 equivalent)");
            ConversionResult::Drop
        },
        V112_S2C_OPEN_SIGN_EDITOR => rewrap(body, V18_S2C_OPEN_SIGN_EDITOR),
        V112_S2C_CRAFT_RECIPE_RESP => {
            tracing::debug!(target: "converter", "dropping CraftRecipeResponse (no 1.8 equivalent)");
            ConversionResult::Drop
        },
        V112_S2C_PLAYER_ABILITIES => rewrap(body, V18_S2C_PLAYER_ABILITIES),
        V112_S2C_COMBAT_EVENT => rewrap(body, V18_S2C_COMBAT_EVENT),
        V112_S2C_PLAYER_LIST_ITEM => rewrap(body, V18_S2C_PLAYER_LIST_ITEM),
        V112_S2C_PLAYER_POS_LOOK => s2c_player_pos_look(body),
        V112_S2C_USE_BED => rewrap(body, V18_S2C_USE_BED),
        V112_S2C_UNLOCK_RECIPES => {
            tracing::debug!(target: "converter", "dropping UnlockRecipes (no 1.8 equivalent)");
            ConversionResult::Drop
        },
        V112_S2C_DESTROY_ENTITIES => rewrap(body, V18_S2C_DESTROY_ENTITIES),
        V112_S2C_REMOVE_ENTITY_EFFECT => rewrap(body, V18_S2C_REMOVE_ENTITY_EFFECT),
        V112_S2C_RESOURCE_PACK => rewrap(body, V18_S2C_RESOURCE_PACK),
        V112_S2C_RESPAWN => s2c_respawn(body),
        V112_S2C_ENTITY_HEAD_LOOK => rewrap(body, V18_S2C_ENTITY_HEAD_LOOK),
        V112_S2C_SELECT_ADVANCEMENT_TAB => {
            tracing::debug!(target: "converter", "dropping SelectAdvancementTab (no 1.8 equivalent)");
            ConversionResult::Drop
        },
        V112_S2C_WORLD_BORDER => rewrap(body, V18_S2C_WORLD_BORDER),
        V112_S2C_CAMERA => rewrap(body, V18_S2C_CAMERA),
        V112_S2C_HELD_ITEM_CHANGE => rewrap(body, V18_S2C_HELD_ITEM_CHANGE),
        V112_S2C_DISPLAY_SCOREBOARD => rewrap(body, V18_S2C_DISPLAY_SCOREBOARD),
        V112_S2C_ENTITY_METADATA => rewrap(body, V18_S2C_ENTITY_METADATA),
        V112_S2C_ATTACH_ENTITY => rewrap(body, V18_S2C_ATTACH_ENTITY),
        V112_S2C_ENTITY_VELOCITY => rewrap(body, V18_S2C_ENTITY_VELOCITY),
        V112_S2C_ENTITY_EQUIPMENT => rewrap(body, V18_S2C_ENTITY_EQUIPMENT),
        V112_S2C_SET_EXPERIENCE => rewrap(body, V18_S2C_SET_EXPERIENCE),
        V112_S2C_UPDATE_HEALTH => rewrap(body, V18_S2C_UPDATE_HEALTH),
        V112_S2C_SCOREBOARD_OBJ => rewrap(body, V18_S2C_SCOREBOARD_OBJ),
        V112_S2C_SET_PASSENGERS => {
            tracing::debug!(target: "converter", "dropping SetPassengers (no 1.8 equivalent)");
            ConversionResult::Drop
        },
        V112_S2C_TEAMS => rewrap(body, V18_S2C_TEAMS),
        V112_S2C_UPDATE_SCORE => rewrap(body, V18_S2C_UPDATE_SCORE),
        V112_S2C_SPAWN_POSITION => rewrap(body, V18_S2C_SPAWN_POSITION),
        V112_S2C_TIME_UPDATE => rewrap(body, V18_S2C_TIME_UPDATE),
        V112_S2C_TITLE => rewrap(body, V18_S2C_TITLE),
        V112_S2C_SOUND_EFFECT => rewrap(body, V18_S2C_SOUND_EFFECT),
        V112_S2C_PLAYER_LIST_HEADER_FOOTER => rewrap(body, V18_S2C_PLAYER_LIST_HEADER_FOOTER),
        V112_S2C_COLLECT_ITEM => s2c_collect_item(body),
        V112_S2C_ENTITY_TELEPORT => s2c_entity_teleport(body),
        V112_S2C_ADVANCEMENTS => {
            tracing::debug!(target: "converter", "dropping Advancements (replaced 1.8 statistics tab)");
            ConversionResult::Drop
        },
        V112_S2C_ENTITY_PROPERTIES => rewrap(body, V18_S2C_ENTITY_PROPERTIES),
        V112_S2C_ENTITY_EFFECT => rewrap(body, V18_S2C_ENTITY_EFFECT),
        _ => ConversionResult::Passthrough,
    }
}

// Map packet ID kept distinct so map data can pass through unchanged.
const V18_S2C_MAP_UNMAPPED: u8 = 0x34;

// ── 1.16+ dispatchers (retained from previous implementation) ─────────────

fn dispatch_1_16(id: u8, body: Bytes) -> ConversionResult {
    // Not the focus of this rewrite; retained best-effort coverage.
    match id {
        0x24 => s2c_join_game(body),
        0x34 => s2c_player_pos_look(body),
        0x0E => rewrap(body, V18_S2C_CHAT),
        0x49 => rewrap(body, V18_S2C_UPDATE_HEALTH),
        0x3A => s2c_respawn(body),
        0x1F => s2c_keep_alive_long_to_varint(body),
        0x4E => rewrap(body, V18_S2C_TIME_UPDATE),
        0x53 => rewrap(body, V18_S2C_PLAYER_LIST_HEADER_FOOTER),
        0x46 => rewrap(body, V18_S2C_ENTITY_VELOCITY),
        0x1C => {
            tracing::debug!(target: "converter", "dropping UnloadChunk (1.16, no 1.8 equivalent)");
            ConversionResult::Drop
        },
        0x56 => s2c_entity_teleport(body),
        0x27 => s2c_entity_rel_move(body, false),
        0x28 => s2c_entity_rel_move(body, true),
        0x36 => rewrap(body, V18_S2C_DESTROY_ENTITIES),
        0x0B => s2c_block_change(body),
        0x15 => rewrap(body, V18_S2C_SET_SLOT),
        0x14 => rewrap(body, V18_S2C_WINDOW_ITEMS),
        0x44 => rewrap(body, V18_S2C_ENTITY_METADATA),
        0x47 => rewrap(body, V18_S2C_ENTITY_EQUIPMENT),
        0x48 => rewrap(body, V18_S2C_SET_EXPERIENCE),
        0x4B => rewrap(body, V18_S2C_SCOREBOARD_OBJ),
        0x4D => rewrap(body, V18_S2C_UPDATE_SCORE),
        0x3F => rewrap(body, V18_S2C_HELD_ITEM_CHANGE),
        _ => ConversionResult::Passthrough,
    }
}

fn dispatch_modern(id: u8, body: Bytes, _server_proto: u32) -> ConversionResult {
    // Best-effort fall-through for 1.19+. The proxy's reference target is
    // 1.12.2; this branch exists only to avoid breaking other paths.
    match id {
        0x28 => s2c_join_game(body),
        0x3E => s2c_player_pos_look(body),
        0x23 => s2c_keep_alive_long_to_varint(body),
        _ => ConversionResult::Passthrough,
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn rewrap(body: Bytes, new_id: u8) -> ConversionResult {
    ConversionResult::Converted(vec![build_payload(new_id, &body)])
}

/// 1.12.2 JoinGame: i32 eid, u8 gamemode, i32 dimension, u8 difficulty,
///                  u8 max_players, String level_type, bool reduced_debug.
/// 1.8 JoinGame:    i32 eid, u8 gamemode, i8 dimension, u8 difficulty,
///                  u8 max_players, String level_type, bool reduced_debug.
fn s2c_join_game(body: Bytes) -> ConversionResult {
    let mut r = super::safe::Reader::new(body);
    let Some(eid) = r.i32() else {
        return ConversionResult::Passthrough;
    };
    let Some(gm) = r.u8() else {
        return ConversionResult::Passthrough;
    };
    // 1.12.2 dimension is i32; we narrow to i8 (overworld=0, nether=-1, end=1).
    let dimension = r.i32().unwrap_or(0) as i8;
    let difficulty = r.u8().unwrap_or(2);
    let max_players = r.u8().unwrap_or(100);
    let level_type = r.string().unwrap_or_else(|| "default".to_owned());
    let reduced_debug = r.u8().unwrap_or(0);

    let mut out = BytesMut::new();
    out.put_i32(eid);
    out.put_u8(gm);
    out.put_i8(dimension);
    out.put_u8(difficulty);
    out.put_u8(max_players);
    level_type.encode(&mut out).unwrap();
    out.put_u8(reduced_debug);
    ConversionResult::Converted(vec![build_payload(V18_S2C_JOIN_GAME, &out)])
}

/// 1.12.2 PlayerPosLook: 3×f64, 2×f32, u8 flags, VarInt teleport_id.
/// 1.8:                  3×f64, 2×f32, u8 flags.
/// We strip the trailing teleport_id.
fn s2c_player_pos_look(body: Bytes) -> ConversionResult {
    let mut r = super::safe::Reader::new(body);
    let Some(x) = r.f64() else {
        return ConversionResult::Passthrough;
    };
    let Some(y) = r.f64() else {
        return ConversionResult::Passthrough;
    };
    let Some(z) = r.f64() else {
        return ConversionResult::Passthrough;
    };
    let Some(yaw) = r.f32() else {
        return ConversionResult::Passthrough;
    };
    let Some(pitch) = r.f32() else {
        return ConversionResult::Passthrough;
    };
    let flags = r.u8().unwrap_or(0);
    // teleport_id (VarInt) — read & discard; absence is tolerated for 1.8 input
    let _ = r.varint();

    let mut out = BytesMut::new();
    out.put_f64(x);
    out.put_f64(y);
    out.put_f64(z);
    out.put_f32(yaw);
    out.put_f32(pitch);
    out.put_u8(flags);
    ConversionResult::Converted(vec![build_payload(V18_S2C_PLAYER_POS_LOOK, &out)])
}

/// 1.12.2 Respawn: i32 dimension, u8 difficulty, u8 gamemode, String level_type.
/// 1.8:           i8 dimension, u8 difficulty, u8 gamemode, String level_type.
fn s2c_respawn(body: Bytes) -> ConversionResult {
    let mut r = super::safe::Reader::new(body);
    let dimension = r.i32().unwrap_or(0) as i8;
    let difficulty = r.u8().unwrap_or(2);
    let gamemode = r.u8().unwrap_or(0);
    let level_type = r.string().unwrap_or_else(|| "default".to_owned());

    let mut out = BytesMut::new();
    out.put_i8(dimension);
    out.put_u8(difficulty);
    out.put_u8(gamemode);
    level_type.encode(&mut out).unwrap();
    ConversionResult::Converted(vec![build_payload(V18_S2C_RESPAWN, &out)])
}

/// 1.12.2 KeepAlive uses i64; 1.8 uses VarInt.
fn s2c_keep_alive_long_to_varint(body: Bytes) -> ConversionResult {
    let mut r = super::safe::Reader::new(body);
    let Some(id) = r.i64() else {
        return ConversionResult::Passthrough;
    };
    let mut out = BytesMut::new();
    VarInt(id as i32).encode(&mut out).unwrap();
    ConversionResult::Converted(vec![build_payload(V18_S2C_KEEP_ALIVE, &out)])
}

/// BlockChange: 1.12.2 is (Position, VarInt block_state). 1.8 is
/// (i32 x, u8 y, i32 z, VarInt block_id, u8 metadata). block_state = id<<4 | meta.
fn s2c_block_change(body: Bytes) -> ConversionResult {
    let mut r = super::safe::Reader::new(body);
    let Some(pos) = r.i64() else {
        return ConversionResult::Passthrough;
    };
    let Some(block_state) = r.varint() else {
        return ConversionResult::Passthrough;
    };
    let (x, y, z) = unpack_position(pos);
    let block_id = block_state >> 4;
    let metadata = (block_state & 0xF) as u8;

    let mut out = BytesMut::new();
    out.put_i32(x);
    out.put_u8(y as u8);
    out.put_i32(z);
    VarInt(block_id).encode(&mut out).unwrap();
    out.put_u8(metadata);
    ConversionResult::Converted(vec![build_payload(V18_S2C_BLOCK_CHANGE, &out)])
}

/// 1.12.2 EntityRelMove / EntityLookAndRelMove:
///   VarInt eid, i16 dx, i16 dy, i16 dz, [if has_look: u8 yaw, u8 pitch,] bool on_ground.
/// 1.8 equivalents use i8 deltas. The 1.9+ deltas are in units of 1/4096 of a
/// block; 1.8's are in units of 1/32. So divide by 128 (with sign).
fn s2c_entity_rel_move(body: Bytes, has_look: bool) -> ConversionResult {
    let mut r = super::safe::Reader::new(body);
    let Some(eid) = r.varint() else {
        return ConversionResult::Passthrough;
    };
    let Some(dx) = r.i16() else {
        return ConversionResult::Passthrough;
    };
    let Some(dy) = r.i16() else {
        return ConversionResult::Passthrough;
    };
    let Some(dz) = r.i16() else {
        return ConversionResult::Passthrough;
    };
    let look = if has_look {
        let yaw = r.u8().unwrap_or(0);
        let pitch = r.u8().unwrap_or(0);
        Some((yaw, pitch))
    } else {
        None
    };
    let on_ground = r.u8().unwrap_or(1);

    let dx_8 = (dx / 128).clamp(i8::MIN as i16, i8::MAX as i16) as i8;
    let dy_8 = (dy / 128).clamp(i8::MIN as i16, i8::MAX as i16) as i8;
    let dz_8 = (dz / 128).clamp(i8::MIN as i16, i8::MAX as i16) as i8;

    let mut out = BytesMut::new();
    VarInt(eid).encode(&mut out).unwrap();
    out.put_i8(dx_8);
    out.put_i8(dy_8);
    out.put_i8(dz_8);
    let new_id = if let Some((yaw, pitch)) = look {
        out.put_u8(yaw);
        out.put_u8(pitch);
        V18_S2C_ENTITY_LOOK_REL_MOVE
    } else {
        V18_S2C_ENTITY_REL_MOVE
    };
    out.put_u8(on_ground);
    ConversionResult::Converted(vec![build_payload(new_id, &out)])
}

/// 1.12.2 EntityTeleport: VarInt eid, f64 x, f64 y, f64 z, u8 yaw, u8 pitch, bool on_ground.
/// 1.8 EntityTeleport:    VarInt eid, i32 x, i32 y, i32 z, u8 yaw, u8 pitch, bool on_ground.
/// 1.8 uses *fixed-point* coords = block_coord * 32. Multiply f64 by 32 and round.
fn s2c_entity_teleport(body: Bytes) -> ConversionResult {
    let mut r = super::safe::Reader::new(body);
    let Some(eid) = r.varint() else {
        return ConversionResult::Passthrough;
    };
    let Some(x) = r.f64() else {
        return ConversionResult::Passthrough;
    };
    let Some(y) = r.f64() else {
        return ConversionResult::Passthrough;
    };
    let Some(z) = r.f64() else {
        return ConversionResult::Passthrough;
    };
    let yaw = r.u8().unwrap_or(0);
    let pitch = r.u8().unwrap_or(0);
    let on_ground = r.u8().unwrap_or(1);

    let mut out = BytesMut::new();
    VarInt(eid).encode(&mut out).unwrap();
    out.put_i32((x * 32.0).round() as i32);
    out.put_i32((y * 32.0).round() as i32);
    out.put_i32((z * 32.0).round() as i32);
    out.put_u8(yaw);
    out.put_u8(pitch);
    out.put_u8(on_ground);
    ConversionResult::Converted(vec![build_payload(V18_S2C_ENTITY_TELEPORT, &out)])
}

/// 1.12.2 CollectItem: VarInt collected, VarInt collector, VarInt count.
/// 1.8 CollectItem:    VarInt collected, VarInt collector.
/// We strip the count for the 1.8 client.
fn s2c_collect_item(body: Bytes) -> ConversionResult {
    let mut r = super::safe::Reader::new(body);
    let Some(collected) = r.varint() else {
        return ConversionResult::Passthrough;
    };
    let Some(collector) = r.varint() else {
        return ConversionResult::Passthrough;
    };
    let _count = r.varint();

    let mut out = BytesMut::new();
    VarInt(collected).encode(&mut out).unwrap();
    VarInt(collector).encode(&mut out).unwrap();
    ConversionResult::Converted(vec![build_payload(V18_S2C_COLLECT_ITEM, &out)])
}

fn unpack_position(packed: i64) -> (i32, i32, i32) {
    // 1.8–1.13 Position layout (changed to (x|z|y) in 1.14):
    //   signed 26-bit x in bits 38–63,
    //   signed 12-bit y in bits 26–37,
    //   signed 26-bit z in bits  0–25.
    // Arithmetic right-shift on i64 sign-extends, which we rely on for x/z.
    let x = (packed >> 38) as i32;
    let mut y = ((packed >> 26) & 0xFFF) as i32;
    if y >= 1 << 11 {
        y -= 1 << 12;
    }
    let z = ((packed << 38) >> 38) as i32;
    (x, y, z)
}

// ── items.rs helper bridge ────────────────────────────────────────────────

// Compatibility shim used by dispatch_1_16 for SetSlot in older slot format.
// Implemented inline in items.rs as map_set_slot_legacy.

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Buf;
    use kojacoord_protocol::codec::Decode;

    fn convert(id: u8, body: &[u8], proto: u32) -> Option<(u8, Bytes)> {
        let mut full = BytesMut::new();
        VarInt(id as i32).encode(&mut full).unwrap();
        full.extend_from_slice(body);
        match convert_s2c(full.freeze(), proto) {
            ConversionResult::Converted(mut pkts) if pkts.len() == 1 => {
                let mut p = pkts.remove(0);
                let new_id = VarInt::decode(&mut p).ok()?.0 as u8;
                Some((new_id, p))
            },
            _ => None,
        }
    }

    fn pack_position(x: i32, y: i32, z: i32) -> i64 {
        // 1.8/1.12.2 layout — Y is in the middle 12 bits, not the low 12.
        (((x as i64) & 0x3FF_FFFF) << 38) | (((y as i64) & 0xFFF) << 26) | ((z as i64) & 0x3FF_FFFF)
    }

    #[test]
    fn keep_alive_long_to_varint() {
        let mut body = BytesMut::new();
        body.put_i64(99);
        let (id, mut rest) = convert(V112_S2C_KEEP_ALIVE, &body, 340).unwrap();
        assert_eq!(id, V18_S2C_KEEP_ALIVE);
        assert_eq!(VarInt::decode(&mut rest).unwrap().0, 99);
    }

    #[test]
    fn join_game_dimension_narrows() {
        let mut body = BytesMut::new();
        body.put_i32(7); // eid
        body.put_u8(1); // gamemode
        body.put_i32(-1); // dimension nether
        body.put_u8(2); // difficulty
        body.put_u8(20); // max players
        "default".to_owned().encode(&mut body).unwrap();
        body.put_u8(0); // reduced_debug
        let (id, mut rest) = convert(V112_S2C_JOIN_GAME, &body, 340).unwrap();
        assert_eq!(id, V18_S2C_JOIN_GAME);
        assert_eq!(rest.get_i32(), 7);
        assert_eq!(rest.get_u8(), 1);
        assert_eq!(rest.get_i8(), -1);
    }

    #[test]
    fn player_pos_look_strips_teleport_id() {
        let mut body = BytesMut::new();
        body.put_f64(1.0);
        body.put_f64(2.0);
        body.put_f64(3.0);
        body.put_f32(10.0);
        body.put_f32(20.0);
        body.put_u8(0);
        VarInt(42).encode(&mut body).unwrap();
        let (id, rest) = convert(V112_S2C_PLAYER_POS_LOOK, &body, 340).unwrap();
        assert_eq!(id, V18_S2C_PLAYER_POS_LOOK);
        // 3×8 + 2×4 + 1 = 33 bytes, no trailing teleport_id
        assert_eq!(rest.len(), 33);
    }

    #[test]
    fn block_change_unpacks_position_and_state() {
        let pos = pack_position(100, 64, -50);
        let block_state = (5_i32 << 4) | 7; // id=5, meta=7
        let mut body = BytesMut::new();
        body.put_i64(pos);
        VarInt(block_state).encode(&mut body).unwrap();
        let (id, mut rest) = convert(V112_S2C_BLOCK_CHANGE, &body, 340).unwrap();
        assert_eq!(id, V18_S2C_BLOCK_CHANGE);
        assert_eq!(rest.get_i32(), 100);
        assert_eq!(rest.get_u8(), 64);
        assert_eq!(rest.get_i32(), -50);
        assert_eq!(VarInt::decode(&mut rest).unwrap().0, 5);
        assert_eq!(rest.get_u8(), 7);
    }

    #[test]
    fn entity_teleport_scales_to_fixed_point() {
        let mut body = BytesMut::new();
        VarInt(10).encode(&mut body).unwrap();
        body.put_f64(1.5); // x
        body.put_f64(64.0); // y
        body.put_f64(-2.25); // z
        body.put_u8(0);
        body.put_u8(0);
        body.put_u8(1);
        let (id, mut rest) = convert(V112_S2C_ENTITY_TELEPORT, &body, 340).unwrap();
        assert_eq!(id, V18_S2C_ENTITY_TELEPORT);
        assert_eq!(VarInt::decode(&mut rest).unwrap().0, 10);
        assert_eq!(rest.get_i32(), 48); // 1.5 * 32
        assert_eq!(rest.get_i32(), 2048); // 64 * 32
        assert_eq!(rest.get_i32(), -72); // -2.25 * 32
    }

    #[test]
    fn entity_rel_move_scales_delta() {
        let mut body = BytesMut::new();
        VarInt(7).encode(&mut body).unwrap();
        body.put_i16(256); // dx in 1/4096 → 2 in 1/32
        body.put_i16(-128); // dy → -1
        body.put_i16(0);
        body.put_u8(1);
        let (id, mut rest) = convert(V112_S2C_ENTITY_REL_MOVE, &body, 340).unwrap();
        assert_eq!(id, V18_S2C_ENTITY_REL_MOVE);
        assert_eq!(VarInt::decode(&mut rest).unwrap().0, 7);
        assert_eq!(rest.get_i8(), 2);
        assert_eq!(rest.get_i8(), -1);
        assert_eq!(rest.get_i8(), 0);
        assert_eq!(rest.get_u8(), 1);
    }

    #[test]
    fn boss_bar_drops() {
        let mut full = BytesMut::new();
        VarInt(V112_S2C_BOSS_BAR as i32).encode(&mut full).unwrap();
        match convert_s2c(full.freeze(), 340) {
            ConversionResult::Drop => {},
            _ => panic!("BossBar should drop"),
        }
    }
}
