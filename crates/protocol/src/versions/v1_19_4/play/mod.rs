use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::codec::{Decode, Encode, PacketId};
use crate::error::ProtocolError;
use crate::types::VarInt;

pub use packets::{
    ClientboundAcknowledgeBlockChange, ClientboundAwardStats, ClientboundBlockAction,
    ClientboundBlockDestroyStage, ClientboundBlockEntityData, ClientboundBlockUpdate,
    ClientboundBossBar, ClientboundBundle, ClientboundChangeDifficulty, ClientboundChunkBiomes,
    ClientboundClearTitles, ClientboundCommandSuggestions, ClientboundCommands,
    ClientboundContainerClose, ClientboundContainerSetContent, ClientboundContainerSetProperty,
    ClientboundContainerSetSlot, ClientboundCooldown, ClientboundCustomChatCompletions,
    ClientboundDamageEvent, ClientboundDeleteChat, ClientboundDisconnect, ClientboundDisguisedChat,
    ClientboundEntityAnimation, ClientboundEntityEvent, ClientboundExplosion,
    ClientboundForgetLevelChunk, ClientboundGameEvent, ClientboundHorseScreenOpen,
    ClientboundHurtAnimation, ClientboundInitializeBorder, ClientboundKeepAlive,
    ClientboundLevelChunkWithLight, ClientboundLevelEvent, ClientboundLevelParticles,
    ClientboundLightUpdate, ClientboundLogin, ClientboundMapItemData, ClientboundMerchantOffers,
    ClientboundMoveEntityPos, ClientboundMoveEntityPosRot, ClientboundMoveEntityRot,
    ClientboundMoveVehicle, ClientboundOpenBook, ClientboundOpenScreen, ClientboundOpenSignEditor,
    ClientboundPing, ClientboundPlaceGhostRecipe, ClientboundPlayerAbilities,
    ClientboundPlayerChat, ClientboundPlayerCombatEnd, ClientboundPlayerCombatEnter,
    ClientboundPlayerCombatKill, ClientboundPlayerInfoRemove, ClientboundPlayerInfoUpdate,
    ClientboundPlayerLookAt, ClientboundPlayerPosition, ClientboundPluginMessage,
    ClientboundRecipeBookSettings, ClientboundRecipes, ClientboundRemoveEntities,
    ClientboundRemoveEntityEffect, ClientboundResourcePackPush, ClientboundRespawn,
    ClientboundRotateHead, ClientboundSectionBlocksUpdate, ClientboundSelectAdvancementsTab,
    ClientboundServerData, ClientboundSetActionBarText, ClientboundSetBorderCenter,
    ClientboundSetBorderLerpSize, ClientboundSetBorderSize, ClientboundSetBorderWarningDelay,
    ClientboundSetBorderWarningDistance, ClientboundSetCamera, ClientboundSetCarriedItem,
    ClientboundSetEntityLink, ClientboundSetEntityMotion, ClientboundSetEquipment,
    ClientboundSetExperience, ClientboundSetHealth, ClientboundSetScoreboardObjective,
    ClientboundSetScoreboardScore, ClientboundSetSimulationDistance, ClientboundSetSubtitleText,
    ClientboundSetTime, ClientboundSetTitleAnimationTimes, ClientboundSetTitleText,
    ClientboundSound, ClientboundSoundEntity, ClientboundSpawnEntity,
    ClientboundSpawnExperienceOrb, ClientboundSpawnPlayer, ClientboundStopSound,
    ClientboundSystemChat, ClientboundTabList, ClientboundTagQuery, ClientboundTakeItemEntity,
    ClientboundTeleportEntity, ClientboundUpdateAdvancements, ClientboundUpdateAttributes,
    ClientboundUpdateEffects, ClientboundUpdateRecipes, ClientboundUpdateTags, InteractAction,
    ServerboundAcceptTeleportation, ServerboundChatCommand, ServerboundChatMessage,
    ServerboundChatSessionUpdate, ServerboundClickWindow, ServerboundClickWindowButton,
    ServerboundClientInformation, ServerboundClientStatus, ServerboundCloseWindow,
    ServerboundCommandSuggestion, ServerboundCreativeInventoryAction, ServerboundDifficultyChange,
    ServerboundDifficultyLock, ServerboundEditBook, ServerboundEntityAction,
    ServerboundEntityTagQuery, ServerboundInteract, ServerboundJigsawGenerate,
    ServerboundKeepAlive, ServerboundMovePlayerPos, ServerboundMovePlayerPosRot,
    ServerboundMovePlayerRot, ServerboundMovePlayerStatusOnly, ServerboundPaddleBoat,
    ServerboundPickItem, ServerboundPlaceRecipe, ServerboundPlayerAbilities,
    ServerboundPlayerAction, ServerboundPluginMessage, ServerboundPong,
    ServerboundRecipeBookChangeSettings, ServerboundRecipeBookSeenRecipe, ServerboundRenameItem,
    ServerboundResourcePackStatus, ServerboundSelectTrade, ServerboundSetBeaconEffect,
    ServerboundSetCarriedItem, ServerboundSetStructureBlock, ServerboundSpectate,
    ServerboundSteerBoat, ServerboundSteerVehicle, ServerboundSwingArm,
    ServerboundUpdateCommandBlock, ServerboundUpdateCommandBlockMinecart, ServerboundUpdateSign,
    ServerboundUseItem, ServerboundUseItemOn, ServerboundVehicleMove,
};

fn need(src: &Bytes, n: usize) -> Result<(), ProtocolError> {
    if src.remaining() < n {
        return Err(ProtocolError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Not enough bytes",
        )));
    }
    Ok(())
}

mod packets {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundBundle;

