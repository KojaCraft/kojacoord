use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::codec::{Decode, Encode, PacketId};
use crate::error::ProtocolError;
use crate::types::VarInt;

fn need(src: &Bytes, n: usize) -> Result<(), ProtocolError> {
    if src.remaining() < n {
        Err(ProtocolError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            format!("need {} bytes, only {} remaining", n, src.remaining()),
        )))
    } else {
        Ok(())
    }
}

fn encode_string(s: &str, dst: &mut BytesMut) -> Result<(), ProtocolError> {
    VarInt(s.len() as i32).encode(dst)?;
    dst.extend_from_slice(s.as_bytes());
    Ok(())
}

fn decode_string(src: &mut Bytes) -> Result<String, ProtocolError> {
    let len = VarInt::decode(src)?.0 as usize;
    need(src, len)?;
    let bytes = src.copy_to_bytes(len);
    String::from_utf8(bytes.to_vec()).map_err(|_| {
        ProtocolError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid utf-8 string",
        ))
    })
}

pub use packets::{
    ClientboundAcknowledgeBlockChange, ClientboundAwardStats, ClientboundBlockAction,
    ClientboundBlockDestroyStage, ClientboundBlockEntityData, ClientboundBlockUpdate,
    ClientboundBossBar, ClientboundBundle, ClientboundChangeDifficulty,
    ClientboundChunkBatchFinished, ClientboundChunkBatchStart, ClientboundChunkBiomes,
    ClientboundClearTitles, ClientboundCommandSuggestions, ClientboundCommands,
    ClientboundContainerClose, ClientboundContainerSetContent, ClientboundContainerSetProperty,
    ClientboundContainerSetSlot, ClientboundCookieRequest, ClientboundCooldown,
    ClientboundCustomChatCompletions, ClientboundDamageEvent, ClientboundDebugSample,
    ClientboundDeleteChat, ClientboundDisconnect, ClientboundDisguisedChat,
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
    ClientboundRemoveEntityEffect, ClientboundResetScore, ClientboundResourcePackPop,
    ClientboundResourcePackPush, ClientboundRespawn, ClientboundRotateHead,
    ClientboundSectionBlocksUpdate, ClientboundSelectAdvancementsTab, ClientboundServerData,
    ClientboundSetActionBarText, ClientboundSetBorderCenter, ClientboundSetBorderLerpSize,
    ClientboundSetBorderSize, ClientboundSetBorderWarningDelay,
    ClientboundSetBorderWarningDistance, ClientboundSetCamera, ClientboundSetCarriedItem,
    ClientboundSetCursorItem, ClientboundSetEntityLink, ClientboundSetEntityMotion,
    ClientboundSetEquipment, ClientboundSetExperience, ClientboundSetHealth,
    ClientboundSetPlayerInventory, ClientboundSetScoreboardObjective,
    ClientboundSetScoreboardScore, ClientboundSetSimulationDistance, ClientboundSetSubtitleText,
    ClientboundSetTime, ClientboundSetTitleAnimationTimes, ClientboundSetTitleText,
    ClientboundSound, ClientboundSoundEntity, ClientboundSpawnEntity,
    ClientboundSpawnExperienceOrb, ClientboundSpawnPlayer, ClientboundStartConfiguration,
    ClientboundStopSound, ClientboundStoreCookie, ClientboundSystemChat, ClientboundTabList,
    ClientboundTagQuery, ClientboundTakeItemEntity, ClientboundTeleportEntity,
    ClientboundTickingState, ClientboundTickingStep, ClientboundTransfer,
    ClientboundUpdateAdvancements, ClientboundUpdateAttributes, ClientboundUpdateEffects,
    ClientboundUpdateRecipes, ClientboundUpdateTags, InteractAction,
    ServerboundAcceptTeleportation, ServerboundButtonPressed, ServerboundChatCommand,
    ServerboundChatMessage, ServerboundChatSessionUpdate, ServerboundClickWindow,
    ServerboundClickWindowButton, ServerboundClientInformation, ServerboundClientStatus,
    ServerboundCloseWindow, ServerboundCommandSuggestion, ServerboundConfigurationAcknowledged,
    ServerboundCreativeInventoryAction, ServerboundDifficultyChange, ServerboundDifficultyLock,
    ServerboundEditBook, ServerboundEntityAction, ServerboundEntityTagQuery,
    ServerboundHeldItemChange, ServerboundInteract, ServerboundJigsawGenerate,
    ServerboundKeepAlive, ServerboundMovePlayerPos, ServerboundMovePlayerPosRot,
    ServerboundMovePlayerRot, ServerboundMovePlayerStatusOnly, ServerboundPaddleBoat,
    ServerboundPickItem, ServerboundPlaceRecipe, ServerboundPlayerAbilities,
    ServerboundPlayerAction, ServerboundPlayerWeaponAttack, ServerboundPluginMessage,
    ServerboundPong, ServerboundRecipeBookChangeSettings, ServerboundRecipeBookSeenRecipe,
    ServerboundRenameItem, ServerboundResourcePackStatus, ServerboundSelectTrade,
    ServerboundSetBeaconEffect, ServerboundSetCreativeModeSlot, ServerboundSetMerchantTrade,
    ServerboundSetStructureBlock, ServerboundSpectate, ServerboundSteerBoat,
    ServerboundSteerVehicle, ServerboundSwingArm, ServerboundUpdateCommandBlock,
    ServerboundUpdateCommandBlockMinecart, ServerboundUpdateSign, ServerboundUseItem,
    ServerboundUseItemOn, ServerboundVehicleMove,
};