    impl PacketId for ClientboundBundle {
        fn packet_id(_ver: u32) -> u8 {
            0x00
        }
    }
    impl Encode for ClientboundBundle {
        fn encode(&self, _dst: &mut BytesMut) -> Result<(), ProtocolError> {
            Ok(())
        }
    }
    impl Decode for ClientboundBundle {
        fn decode(_src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self)
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundDisconnect {
        pub reason: String,
    }
    impl PacketId for ClientboundDisconnect {
        fn packet_id(_ver: u32) -> u8 {
            0x1A
        }
    }
    impl Encode for ClientboundDisconnect {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.reason.encode(dst)
        }
    }
    impl Decode for ClientboundDisconnect {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                reason: String::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundKeepAlive {
        pub id: i64,
    }
    impl PacketId for ClientboundKeepAlive {
        fn packet_id(_ver: u32) -> u8 {
            0x24
        }
    }
    impl Encode for ClientboundKeepAlive {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.id.encode(dst)
        }
    }
    impl Decode for ClientboundKeepAlive {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                id: i64::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundLogin {
        pub entity_id: i32,
        pub is_hardcore: bool,
        pub game_mode: u8,
        pub previous_game_mode: i8,
        pub dimensions: Vec<String>,

        pub registry_codec: Vec<u8>,
        pub dimension_type: String,
        pub dimension_name: String,
        pub hashed_seed: i64,
        pub max_players: VarInt,
        pub chunk_radius: VarInt,
        pub simulation_distance: VarInt,
        pub reduced_debug_info: bool,
        pub enable_respawn_screen: bool,
        pub is_debug: bool,
        pub is_flat: bool,
        pub death_location: Option<(String, i64)>,
    }
    impl PacketId for ClientboundLogin {
        fn packet_id(_ver: u32) -> u8 {
            0x29
        }
    }
    impl Encode for ClientboundLogin {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            self.is_hardcore.encode(dst)?;
            self.game_mode.encode(dst)?;
            self.previous_game_mode.encode(dst)?;
            self.dimensions.encode(dst)?;
            VarInt(self.registry_codec.len() as i32).encode(dst)?;
            dst.extend_from_slice(&self.registry_codec);
            self.dimension_type.encode(dst)?;
            self.dimension_name.encode(dst)?;
            self.hashed_seed.encode(dst)?;
            self.max_players.encode(dst)?;
            self.chunk_radius.encode(dst)?;
            self.simulation_distance.encode(dst)?;
            self.reduced_debug_info.encode(dst)?;
            self.enable_respawn_screen.encode(dst)?;
            self.is_debug.encode(dst)?;
            self.is_flat.encode(dst)?;
            match &self.death_location {
                Some((dim, pos)) => {
                    true.encode(dst)?;
                    dim.encode(dst)?;
                    pos.encode(dst)?;
                },
                None => false.encode(dst)?,
            }
            Ok(())
        }
    }
    impl Decode for ClientboundLogin {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = i32::decode(src)?;
            let is_hardcore = bool::decode(src)?;
            let game_mode = u8::decode(src)?;
            let previous_game_mode = i8::decode(src)?;
            let dimensions = Vec::<String>::decode(src)?;
            let codec_len = VarInt::decode(src)?.0 as usize;
            let registry_codec = src.copy_to_bytes(codec_len).to_vec();
            let dimension_type = String::decode(src)?;
            let dimension_name = String::decode(src)?;
            let hashed_seed = i64::decode(src)?;
            let max_players = VarInt::decode(src)?;
            let chunk_radius = VarInt::decode(src)?;
            let simulation_distance = VarInt::decode(src)?;
            let reduced_debug_info = bool::decode(src)?;
            let enable_respawn_screen = bool::decode(src)?;
            let is_debug = bool::decode(src)?;
            let is_flat = bool::decode(src)?;
            let has_death = bool::decode(src)?;
            let death_location = if has_death {
                Some((String::decode(src)?, i64::decode(src)?))
            } else {
                None
            };
            Ok(Self {
                entity_id,
                is_hardcore,
                game_mode,
                previous_game_mode,
                dimensions,
                registry_codec,
                dimension_type,
                dimension_name,
                hashed_seed,
                max_players,
                chunk_radius,
                simulation_distance,
                reduced_debug_info,
                enable_respawn_screen,
                is_debug,
                is_flat,
                death_location,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundPluginMessage {
        pub channel: String,
        pub data: Vec<u8>,
    }
    impl PacketId for ClientboundPluginMessage {
        fn packet_id(_ver: u32) -> u8 {
            0x17
        }
    }
    impl Encode for ClientboundPluginMessage {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.channel.encode(dst)?;
            dst.extend_from_slice(&self.data);
            Ok(())
        }
    }
    impl Decode for ClientboundPluginMessage {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let channel = String::decode(src)?;
            let data = src.copy_to_bytes(src.remaining()).to_vec();
            Ok(Self { channel, data })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundPlayerPosition {
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub yaw: f32,
        pub pitch: f32,
        pub flags: u8,
        pub teleport_id: VarInt,
    }
    impl PacketId for ClientboundPlayerPosition {
        fn packet_id(_ver: u32) -> u8 {
            0x3E
        }
    }
    impl Encode for ClientboundPlayerPosition {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.x.encode(dst)?;
            self.y.encode(dst)?;
            self.z.encode(dst)?;
            self.yaw.encode(dst)?;
            self.pitch.encode(dst)?;
            self.flags.encode(dst)?;
            self.teleport_id.encode(dst)
        }
    }
    impl Decode for ClientboundPlayerPosition {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                x: f64::decode(src)?,
                y: f64::decode(src)?,
                z: f64::decode(src)?,
                yaw: f32::decode(src)?,
                pitch: f32::decode(src)?,
                flags: u8::decode(src)?,
                teleport_id: VarInt::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundRespawn {
        pub dimension_type: String,
        pub dimension_name: String,
        pub hashed_seed: i64,
        pub game_mode: u8,
        pub previous_game_mode: i8,
        pub is_debug: bool,
        pub is_flat: bool,
        pub data_kept: u8,
        pub death_location: Option<(String, i64)>,
    }
    impl PacketId for ClientboundRespawn {
        fn packet_id(_ver: u32) -> u8 {
            0x44
        }
    }
    impl Encode for ClientboundRespawn {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.dimension_type.encode(dst)?;
            self.dimension_name.encode(dst)?;
            self.hashed_seed.encode(dst)?;
            self.game_mode.encode(dst)?;
            self.previous_game_mode.encode(dst)?;
            self.is_debug.encode(dst)?;
            self.is_flat.encode(dst)?;
            self.data_kept.encode(dst)?;
            match &self.death_location {
                Some((dim, pos)) => {
                    true.encode(dst)?;
                    dim.encode(dst)?;
                    pos.encode(dst)?;
                },
                None => false.encode(dst)?,
            }
            Ok(())
        }
    }
    impl Decode for ClientboundRespawn {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let dimension_type = String::decode(src)?;
            let dimension_name = String::decode(src)?;
            let hashed_seed = i64::decode(src)?;
            let game_mode = u8::decode(src)?;
            let previous_game_mode = i8::decode(src)?;
            let is_debug = bool::decode(src)?;
            let is_flat = bool::decode(src)?;
            let data_kept = u8::decode(src)?;
            let has_death = bool::decode(src)?;
            let death_location = if has_death {
                Some((String::decode(src)?, i64::decode(src)?))
            } else {
                None
            };
            Ok(Self {
                dimension_type,
                dimension_name,
                hashed_seed,
                game_mode,
                previous_game_mode,
                is_debug,
                is_flat,
                data_kept,
                death_location,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundPlayerAbilities {
        pub flags: u8,
        pub flying_speed: f32,
        pub walking_speed: f32,
    }
    impl PacketId for ClientboundPlayerAbilities {
        fn packet_id(_ver: u32) -> u8 {
            0x36
        }
    }
    impl Encode for ClientboundPlayerAbilities {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.flags.encode(dst)?;
            self.flying_speed.encode(dst)?;
            self.walking_speed.encode(dst)
        }
    }
    impl Decode for ClientboundPlayerAbilities {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                flags: u8::decode(src)?,
                flying_speed: f32::decode(src)?,
                walking_speed: f32::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSystemChat {
        pub content: String,
        pub overlay: bool,
    }
    impl PacketId for ClientboundSystemChat {
        fn packet_id(_ver: u32) -> u8 {
            0x62
        }
    }
    impl Encode for ClientboundSystemChat {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.content.encode(dst)?;
            self.overlay.encode(dst)
        }
    }
    impl Decode for ClientboundSystemChat {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                content: String::decode(src)?,
                overlay: bool::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetTime {
        pub world_age: i64,
        pub time_of_day: i64,
    }
    impl PacketId for ClientboundSetTime {
        fn packet_id(_ver: u32) -> u8 {
            0x59
        }
    }
    impl Encode for ClientboundSetTime {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.world_age.encode(dst)?;
            self.time_of_day.encode(dst)
        }
    }
    impl Decode for ClientboundSetTime {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                world_age: i64::decode(src)?,
                time_of_day: i64::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundEntityEvent {
        pub entity_id: i32,
        pub event_id: i8,
    }
    impl PacketId for ClientboundEntityEvent {
        fn packet_id(_ver: u32) -> u8 {
            0x1C
        }
    }
    impl Encode for ClientboundEntityEvent {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            self.event_id.encode(dst)
        }
    }
    impl Decode for ClientboundEntityEvent {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                entity_id: i32::decode(src)?,
                event_id: i8::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetCarriedItem {
        pub slot: i8,
    }
    impl PacketId for ClientboundSetCarriedItem {
        fn packet_id(_ver: u32) -> u8 {
            0x4A
        }
    }
    impl Encode for ClientboundSetCarriedItem {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.slot.encode(dst)
        }
    }
    impl Decode for ClientboundSetCarriedItem {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                slot: i8::decode(src)?,
            })
        }
    }

    macro_rules! passthrough {
        ($(#[doc = $doc:expr])* $name:ident, $id:expr) => {
            $(#[doc = $doc])*

            #[derive(Debug, Clone, PartialEq)]
            pub struct $name { pub raw: Vec<u8> }
            impl PacketId for $name {
                fn packet_id(_ver: u32) -> u8 { $id }
            }
            impl Encode for $name {
                fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
                    dst.extend_from_slice(&self.raw);
                    Ok(())
                }
            }
            impl Decode for $name {
                fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
                    let raw = src.copy_to_bytes(src.remaining()).to_vec();
                    Ok(Self { raw })
                }
            }
        };
    }

    passthrough!(ClientboundSpawnEntity, 0x01);
    passthrough!(ClientboundSpawnExperienceOrb, 0x02);
    passthrough!(ClientboundSpawnPlayer, 0x03);
    passthrough!(ClientboundEntityAnimation, 0x04);
    passthrough!(ClientboundAwardStats, 0x05);
    passthrough!(ClientboundAcknowledgeBlockChange, 0x06);
    passthrough!(ClientboundBlockDestroyStage, 0x07);
    passthrough!(ClientboundBlockEntityData, 0x08);
    passthrough!(ClientboundBlockAction, 0x09);
    passthrough!(ClientboundBlockUpdate, 0x0A);
    passthrough!(ClientboundBossBar, 0x0B);
    passthrough!(ClientboundChangeDifficulty, 0x0C);
    passthrough!(ClientboundChunkBiomes, 0x0D);
    passthrough!(ClientboundClearTitles, 0x0E);
    passthrough!(ClientboundCommandSuggestions, 0x0F);
    passthrough!(ClientboundCommands, 0x10);
    passthrough!(ClientboundContainerClose, 0x11);
    passthrough!(ClientboundContainerSetProperty, 0x13);

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundContainerSetContent {
        pub window_id: u8,
        pub slots: Vec<crate::types::Slot>,
        pub carried_item: crate::types::Slot,
    }