mod packets {
    use super::*;

    macro_rules! passthrough {
        ($(#[$meta:meta])* $name:ident, $id:expr) => {
            $(#[$meta])*
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

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundKeepAlive {
        pub keep_alive_id: i64,
    }

    impl PacketId for ClientboundKeepAlive {
        fn packet_id(_ver: u32) -> u8 {
            0x29
        }
    }

    impl Encode for ClientboundKeepAlive {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.keep_alive_id.encode(dst)
        }
    }

    impl Decode for ClientboundKeepAlive {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                keep_alive_id: i64::decode(src)?,
            })
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
            0x43
        }
    }

    impl Encode for ClientboundPlayerPosition {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_f64(self.x);
            dst.put_f64(self.y);
            dst.put_f64(self.z);
            dst.put_f32(self.yaw);
            dst.put_f32(self.pitch);
            dst.put_u8(self.flags);
            self.teleport_id.encode(dst)
        }
    }

    impl Decode for ClientboundPlayerPosition {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8 + 8 + 8 + 4 + 4 + 1)?;
            Ok(Self {
                x: src.get_f64(),
                y: src.get_f64(),
                z: src.get_f64(),
                yaw: src.get_f32(),
                pitch: src.get_f32(),
                flags: src.get_u8(),
                teleport_id: VarInt::decode(src)?,
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
            0x1A
        }
    }

    impl Encode for ClientboundPluginMessage {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_string(&self.channel, dst)?;
            dst.extend_from_slice(&self.data);
            Ok(())
        }
    }

    impl Decode for ClientboundPluginMessage {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let channel = decode_string(src)?;
            let data = src.copy_to_bytes(src.remaining()).to_vec();
            Ok(Self { channel, data })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSystemChat {
        pub json_message: String,
        pub overlay: bool,
    }

    impl PacketId for ClientboundSystemChat {
        fn packet_id(_ver: u32) -> u8 {
            0x6B
        }
    }

    impl Encode for ClientboundSystemChat {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_string(&self.json_message, dst)?;
            dst.put_u8(self.overlay as u8);
            Ok(())
        }
    }

    impl Decode for ClientboundSystemChat {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let json_message = decode_string(src)?;
            need(src, 1)?;
            let overlay = src.get_u8() != 0;
            Ok(Self {
                json_message,
                overlay,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundLogin {
        pub entity_id: i32,
        pub is_hardcore: bool,
        pub dimension_names: Vec<String>,
        pub max_players: VarInt,
        pub view_distance: VarInt,
        pub simulation_distance: VarInt,
        pub reduced_debug_info: bool,
        pub enable_respawn_screen: bool,
        pub do_limited_crafting: bool,
        pub dimension_type: VarInt,
        pub dimension_name: String,
        pub hashed_seed: i64,
        pub game_mode: u8,
        pub previous_game_mode: i8,
        pub is_debug: bool,
        pub is_flat: bool,
        pub death_location: Option<(String, i64)>,
        pub portal_cooldown: VarInt,
        pub sea_level: VarInt,
    }

    impl PacketId for ClientboundLogin {
        fn packet_id(_ver: u32) -> u8 {
            0x2E
        }
    }

    impl Encode for ClientboundLogin {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.entity_id);
            dst.put_u8(self.is_hardcore as u8);
            self.dimension_names.encode(dst)?;
            self.max_players.encode(dst)?;
            self.view_distance.encode(dst)?;
            self.simulation_distance.encode(dst)?;
            dst.put_u8(self.reduced_debug_info as u8);
            dst.put_u8(self.enable_respawn_screen as u8);
            dst.put_u8(self.do_limited_crafting as u8);
            self.dimension_type.encode(dst)?;
            encode_string(&self.dimension_name, dst)?;
            dst.put_i64(self.hashed_seed);
            dst.put_u8(self.game_mode);
            dst.put_i8(self.previous_game_mode);
            dst.put_u8(self.is_debug as u8);
            dst.put_u8(self.is_flat as u8);
            match &self.death_location {
                Some((dim, pos)) => {
                    dst.put_u8(1);
                    encode_string(dim, dst)?;
                    dst.put_i64(*pos);
                },
                None => dst.put_u8(0),
            }
            self.portal_cooldown.encode(dst)?;
            self.sea_level.encode(dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundLogin {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4)?;
            let entity_id = src.get_i32();
            need(src, 1)?;
            let is_hardcore = src.get_u8() != 0;
            let dimension_names = Vec::<String>::decode(src)?;
            let max_players = VarInt::decode(src)?;
            let view_distance = VarInt::decode(src)?;
            let simulation_distance = VarInt::decode(src)?;
            need(src, 3)?;
            let reduced_debug_info = src.get_u8() != 0;
            let enable_respawn_screen = src.get_u8() != 0;
            let do_limited_crafting = src.get_u8() != 0;
            let dimension_type = VarInt::decode(src)?;
            let dimension_name = decode_string(src)?;
            need(src, 8)?;
            let hashed_seed = src.get_i64();
            need(src, 4)?;
            let game_mode = src.get_u8();
            let previous_game_mode = src.get_i8();
            let is_debug = src.get_u8() != 0;
            let is_flat = src.get_u8() != 0;
            need(src, 1)?;
            let death_location = if src.get_u8() != 0 {
                let dim = decode_string(src)?;
                need(src, 8)?;
                let pos = src.get_i64();
                Some((dim, pos))
            } else {
                None
            };
            let portal_cooldown = VarInt::decode(src)?;
            let sea_level = VarInt::decode(src)?;
            Ok(Self {
                entity_id,
                is_hardcore,
                dimension_names,
                max_players,
                view_distance,
                simulation_distance,
                reduced_debug_info,
                enable_respawn_screen,
                do_limited_crafting,
                dimension_type,
                dimension_name,
                hashed_seed,
                game_mode,
                previous_game_mode,
                is_debug,
                is_flat,
                death_location,
                portal_cooldown,
                sea_level,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundRespawn {
        pub dimension_type: VarInt,
        pub dimension_name: String,
        pub hashed_seed: i64,
        pub game_mode: u8,
        pub previous_game_mode: i8,
        pub is_debug: bool,
        pub is_flat: bool,
        pub data_kept: u8,
        pub death_location: Option<(String, i64)>,
        pub portal_cooldown: VarInt,
        pub sea_level: VarInt,
    }

    impl PacketId for ClientboundRespawn {
        fn packet_id(_ver: u32) -> u8 {
            0x4B
        }
    }

    impl Encode for ClientboundRespawn {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.dimension_type.encode(dst)?;
            encode_string(&self.dimension_name, dst)?;
            dst.put_i64(self.hashed_seed);
            dst.put_u8(self.game_mode);
            dst.put_i8(self.previous_game_mode);
            dst.put_u8(self.is_debug as u8);
            dst.put_u8(self.is_flat as u8);
            dst.put_u8(self.data_kept);
            match &self.death_location {
                Some((dim, pos)) => {
                    dst.put_u8(1);
                    encode_string(dim, dst)?;
                    dst.put_i64(*pos);
                },
                None => dst.put_u8(0),
            }
            self.portal_cooldown.encode(dst)?;
            self.sea_level.encode(dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundRespawn {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let dimension_type = VarInt::decode(src)?;
            let dimension_name = decode_string(src)?;
            need(src, 8)?;
            let hashed_seed = src.get_i64();
            need(src, 4)?;
            let game_mode = src.get_u8();
            let previous_game_mode = src.get_i8();
            let is_debug = src.get_u8() != 0;
            let is_flat = src.get_u8() != 0;
            need(src, 1)?;
            let data_kept = src.get_u8();
            need(src, 1)?;
            let death_location = if src.get_u8() != 0 {
                let dim = decode_string(src)?;
                need(src, 8)?;
                let pos = src.get_i64();
                Some((dim, pos))
            } else {
                None
            };
            let portal_cooldown = VarInt::decode(src)?;
            let sea_level = VarInt::decode(src)?;
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
                portal_cooldown,
                sea_level,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSound {
        pub sound_type: VarInt,
        pub sound_name: String,
        pub sound_category: VarInt,
        pub effect_pos_x: i32,
        pub effect_pos_y: i32,
        pub effect_pos_z: i32,
        pub volume: f32,
        pub pitch: f32,
        pub seed: i64,
    }

    impl PacketId for ClientboundSound {
        fn packet_id(_ver: u32) -> u8 {
            0x67
        }
    }

    impl Encode for ClientboundSound {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.sound_type.encode(dst)?;
            encode_string(&self.sound_name, dst)?;
            self.sound_category.encode(dst)?;
            dst.put_i32(self.effect_pos_x);
            dst.put_i32(self.effect_pos_y);
            dst.put_i32(self.effect_pos_z);
            dst.put_f32(self.volume);
            dst.put_f32(self.pitch);
            dst.put_i64(self.seed);
            Ok(())
        }
    }

    impl Decode for ClientboundSound {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let sound_type = VarInt::decode(src)?;
            let sound_name = decode_string(src)?;
            let sound_category = VarInt::decode(src)?;
            need(src, 4 + 4 + 4 + 4 + 4 + 8)?;
            let effect_pos_x = src.get_i32();
            let effect_pos_y = src.get_i32();
            let effect_pos_z = src.get_i32();
            let volume = src.get_f32();
            let pitch = src.get_f32();
            let seed = src.get_i64();
            Ok(Self {
                sound_type,
                sound_name,
                sound_category,
                effect_pos_x,
                effect_pos_y,
                effect_pos_z,
                volume,
                pitch,
                seed,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundDisconnect {
        pub reason: String,
    }

    impl PacketId for ClientboundDisconnect {
        fn packet_id(_ver: u32) -> u8 {
            0x1E
        }
    }

    impl Encode for ClientboundDisconnect {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_string(&self.reason, dst)
        }
    }

    impl Decode for ClientboundDisconnect {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                reason: decode_string(src)?,
            })
        }
    }

    passthrough!(ClientboundBundle, 0x00);
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
    passthrough!(ClientboundChunkBatchFinished, 0x0D);
    passthrough!(ClientboundChunkBatchStart, 0x0E);
    passthrough!(ClientboundChunkBiomes, 0x0F);
    passthrough!(ClientboundClearTitles, 0x10);
    passthrough!(ClientboundCommandSuggestions, 0x11);
    passthrough!(ClientboundCommands, 0x12);
    passthrough!(ClientboundContainerClose, 0x13);
    passthrough!(ClientboundContainerSetProperty, 0x15);

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundContainerSetContent {
        pub window_id: u8,
        pub slots: Vec<crate::types::Slot>,
        pub carried_item: crate::types::Slot,
    }

    impl PacketId for ClientboundContainerSetContent {
        fn packet_id(_ver: u32) -> u8 {
            0x14
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
            0x16
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

    passthrough!(ClientboundCookieRequest, 0x17);
    passthrough!(ClientboundCooldown, 0x18);
    passthrough!(ClientboundCustomChatCompletions, 0x19);
    passthrough!(ClientboundDamageEvent, 0x1B);
    passthrough!(ClientboundDebugSample, 0x1C);
    passthrough!(ClientboundDeleteChat, 0x1D);
    passthrough!(ClientboundDisguisedChat, 0x1F);
    passthrough!(ClientboundEntityEvent, 0x20);
    passthrough!(ClientboundExplosion, 0x21);
    passthrough!(ClientboundForgetLevelChunk, 0x22);
    passthrough!(ClientboundGameEvent, 0x23);
    passthrough!(ClientboundHorseScreenOpen, 0x24);
    passthrough!(ClientboundHurtAnimation, 0x25);
    passthrough!(ClientboundInitializeBorder, 0x26);
    passthrough!(ClientboundLevelChunkWithLight, 0x2A);
    passthrough!(ClientboundLevelEvent, 0x2B);
    passthrough!(ClientboundLevelParticles, 0x2C);
    passthrough!(ClientboundLightUpdate, 0x2D);
    passthrough!(ClientboundMapItemData, 0x2F);
    passthrough!(ClientboundMerchantOffers, 0x30);
    passthrough!(ClientboundMoveEntityPos, 0x31);
    passthrough!(ClientboundMoveEntityPosRot, 0x32);
    passthrough!(ClientboundMoveEntityRot, 0x33);
    passthrough!(ClientboundMoveVehicle, 0x34);
    passthrough!(ClientboundOpenBook, 0x35);
    passthrough!(ClientboundOpenScreen, 0x36);
    passthrough!(ClientboundOpenSignEditor, 0x37);
    passthrough!(ClientboundPing, 0x38);
    passthrough!(ClientboundPlaceGhostRecipe, 0x3A);
    passthrough!(ClientboundPlayerAbilities, 0x3B);
    passthrough!(ClientboundPlayerChat, 0x3C);
    passthrough!(ClientboundPlayerCombatEnd, 0x3D);
    passthrough!(ClientboundPlayerCombatEnter, 0x3E);
    passthrough!(ClientboundPlayerCombatKill, 0x3F);
    passthrough!(ClientboundPlayerInfoRemove, 0x40);
    passthrough!(ClientboundPlayerInfoUpdate, 0x41);
    passthrough!(ClientboundPlayerLookAt, 0x42);
    passthrough!(ClientboundRecipeBookSettings, 0x44);
    passthrough!(ClientboundRecipes, 0x45);
    passthrough!(ClientboundRemoveEntities, 0x46);
    passthrough!(ClientboundRemoveEntityEffect, 0x47);
    passthrough!(ClientboundResetScore, 0x48);
    passthrough!(ClientboundResourcePackPop, 0x49);
    passthrough!(ClientboundResourcePackPush, 0x4A);
    passthrough!(ClientboundRotateHead, 0x4C);
    passthrough!(ClientboundSectionBlocksUpdate, 0x4D);
    passthrough!(ClientboundSelectAdvancementsTab, 0x4E);
    passthrough!(ClientboundServerData, 0x4F);
    passthrough!(ClientboundSetActionBarText, 0x50);
    passthrough!(ClientboundSetBorderCenter, 0x51);
    passthrough!(ClientboundSetBorderLerpSize, 0x52);
    passthrough!(ClientboundSetBorderSize, 0x53);
    passthrough!(ClientboundSetBorderWarningDelay, 0x54);
    passthrough!(ClientboundSetBorderWarningDistance, 0x55);
    passthrough!(ClientboundSetCamera, 0x56);
    passthrough!(ClientboundSetCursorItem, 0x57);
    passthrough!(ClientboundSetEntityLink, 0x58);
    passthrough!(ClientboundSetEntityMotion, 0x59);
    passthrough!(ClientboundSetEquipment, 0x5A);
    passthrough!(ClientboundSetExperience, 0x5B);
    passthrough!(ClientboundSetHealth, 0x5C);
    passthrough!(ClientboundSetCarriedItem, 0x5D);
    passthrough!(ClientboundSetPlayerInventory, 0x5E);
    passthrough!(ClientboundSetScoreboardObjective, 0x5F);
    passthrough!(ClientboundSetScoreboardScore, 0x60);
    passthrough!(ClientboundSetSimulationDistance, 0x61);
    passthrough!(ClientboundSetSubtitleText, 0x62);
    passthrough!(ClientboundSetTime, 0x63);
    passthrough!(ClientboundSetTitleText, 0x64);
    passthrough!(ClientboundSetTitleAnimationTimes, 0x65);
    passthrough!(ClientboundSoundEntity, 0x66);
    passthrough!(ClientboundStartConfiguration, 0x68);
    passthrough!(ClientboundStopSound, 0x69);
    passthrough!(ClientboundStoreCookie, 0x6A);
    passthrough!(ClientboundTabList, 0x6C);
    passthrough!(ClientboundTagQuery, 0x6D);
    passthrough!(ClientboundTakeItemEntity, 0x6E);
    passthrough!(ClientboundTeleportEntity, 0x6F);
    passthrough!(ClientboundTickingState, 0x70);
    passthrough!(ClientboundTickingStep, 0x71);
    passthrough!(ClientboundTransfer, 0x72);
    passthrough!(ClientboundUpdateAdvancements, 0x73);
    passthrough!(ClientboundUpdateAttributes, 0x74);
    passthrough!(ClientboundUpdateEffects, 0x75);
    passthrough!(ClientboundUpdateRecipes, 0x76);
    passthrough!(ClientboundUpdateTags, 0x77);

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundKeepAlive {
        pub keep_alive_id: i64,
    }

    impl PacketId for ServerboundKeepAlive {
        fn packet_id(_ver: u32) -> u8 {
            0x15
        }
    }

    impl Encode for ServerboundKeepAlive {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.keep_alive_id.encode(dst)
        }
    }

    impl Decode for ServerboundKeepAlive {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                keep_alive_id: i64::decode(src)?,
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
            0x17
        }
    }

    impl Encode for ServerboundMovePlayerPos {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_f64(self.x);
            dst.put_f64(self.feet_y);
            dst.put_f64(self.z);
            dst.put_u8(self.on_ground as u8);
            Ok(())
        }
    }

    impl Decode for ServerboundMovePlayerPos {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8 + 8 + 8 + 1)?;
            Ok(Self {
                x: src.get_f64(),
                feet_y: src.get_f64(),
                z: src.get_f64(),
                on_ground: src.get_u8() != 0,
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
            0x19
        }
    }

    impl Encode for ServerboundMovePlayerPosRot {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_f64(self.x);
            dst.put_f64(self.feet_y);
            dst.put_f64(self.z);
            dst.put_f32(self.yaw);
            dst.put_f32(self.pitch);
            dst.put_u8(self.on_ground as u8);
            Ok(())
        }
    }

    impl Decode for ServerboundMovePlayerPosRot {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8 + 8 + 8 + 4 + 4 + 1)?;
            Ok(Self {
                x: src.get_f64(),
                feet_y: src.get_f64(),
                z: src.get_f64(),
                yaw: src.get_f32(),
                pitch: src.get_f32(),
                on_ground: src.get_u8() != 0,
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
            0x18
        }
    }

    impl Encode for ServerboundMovePlayerRot {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_f32(self.yaw);
            dst.put_f32(self.pitch);
            dst.put_u8(self.on_ground as u8);
            Ok(())
        }
    }

    impl Decode for ServerboundMovePlayerRot {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 4 + 1)?;
            Ok(Self {
                yaw: src.get_f32(),
                pitch: src.get_f32(),
                on_ground: src.get_u8() != 0,
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
            0x14
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
                    dst.put_f32(*target_x);
                    dst.put_f32(*target_y);
                    dst.put_f32(*target_z);
                    hand.encode(dst)?;
                },
            }
            dst.put_u8(self.sneaking as u8);
            Ok(())
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
                2 => {
                    need(src, 4 + 4 + 4)?;
                    let target_x = src.get_f32();
                    let target_y = src.get_f32();
                    let target_z = src.get_f32();
                    let hand = VarInt::decode(src)?;
                    InteractAction::InteractAt {
                        target_x,
                        target_y,
                        target_z,
                        hand,
                    }
                },
                _ => return Err(ProtocolError::UnexpectedEof),
            };
            need(src, 1)?;
            let sneaking = src.get_u8() != 0;
            Ok(Self {
                entity_id,
                action,
                sneaking,
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
            0x0F
        }
    }

    impl Encode for ServerboundPluginMessage {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_string(&self.channel, dst)?;
            dst.extend_from_slice(&self.data);
            Ok(())
        }
    }

    impl Decode for ServerboundPluginMessage {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let channel = decode_string(src)?;
            let data = src.copy_to_bytes(src.remaining()).to_vec();
            Ok(Self { channel, data })
        }
    }

    passthrough!(ServerboundAcceptTeleportation, 0x00);
    passthrough!(ServerboundMovePlayerStatusOnly, 0x16);
    passthrough!(ServerboundVehicleMove, 0x1A);
    passthrough!(ServerboundSteerBoat, 0x1B);
    passthrough!(ServerboundPaddleBoat, 0x1B);
    passthrough!(ServerboundPlayerAbilities, 0x1E);
    passthrough!(ServerboundPlayerAction, 0x1F);
    passthrough!(ServerboundEntityAction, 0x20);
    passthrough!(ServerboundSteerVehicle, 0x21);
    passthrough!(ServerboundPlayerWeaponAttack, 0x22);
    passthrough!(ServerboundSwingArm, 0x32);
    passthrough!(ServerboundSpectate, 0x33);
    passthrough!(ServerboundUseItemOn, 0x34);
    passthrough!(ServerboundUseItem, 0x35);
    passthrough!(ServerboundCloseWindow, 0x0E);
    passthrough!(ServerboundClickWindow, 0x0D);
    passthrough!(ServerboundClickWindowButton, 0x0C);
    passthrough!(ServerboundCreativeInventoryAction, 0x2E);
    passthrough!(ServerboundHeldItemChange, 0x2B);
    passthrough!(ServerboundSetCreativeModeSlot, 0x2C);
    passthrough!(ServerboundChatMessage, 0x06);
    passthrough!(ServerboundChatCommand, 0x05);
    passthrough!(ServerboundChatSessionUpdate, 0x07);
    passthrough!(ServerboundClientStatus, 0x08);
    passthrough!(ServerboundClientInformation, 0x09);
    passthrough!(ServerboundResourcePackStatus, 0x2A);
    passthrough!(ServerboundPong, 0x24);
    passthrough!(ServerboundConfigurationAcknowledged, 0x0B);
    passthrough!(ServerboundButtonPressed, 0x0C);
    passthrough!(ServerboundCommandSuggestion, 0x0A);
    passthrough!(ServerboundDifficultyChange, 0x02);
    passthrough!(ServerboundDifficultyLock, 0x12);
    passthrough!(ServerboundEditBook, 0x10);
    passthrough!(ServerboundEntityTagQuery, 0x11);
    passthrough!(ServerboundJigsawGenerate, 0x13);
    passthrough!(ServerboundPickItem, 0x1C);
    passthrough!(ServerboundPlaceRecipe, 0x1D);
    passthrough!(ServerboundRecipeBookChangeSettings, 0x26);
    passthrough!(ServerboundRecipeBookSeenRecipe, 0x27);
    passthrough!(ServerboundRenameItem, 0x29);
    passthrough!(ServerboundSelectTrade, 0x2D);
    passthrough!(ServerboundSetMerchantTrade, 0x2D);
    passthrough!(ServerboundSetBeaconEffect, 0x2F);
    passthrough!(ServerboundSetStructureBlock, 0x36);
    passthrough!(ServerboundUpdateCommandBlock, 0x30);
    passthrough!(ServerboundUpdateCommandBlockMinecart, 0x31);
    passthrough!(ServerboundUpdateSign, 0x37);
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
            ClientboundKeepAlive {
                keep_alive_id: 123456789_i64
            }
        );
        roundtrip!(
            ServerboundKeepAlive,
            ServerboundKeepAlive {
                keep_alive_id: 987654321_i64
            }
        );
    }

    #[test]
    fn player_position_roundtrip() {
        roundtrip!(
            ClientboundPlayerPosition,
            ClientboundPlayerPosition {
                x: -100.0,
                y: 70.0,
                z: 200.0,
                yaw: 90.0,
                pitch: -45.0,
                flags: 0,
                teleport_id: VarInt(5),
            }
        );
    }

    #[test]
    fn plugin_message_roundtrip() {
        roundtrip!(
            ClientboundPluginMessage,
            ClientboundPluginMessage {
                channel: "minecraft:brand".to_string(),
                data: b"paper".to_vec(),
            }
        );
        roundtrip!(
            ServerboundPluginMessage,
            ServerboundPluginMessage {
                channel: "minecraft:brand".to_string(),
                data: b"vanilla".to_vec(),
            }
        );
    }

    #[test]
    fn system_chat_roundtrip() {
        roundtrip!(
            ClientboundSystemChat,
            ClientboundSystemChat {
                json_message: r#"{"text":"hello"}"#.to_string(),
                overlay: false,
            }
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
    fn respawn_roundtrip() {
        roundtrip!(
            ClientboundRespawn,
            ClientboundRespawn {
                dimension_type: VarInt(0),
                dimension_name: "minecraft:overworld".to_string(),
                hashed_seed: 0,
                game_mode: 0,
                previous_game_mode: -1,
                is_debug: false,
                is_flat: true,
                data_kept: 0,
                death_location: None,
                portal_cooldown: VarInt(0),
                sea_level: VarInt(63),
            }
        );
    }

    #[test]
    fn sound_roundtrip() {
        roundtrip!(
            ClientboundSound,
            ClientboundSound {
                sound_type: VarInt(0),
                sound_name: "block.note_block.harp".to_string(),
                sound_category: VarInt(2),
                effect_pos_x: 0,
                effect_pos_y: 2048,
                effect_pos_z: 0,
                volume: 1.0,
                pitch: 1.0,
                seed: 0,
            }
        );
    }

    #[test]
    fn login_roundtrip() {
        roundtrip!(
            ClientboundLogin,
            ClientboundLogin {
                entity_id: 0,
                is_hardcore: false,
                dimension_names: vec!["minecraft:overworld".to_string()],
                max_players: VarInt(20),
                view_distance: VarInt(8),
                simulation_distance: VarInt(8),
                reduced_debug_info: false,
                enable_respawn_screen: true,
                do_limited_crafting: false,
                dimension_type: VarInt(0),
                dimension_name: "minecraft:overworld".to_string(),
                hashed_seed: 0,
                game_mode: 3,
                previous_game_mode: -1,
                is_debug: false,
                is_flat: true,
                death_location: None,
                portal_cooldown: VarInt(0),
                sea_level: VarInt(63),
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
    fn interact_roundtrip() {
        roundtrip!(
            ServerboundInteract,
            ServerboundInteract {
                entity_id: VarInt(1),
                action: InteractAction::Attack,
                sneaking: false,
            }
        );
        roundtrip!(
            ServerboundInteract,
            ServerboundInteract {
                entity_id: VarInt(2),
                action: InteractAction::Interact { hand: VarInt(0) },
                sneaking: true,
            }
        );
        roundtrip!(
            ServerboundInteract,
            ServerboundInteract {
                entity_id: VarInt(3),
                action: InteractAction::InteractAt {
                    target_x: 0.5,
                    target_y: 1.0,
                    target_z: 0.5,
                    hand: VarInt(0),
                },
                sneaking: false,
            }
        );
    }

    #[test]
    fn packet_ids() {
        assert_eq!(ClientboundKeepAlive::packet_id(767), 0x29);
        assert_eq!(ClientboundPlayerPosition::packet_id(767), 0x43);
        assert_eq!(ClientboundPluginMessage::packet_id(767), 0x1A);
        assert_eq!(ClientboundSystemChat::packet_id(767), 0x6B);
        assert_eq!(ClientboundLogin::packet_id(767), 0x2E);
        assert_eq!(ClientboundRespawn::packet_id(767), 0x4B);
        assert_eq!(ClientboundSound::packet_id(767), 0x67);
        assert_eq!(ClientboundDisconnect::packet_id(767), 0x1E);
        assert_eq!(ServerboundKeepAlive::packet_id(767), 0x15);
        assert_eq!(ServerboundMovePlayerPos::packet_id(767), 0x17);
        assert_eq!(ServerboundMovePlayerPosRot::packet_id(767), 0x19);
        assert_eq!(ServerboundMovePlayerRot::packet_id(767), 0x18);
        assert_eq!(ServerboundInteract::packet_id(767), 0x14);
        assert_eq!(ServerboundPluginMessage::packet_id(767), 0x0F);
        assert_eq!(ClientboundBundle::packet_id(767), 0x00);
        assert_eq!(ClientboundUpdateTags::packet_id(767), 0x77);
        assert_eq!(ServerboundAcceptTeleportation::packet_id(767), 0x00);
        assert_eq!(ServerboundUpdateSign::packet_id(767), 0x37);
    }
}