    impl PacketId for ClientboundContainerSetContent {
        fn packet_id(_ver: u32) -> u8 {
            0x12
        }
    }

    impl Encode for ClientboundContainerSetContent {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.window_id);
            VarInt(self.slots.len() as i32).encode(dst)?;
            for slot in &self.slots {
                slot.encode(dst)?;
            }
            self.carried_item.encode(dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundContainerSetContent {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1)?;
            let window_id = src.get_u8();
            let count = VarInt::decode(src)?.0 as usize;
            let mut slots = Vec::with_capacity(count);
            for _ in 0..count {
                slots.push(crate::types::Slot::decode(src)?);
            }
            let carried_item = crate::types::Slot::decode(src)?;
            Ok(Self {
                window_id,
                slots,
                carried_item,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundContainerSetSlot {
        pub window_id: i8,
        pub slot: i16,
        pub slot_data: crate::types::Slot,
    }

    impl PacketId for ClientboundContainerSetSlot {
        fn packet_id(_ver: u32) -> u8 {
            0x14
        }
    }

    impl Encode for ClientboundContainerSetSlot {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i8(self.window_id);
            dst.put_i16(self.slot);
            self.slot_data.encode(dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundContainerSetSlot {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1 + 2)?;
            let window_id = src.get_i8();
            let slot = src.get_i16();
            let slot_data = crate::types::Slot::decode(src)?;
            Ok(Self {
                window_id,
                slot,
                slot_data,
            })
        }
    }

    passthrough!(ClientboundCooldown, 0x15);
    passthrough!(ClientboundCustomChatCompletions, 0x16);
    passthrough!(ClientboundDamageEvent, 0x18);
    passthrough!(ClientboundDeleteChat, 0x19);
    passthrough!(ClientboundDisguisedChat, 0x1B);
    passthrough!(ClientboundExplosion, 0x1D);
    passthrough!(ClientboundForgetLevelChunk, 0x1E);
    passthrough!(ClientboundGameEvent, 0x1F);
    passthrough!(ClientboundHorseScreenOpen, 0x20);
    passthrough!(ClientboundHurtAnimation, 0x21);
    passthrough!(ClientboundInitializeBorder, 0x22);
    passthrough!(ClientboundLevelChunkWithLight, 0x25);
    passthrough!(ClientboundLevelEvent, 0x26);
    passthrough!(ClientboundLevelParticles, 0x27);
    passthrough!(ClientboundLightUpdate, 0x28);
    passthrough!(ClientboundMapItemData, 0x2A);
    passthrough!(ClientboundMerchantOffers, 0x2B);
    passthrough!(ClientboundMoveEntityPos, 0x2C);
    passthrough!(ClientboundMoveEntityPosRot, 0x2D);
    passthrough!(ClientboundMoveEntityRot, 0x2E);
    passthrough!(ClientboundMoveVehicle, 0x2F);
    passthrough!(ClientboundOpenBook, 0x30);
    passthrough!(ClientboundOpenScreen, 0x31);
    passthrough!(ClientboundOpenSignEditor, 0x32);
    passthrough!(ClientboundPing, 0x33);
    passthrough!(ClientboundPlaceGhostRecipe, 0x35);
    passthrough!(ClientboundPlayerChat, 0x37);
    passthrough!(ClientboundPlayerCombatEnd, 0x38);
    passthrough!(ClientboundPlayerCombatEnter, 0x39);
    passthrough!(ClientboundPlayerCombatKill, 0x3A);
    passthrough!(ClientboundPlayerInfoRemove, 0x3B);
    passthrough!(ClientboundPlayerInfoUpdate, 0x3C);
    passthrough!(ClientboundPlayerLookAt, 0x3D);
    passthrough!(ClientboundRecipeBookSettings, 0x3F);
    passthrough!(ClientboundRecipes, 0x40);
    passthrough!(ClientboundRemoveEntities, 0x41);
    passthrough!(ClientboundRemoveEntityEffect, 0x42);
    passthrough!(ClientboundResourcePackPush, 0x43);
    passthrough!(ClientboundRotateHead, 0x45);
    passthrough!(ClientboundSectionBlocksUpdate, 0x46);
    passthrough!(ClientboundSelectAdvancementsTab, 0x47);
    passthrough!(ClientboundServerData, 0x48);
    passthrough!(ClientboundSetActionBarText, 0x49);
    passthrough!(ClientboundSetBorderCenter, 0x4A);
    passthrough!(ClientboundSetBorderLerpSize, 0x4B);
    passthrough!(ClientboundSetBorderSize, 0x4C);
    passthrough!(ClientboundSetBorderWarningDelay, 0x4D);
    passthrough!(ClientboundSetBorderWarningDistance, 0x4E);
    passthrough!(ClientboundSetCamera, 0x4F);
    passthrough!(ClientboundSetEntityLink, 0x50);
    passthrough!(ClientboundSetEntityMotion, 0x51);
    passthrough!(ClientboundSetEquipment, 0x52);
    passthrough!(ClientboundSetExperience, 0x53);
    passthrough!(ClientboundSetHealth, 0x54);
    passthrough!(ClientboundSetScoreboardObjective, 0x55);
    passthrough!(ClientboundSetScoreboardScore, 0x56);
    passthrough!(ClientboundSetSimulationDistance, 0x57);
    passthrough!(ClientboundSetSubtitleText, 0x58);
    passthrough!(ClientboundSetTitleText, 0x5A);
    passthrough!(ClientboundSetTitleAnimationTimes, 0x5B);
    passthrough!(ClientboundSoundEntity, 0x5C);
    passthrough!(ClientboundSound, 0x5D);
    passthrough!(ClientboundStopSound, 0x5E);
    passthrough!(ClientboundTabList, 0x63);
    passthrough!(ClientboundTagQuery, 0x64);
    passthrough!(ClientboundTakeItemEntity, 0x65);
    passthrough!(ClientboundTeleportEntity, 0x66);
    passthrough!(ClientboundUpdateAdvancements, 0x67);
    passthrough!(ClientboundUpdateAttributes, 0x68);
    passthrough!(ClientboundUpdateEffects, 0x69);
    passthrough!(ClientboundUpdateRecipes, 0x6A);
    passthrough!(ClientboundUpdateTags, 0x6B);

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundAcceptTeleportation {
        pub teleport_id: VarInt,
    }
    impl PacketId for ServerboundAcceptTeleportation {
        fn packet_id(_ver: u32) -> u8 {
            0x00
        }
    }
    impl Encode for ServerboundAcceptTeleportation {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.teleport_id.encode(dst)
        }
    }
    impl Decode for ServerboundAcceptTeleportation {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                teleport_id: VarInt::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundClientInformation {
        pub locale: String,
        pub view_distance: i8,
        pub chat_mode: VarInt,
        pub chat_colors: bool,
        pub displayed_skin_parts: u8,
        pub main_hand: VarInt,
        pub enable_text_filtering: bool,
        pub allow_server_listings: bool,
    }
    impl PacketId for ServerboundClientInformation {
        fn packet_id(_ver: u32) -> u8 {
            0x07
        }
    }
    impl Encode for ServerboundClientInformation {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.locale.encode(dst)?;
            self.view_distance.encode(dst)?;
            self.chat_mode.encode(dst)?;
            self.chat_colors.encode(dst)?;
            self.displayed_skin_parts.encode(dst)?;
            self.main_hand.encode(dst)?;
            self.enable_text_filtering.encode(dst)?;
            self.allow_server_listings.encode(dst)
        }
    }
    impl Decode for ServerboundClientInformation {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                locale: String::decode(src)?,
                view_distance: i8::decode(src)?,
                chat_mode: VarInt::decode(src)?,
                chat_colors: bool::decode(src)?,
                displayed_skin_parts: u8::decode(src)?,
                main_hand: VarInt::decode(src)?,
                enable_text_filtering: bool::decode(src)?,
                allow_server_listings: bool::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum InteractAction {
        Interact {
            hand: VarInt,
        },
        Attack,
        InteractAt {
            target_x: f32,
            target_y: f32,
            target_z: f32,
            hand: VarInt,
        },
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundInteract {
        pub entity_id: VarInt,
        pub action: InteractAction,
        pub sneaking: bool,
    }
    impl PacketId for ServerboundInteract {
        fn packet_id(_ver: u32) -> u8 {
            0x11
        }
    }
    impl Encode for ServerboundInteract {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            match &self.action {
                InteractAction::Interact { hand } => {
                    VarInt(0).encode(dst)?;
                    hand.encode(dst)?;
                },
                InteractAction::Attack => {
                    VarInt(1).encode(dst)?;
                },
                InteractAction::InteractAt {
                    target_x,
                    target_y,
                    target_z,
                    hand,
                } => {
                    VarInt(2).encode(dst)?;
                    target_x.encode(dst)?;
                    target_y.encode(dst)?;
                    target_z.encode(dst)?;
                    hand.encode(dst)?;
                },
            }
            self.sneaking.encode(dst)
        }
    }
    impl Decode for ServerboundInteract {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            let action = match VarInt::decode(src)?.0 {
                0 => InteractAction::Interact {
                    hand: VarInt::decode(src)?,
                },
                1 => InteractAction::Attack,
                2 => InteractAction::InteractAt {
                    target_x: f32::decode(src)?,
                    target_y: f32::decode(src)?,
                    target_z: f32::decode(src)?,
                    hand: VarInt::decode(src)?,
                },
                _ => return Err(ProtocolError::UnexpectedEof),
            };
            let sneaking = bool::decode(src)?;
            Ok(Self {
                entity_id,
                action,
                sneaking,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundKeepAlive {
        pub id: i64,
    }
    impl PacketId for ServerboundKeepAlive {
        fn packet_id(_ver: u32) -> u8 {
            0x12
        }
    }
    impl Encode for ServerboundKeepAlive {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.id.encode(dst)
        }
    }
    impl Decode for ServerboundKeepAlive {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                id: i64::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundMovePlayerStatusOnly {
        pub on_ground: bool,
    }
    impl PacketId for ServerboundMovePlayerStatusOnly {
        fn packet_id(_ver: u32) -> u8 {
            0x13
        }
    }
    impl Encode for ServerboundMovePlayerStatusOnly {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.on_ground.encode(dst)
        }
    }
    impl Decode for ServerboundMovePlayerStatusOnly {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                on_ground: bool::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundMovePlayerPos {
        pub x: f64,
        pub feet_y: f64,
        pub z: f64,
        pub on_ground: bool,
    }
    impl PacketId for ServerboundMovePlayerPos {
        fn packet_id(_ver: u32) -> u8 {
            0x14
        }
    }
    impl Encode for ServerboundMovePlayerPos {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.x.encode(dst)?;
            self.feet_y.encode(dst)?;
            self.z.encode(dst)?;
            self.on_ground.encode(dst)
        }
    }
    impl Decode for ServerboundMovePlayerPos {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                x: f64::decode(src)?,
                feet_y: f64::decode(src)?,
                z: f64::decode(src)?,
                on_ground: bool::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundMovePlayerRot {
        pub yaw: f32,
        pub pitch: f32,
        pub on_ground: bool,
    }
    impl PacketId for ServerboundMovePlayerRot {
        fn packet_id(_ver: u32) -> u8 {
            0x15
        }
    }
    impl Encode for ServerboundMovePlayerRot {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.yaw.encode(dst)?;
            self.pitch.encode(dst)?;
            self.on_ground.encode(dst)
        }
    }
    impl Decode for ServerboundMovePlayerRot {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                yaw: f32::decode(src)?,
                pitch: f32::decode(src)?,
                on_ground: bool::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundMovePlayerPosRot {
        pub x: f64,
        pub feet_y: f64,
        pub z: f64,
        pub yaw: f32,
        pub pitch: f32,
        pub on_ground: bool,
    }
    impl PacketId for ServerboundMovePlayerPosRot {
        fn packet_id(_ver: u32) -> u8 {
            0x16
        }
    }
    impl Encode for ServerboundMovePlayerPosRot {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.x.encode(dst)?;
            self.feet_y.encode(dst)?;
            self.z.encode(dst)?;
            self.yaw.encode(dst)?;
            self.pitch.encode(dst)?;
            self.on_ground.encode(dst)
        }
    }
    impl Decode for ServerboundMovePlayerPosRot {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                x: f64::decode(src)?,
                feet_y: f64::decode(src)?,
                z: f64::decode(src)?,
                yaw: f32::decode(src)?,
                pitch: f32::decode(src)?,
                on_ground: bool::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundPluginMessage {
        pub channel: String,
        pub data: Vec<u8>,
    }
    impl PacketId for ServerboundPluginMessage {
        fn packet_id(_ver: u32) -> u8 {
            0x0C
        }
    }
    impl Encode for ServerboundPluginMessage {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.channel.encode(dst)?;
            dst.extend_from_slice(&self.data);
            Ok(())
        }
    }
    impl Decode for ServerboundPluginMessage {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let channel = String::decode(src)?;
            let data = src.copy_to_bytes(src.remaining()).to_vec();
            Ok(Self { channel, data })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundPlayerAbilities {
        pub flags: u8,
    }
    impl PacketId for ServerboundPlayerAbilities {
        fn packet_id(_ver: u32) -> u8 {
            0x1B
        }
    }
    impl Encode for ServerboundPlayerAbilities {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.flags.encode(dst)
        }
    }
    impl Decode for ServerboundPlayerAbilities {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                flags: u8::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundPlayerAction {
        pub status: VarInt,
        pub location: i64,
        pub face: i8,
        pub sequence: VarInt,
    }
    impl PacketId for ServerboundPlayerAction {
        fn packet_id(_ver: u32) -> u8 {
            0x1C
        }
    }
    impl Encode for ServerboundPlayerAction {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.status.encode(dst)?;
            self.location.encode(dst)?;
            self.face.encode(dst)?;
            self.sequence.encode(dst)
        }
    }
    impl Decode for ServerboundPlayerAction {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                status: VarInt::decode(src)?,
                location: i64::decode(src)?,
                face: i8::decode(src)?,
                sequence: VarInt::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundSetCarriedItem {
        pub slot: i16,
    }
    impl PacketId for ServerboundSetCarriedItem {
        fn packet_id(_ver: u32) -> u8 {
            0x25
        }
    }
    impl Encode for ServerboundSetCarriedItem {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.slot.encode(dst)
        }
    }
    impl Decode for ServerboundSetCarriedItem {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                slot: i16::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundSwingArm {
        pub hand: VarInt,
    }
    impl PacketId for ServerboundSwingArm {
        fn packet_id(_ver: u32) -> u8 {
            0x2C
        }
    }
    impl Encode for ServerboundSwingArm {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.hand.encode(dst)
        }
    }
    impl Decode for ServerboundSwingArm {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                hand: VarInt::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundUseItemOn {
        pub hand: VarInt,
        pub location: i64,
        pub face: VarInt,
        pub cursor_x: f32,
        pub cursor_y: f32,
        pub cursor_z: f32,
        pub inside_block: bool,
        pub sequence: VarInt,
    }
    impl PacketId for ServerboundUseItemOn {
        fn packet_id(_ver: u32) -> u8 {
            0x2E
        }
    }
    impl Encode for ServerboundUseItemOn {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.hand.encode(dst)?;
            self.location.encode(dst)?;
            self.face.encode(dst)?;
            self.cursor_x.encode(dst)?;
            self.cursor_y.encode(dst)?;
            self.cursor_z.encode(dst)?;
            self.inside_block.encode(dst)?;
            self.sequence.encode(dst)
        }
    }
    impl Decode for ServerboundUseItemOn {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                hand: VarInt::decode(src)?,
                location: i64::decode(src)?,
                face: VarInt::decode(src)?,
                cursor_x: f32::decode(src)?,
                cursor_y: f32::decode(src)?,
                cursor_z: f32::decode(src)?,
                inside_block: bool::decode(src)?,
                sequence: VarInt::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundUseItem {
        pub hand: VarInt,
        pub sequence: VarInt,
    }
    impl PacketId for ServerboundUseItem {
        fn packet_id(_ver: u32) -> u8 {
            0x2F
        }
    }
    impl Encode for ServerboundUseItem {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.hand.encode(dst)?;
            self.sequence.encode(dst)
        }
    }
    impl Decode for ServerboundUseItem {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                hand: VarInt::decode(src)?,
                sequence: VarInt::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundSteerBoat {
        pub left_paddle_turning: bool,
        pub right_paddle_turning: bool,
    }
    impl PacketId for ServerboundSteerBoat {
        fn packet_id(_ver: u32) -> u8 {
            0x18
        }
    }
    impl Encode for ServerboundSteerBoat {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.left_paddle_turning.encode(dst)?;
            self.right_paddle_turning.encode(dst)
        }
    }
    impl Decode for ServerboundSteerBoat {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                left_paddle_turning: bool::decode(src)?,
                right_paddle_turning: bool::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundPaddleBoat {
        pub left_paddle_turning: bool,
        pub right_paddle_turning: bool,
    }
    impl PacketId for ServerboundPaddleBoat {
        fn packet_id(_ver: u32) -> u8 {
            0x18
        }
    }
    impl Encode for ServerboundPaddleBoat {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.left_paddle_turning.encode(dst)?;
            self.right_paddle_turning.encode(dst)
        }
    }
    impl Decode for ServerboundPaddleBoat {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                left_paddle_turning: bool::decode(src)?,
                right_paddle_turning: bool::decode(src)?,
            })
        }
    }

    passthrough!(ServerboundChatCommand, 0x03);
    passthrough!(ServerboundChatMessage, 0x04);
    passthrough!(ServerboundChatSessionUpdate, 0x05);
    passthrough!(ServerboundClientStatus, 0x06);
    passthrough!(ServerboundCommandSuggestion, 0x09);
    passthrough!(ServerboundClickWindow, 0x0A);
    passthrough!(ServerboundClickWindowButton, 0x09);
    passthrough!(ServerboundCloseWindow, 0x0B);
    passthrough!(ServerboundCreativeInventoryAction, 0x28);
    passthrough!(ServerboundDifficultyChange, 0x02);
    passthrough!(ServerboundDifficultyLock, 0x10);
    passthrough!(ServerboundEditBook, 0x0D);
    passthrough!(ServerboundEntityAction, 0x1D);
    passthrough!(ServerboundEntityTagQuery, 0x0E);
    passthrough!(ServerboundJigsawGenerate, 0x0F);
    passthrough!(ServerboundPickItem, 0x19);
    passthrough!(ServerboundPlaceRecipe, 0x1A);
    passthrough!(ServerboundPong, 0x20);
    passthrough!(ServerboundRecipeBookChangeSettings, 0x21);
    passthrough!(ServerboundRecipeBookSeenRecipe, 0x22);
    passthrough!(ServerboundRenameItem, 0x23);
    passthrough!(ServerboundResourcePackStatus, 0x24);
    passthrough!(ServerboundSelectTrade, 0x26);
    passthrough!(ServerboundSetBeaconEffect, 0x27);
    passthrough!(ServerboundSetStructureBlock, 0x30);
    passthrough!(ServerboundSpectate, 0x2D);
    passthrough!(ServerboundSteerVehicle, 0x1E);
    passthrough!(ServerboundUpdateCommandBlock, 0x2A);
    passthrough!(ServerboundUpdateCommandBlockMinecart, 0x2B);
    passthrough!(ServerboundUpdateSign, 0x2A);
    passthrough!(ServerboundVehicleMove, 0x17);
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! roundtrip {
        ($T:ty, $val:expr) => {{
            let v = $val;
            let mut buf = BytesMut::new();
            v.encode(&mut buf).unwrap();
            let mut b = buf.freeze();
            assert_eq!(<$T>::decode(&mut b).unwrap(), v);
        }};
    }

    #[test]
    fn keepalive_roundtrip() {
        roundtrip!(
            ClientboundKeepAlive,
            ClientboundKeepAlive { id: 987654321_i64 }
        );
        roundtrip!(
            ServerboundKeepAlive,
            ServerboundKeepAlive { id: 987654321_i64 }
        );
    }

    #[test]
    fn disconnect_roundtrip() {
        roundtrip!(
            ClientboundDisconnect,
            ClientboundDisconnect {
                reason: r#"{"text":"bye"}"#.to_string(),
            }
        );
    }

    #[test]
    fn system_chat_roundtrip() {
        roundtrip!(
            ClientboundSystemChat,
            ClientboundSystemChat {
                content: r#"{"text":"Hello"}"#.to_string(),
                overlay: false,
            }
        );
    }

    #[test]
    fn player_position_roundtrip() {
        roundtrip!(
            ClientboundPlayerPosition,
            ClientboundPlayerPosition {
                x: 0.0,
                y: 64.0,
                z: 0.0,
                yaw: 0.0,
                pitch: 0.0,
                flags: 0,
                teleport_id: VarInt(3),
            }
        );
    }

    #[test]
    fn set_time_roundtrip() {
        roundtrip!(
            ClientboundSetTime,
            ClientboundSetTime {
                world_age: 1000,
                time_of_day: 6000,
            }
        );
    }

    #[test]
    fn respawn_roundtrip() {
        roundtrip!(
            ClientboundRespawn,
            ClientboundRespawn {
                dimension_type: "minecraft:overworld".to_string(),
                dimension_name: "minecraft:overworld".to_string(),
                hashed_seed: 42,
                game_mode: 0,
                previous_game_mode: -1,
                is_debug: false,
                is_flat: false,
                data_kept: 0x01,
                death_location: None,
            }
        );
    }

    #[test]
    fn accept_teleportation_roundtrip() {
        roundtrip!(
            ServerboundAcceptTeleportation,
            ServerboundAcceptTeleportation {
                teleport_id: VarInt(7),
            }
        );
    }

    #[test]
    fn client_information_roundtrip() {
        roundtrip!(
            ServerboundClientInformation,
            ServerboundClientInformation {
                locale: "en_US".to_string(),
                view_distance: 12,
                chat_mode: VarInt(0),
                chat_colors: true,
                displayed_skin_parts: 0x7F,
                main_hand: VarInt(1),
                enable_text_filtering: false,
                allow_server_listings: true,
            }
        );
    }

    #[test]
    fn move_pos_roundtrip() {
        roundtrip!(
            ServerboundMovePlayerPos,
            ServerboundMovePlayerPos {
                x: 5.0,
                feet_y: 64.0,
                z: -2.0,
                on_ground: true,
            }
        );
    }

    #[test]
    fn move_rot_roundtrip() {
        roundtrip!(
            ServerboundMovePlayerRot,
            ServerboundMovePlayerRot {
                yaw: 90.0,
                pitch: -10.0,
                on_ground: true,
            }
        );
    }

    #[test]
    fn move_pos_rot_roundtrip() {
        roundtrip!(
            ServerboundMovePlayerPosRot,
            ServerboundMovePlayerPosRot {
                x: 1.0,
                feet_y: 65.0,
                z: 2.0,
                yaw: 180.0,
                pitch: 45.0,
                on_ground: false,
            }
        );
    }

    #[test]
    fn move_status_only_roundtrip() {
        roundtrip!(
            ServerboundMovePlayerStatusOnly,
            ServerboundMovePlayerStatusOnly { on_ground: true }
        );
    }

    #[test]
    fn plugin_message_roundtrip() {
        roundtrip!(
            ServerboundPluginMessage,
            ServerboundPluginMessage {
                channel: "minecraft:brand".to_string(),
                data: b"fabric".to_vec(),
            }
        );
        roundtrip!(
            ClientboundPluginMessage,
            ClientboundPluginMessage {
                channel: "minecraft:brand".to_string(),
                data: b"paper".to_vec(),
            }
        );
    }

    #[test]
    fn interact_roundtrip() {
        roundtrip!(
            ServerboundInteract,
            ServerboundInteract {
                entity_id: VarInt(99),
                action: InteractAction::Attack,
                sneaking: false,
            }
        );
    }

    #[test]
    fn use_item_on_roundtrip() {
        roundtrip!(
            ServerboundUseItemOn,
            ServerboundUseItemOn {
                hand: VarInt(0),
                location: 0,
                face: VarInt(1),
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.5,
                inside_block: false,
                sequence: VarInt(1),
            }
        );
    }

    #[test]
    fn player_abilities_roundtrip() {
        roundtrip!(
            ClientboundPlayerAbilities,
            ClientboundPlayerAbilities {
                flags: 0x06,
                flying_speed: 0.05,
                walking_speed: 0.1,
            }
        );
        roundtrip!(
            ServerboundPlayerAbilities,
            ServerboundPlayerAbilities { flags: 0x02 }
        );
    }

    #[test]
    fn swing_arm_roundtrip() {
        roundtrip!(ServerboundSwingArm, ServerboundSwingArm { hand: VarInt(0) });
    }

    #[test]
    fn steer_boat_roundtrip() {
        roundtrip!(
            ServerboundSteerBoat,
            ServerboundSteerBoat {
                left_paddle_turning: true,
                right_paddle_turning: false,
            }
        );
    }

    #[test]
    fn packet_ids() {
        assert_eq!(ClientboundBundle::packet_id(762), 0x00);
        assert_eq!(ClientboundSpawnEntity::packet_id(762), 0x01);
        assert_eq!(ClientboundEntityAnimation::packet_id(762), 0x04);
        assert_eq!(ClientboundDisconnect::packet_id(762), 0x1A);
        assert_eq!(ClientboundEntityEvent::packet_id(762), 0x1C);
        assert_eq!(ClientboundKeepAlive::packet_id(762), 0x24);
        assert_eq!(ClientboundLogin::packet_id(762), 0x29);
        assert_eq!(ClientboundPluginMessage::packet_id(762), 0x17);
        assert_eq!(ClientboundPlayerAbilities::packet_id(762), 0x36);
        assert_eq!(ClientboundPlayerPosition::packet_id(762), 0x3E);
        assert_eq!(ClientboundRespawn::packet_id(762), 0x44);
        assert_eq!(ClientboundSetCarriedItem::packet_id(762), 0x4A);
        assert_eq!(ClientboundSetTime::packet_id(762), 0x59);
        assert_eq!(ClientboundSystemChat::packet_id(762), 0x62);

        assert_eq!(ServerboundAcceptTeleportation::packet_id(762), 0x00);
        assert_eq!(ServerboundChatMessage::packet_id(762), 0x04);
        assert_eq!(ServerboundClientInformation::packet_id(762), 0x07);
        assert_eq!(ServerboundInteract::packet_id(762), 0x11);
        assert_eq!(ServerboundKeepAlive::packet_id(762), 0x12);
        assert_eq!(ServerboundMovePlayerStatusOnly::packet_id(762), 0x13);
        assert_eq!(ServerboundMovePlayerPos::packet_id(762), 0x14);
        assert_eq!(ServerboundMovePlayerRot::packet_id(762), 0x15);
        assert_eq!(ServerboundMovePlayerPosRot::packet_id(762), 0x16);
        assert_eq!(ServerboundPluginMessage::packet_id(762), 0x0C);
        assert_eq!(ServerboundPlayerAbilities::packet_id(762), 0x1B);
        assert_eq!(ServerboundPlayerAction::packet_id(762), 0x1C);
        assert_eq!(ServerboundSetCarriedItem::packet_id(762), 0x25);
        assert_eq!(ServerboundSwingArm::packet_id(762), 0x2C);
        assert_eq!(ServerboundUseItemOn::packet_id(762), 0x2E);
        assert_eq!(ServerboundUseItem::packet_id(762), 0x2F);
        assert_eq!(ServerboundSteerBoat::packet_id(762), 0x18);
        assert_eq!(ServerboundPaddleBoat::packet_id(762), 0x18);
    }
}
