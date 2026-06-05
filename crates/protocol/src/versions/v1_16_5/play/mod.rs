use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::codec::{Decode, Encode, PacketId};
use crate::error::ProtocolError;
use crate::types::VarInt;

pub use packets::{
    ClientboundAcknowledgePlayerDigging, ClientboundAdvancements, ClientboundAttachEntity,
    ClientboundAwardStats, ClientboundBlockAction, ClientboundBlockBreakAnimation,
    ClientboundBlockChange, ClientboundBlockEntityData, ClientboundBossBar, ClientboundCamera,
    ClientboundChangeGameState, ClientboundChatMessage, ClientboundChunkData,
    ClientboundCloseWindow, ClientboundCollectItem, ClientboundCombatEvent,
    ClientboundCraftRecipeResponse, ClientboundDeclareCommands, ClientboundDeclareRecipes,
    ClientboundDestroyEntities, ClientboundDisconnect, ClientboundDisplayScoreboard,
    ClientboundEffect, ClientboundEntityAnimation, ClientboundEntityEffect,
    ClientboundEntityEquipment, ClientboundEntityHeadLook, ClientboundEntityMetadata,
    ClientboundEntityPosition, ClientboundEntityPositionAndRotation, ClientboundEntityProperties,
    ClientboundEntityRotation, ClientboundEntitySoundEffect, ClientboundEntityStatus,
    ClientboundEntityTeleport, ClientboundEntityVelocity, ClientboundExplosion,
    ClientboundFacePlayer, ClientboundHeldItemChange, ClientboundInitializeWorldBorder,
    ClientboundJoinGame, ClientboundKeepAlive, ClientboundMap, ClientboundMultiBlockChange,
    ClientboundNamedSoundEffect, ClientboundNbtQueryResponse, ClientboundOpenBook,
    ClientboundOpenHorseWindow, ClientboundOpenSignEditor, ClientboundOpenWindow,
    ClientboundParticle, ClientboundPlayerAbilities, ClientboundPlayerInfo,
    ClientboundPlayerListHeaderAndFooter, ClientboundPlayerPosition, ClientboundPluginMessage,
    ClientboundRemoveEntityEffect, ClientboundResourcePackSend, ClientboundRespawn,
    ClientboundScoreboardObjective, ClientboundSelectAdvancementsTab, ClientboundServerDifficulty,
    ClientboundSetCooldown, ClientboundSetExperience, ClientboundSetPassengers, ClientboundSetSlot,
    ClientboundSoundEffect, ClientboundSpawnEntity, ClientboundSpawnExperienceOrb,
    ClientboundSpawnLivingEntity, ClientboundSpawnPainting, ClientboundSpawnPlayer,
    ClientboundSpawnPosition, ClientboundStopSound, ClientboundTabComplete, ClientboundTags,
    ClientboundTeams, ClientboundTimeUpdate, ClientboundTitle, ClientboundTradeList,
    ClientboundUnloadChunk, ClientboundUpdateHealth, ClientboundUpdateLight,
    ClientboundUpdateScore, ClientboundUpdateViewDistance, ClientboundUpdateViewPosition,
    ClientboundVehicleMove, ClientboundWindowConfirmation, ClientboundWindowItems,
    ClientboundWindowProperty, ClientboundWorldBorder, InteractAction, ServerboundChatMessage,
    ServerboundClickWindow, ServerboundClickWindowButton, ServerboundCloseWindow,
    ServerboundCraftRecipeRequest, ServerboundCreativeModeSlot, ServerboundEditBook,
    ServerboundEntityAction, ServerboundGenerateStructure, ServerboundHeldItemChange,
    ServerboundInteract, ServerboundKeepAlive, ServerboundLockDifficulty, ServerboundMovePlayerPos,
    ServerboundMovePlayerPosRot, ServerboundMovePlayerPosRotSb, ServerboundMovePlayerRot,
    ServerboundMovePlayerStatus, ServerboundPickItem, ServerboundPlaceRecipe,
    ServerboundPlayerAbilities, ServerboundPlayerBlockPlacement, ServerboundPlayerDigging,
    ServerboundPluginMessage, ServerboundQueryBlockNbt, ServerboundQueryEntityNbt,
    ServerboundResourcePackStatus, ServerboundSelectTrade, ServerboundSetBeaconEffect,
    ServerboundSetDifficulty, ServerboundSpectate, ServerboundSteerBoat, ServerboundSteerVehicle,
    ServerboundSwingArm, ServerboundTabComplete, ServerboundTeleportConfirm,
    ServerboundUpdateCommandBlock, ServerboundUpdateCommandBlockMinecart,
    ServerboundUpdateJigsawBlock, ServerboundUpdateSign, ServerboundUpdateStructureBlock,
    ServerboundUseEntity, ServerboundUseItem, ServerboundVehicleMove,
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

fn encode_string(s: &str, dst: &mut BytesMut) -> Result<(), ProtocolError> {
    let bytes = s.as_bytes();
    VarInt(bytes.len() as i32).encode(dst)?;
    dst.put_slice(bytes);
    Ok(())
}

fn decode_string(src: &mut Bytes) -> Result<String, ProtocolError> {
    let len = VarInt::decode(src)?.0 as usize;
    if src.remaining() < len {
        return Err(ProtocolError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Missing bytes for string",
        )));
    }
    let mut buf = vec![0u8; len];
    src.copy_to_slice(&mut buf);
    String::from_utf8(buf).map_err(|_| {
        ProtocolError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid UTF-8 in string",
        ))
    })
}

mod packets {
    use super::*;

    macro_rules! raw_packet {
        ($name:ident, $id:expr) => {
            #[derive(Debug, Clone, PartialEq)]
            pub struct $name {
                pub raw: Vec<u8>,
            }
            impl PacketId for $name {
                fn packet_id(_ver: u32) -> u8 {
                    $id
                }
            }
            impl Encode for $name {
                fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
                    dst.put_slice(&self.raw);
                    Ok(())
                }
            }
            impl Decode for $name {
                fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
                    let len = src.remaining();
                    let mut raw = vec![0u8; len];
                    src.copy_to_slice(&mut raw);
                    Ok(Self { raw })
                }
            }
        };
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSpawnEntity {
        pub entity_id: VarInt,
        pub entity_uuid: uuid::Uuid,
        pub entity_type: VarInt,
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub pitch: i8,
        pub yaw: i8,
        pub head_pitch: i8,
        pub data: VarInt,
        pub velocity_x: i16,
        pub velocity_y: i16,
        pub velocity_z: i16,
    }

    impl PacketId for ClientboundSpawnEntity {
        fn packet_id(_ver: u32) -> u8 {
            0x00
        }
    }

    impl Encode for ClientboundSpawnEntity {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            let (hi, lo) = self.entity_uuid.as_u64_pair();
            dst.put_i64(hi as i64);
            dst.put_i64(lo as i64);
            self.entity_type.encode(dst)?;
            dst.put_f64(self.x);
            dst.put_f64(self.y);
            dst.put_f64(self.z);
            dst.put_i8(self.pitch);
            dst.put_i8(self.yaw);
            dst.put_i8(self.head_pitch);
            self.data.encode(dst)?;
            dst.put_i16(self.velocity_x);
            dst.put_i16(self.velocity_y);
            dst.put_i16(self.velocity_z);
            Ok(())
        }
    }

    impl Decode for ClientboundSpawnEntity {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            let hi = src.get_i64();
            let lo = src.get_i64();
            let entity_uuid = uuid::Uuid::from_u64_pair(hi as u64, lo as u64);
            let entity_type = VarInt::decode(src)?;
            let x = src.get_f64();
            let y = src.get_f64();
            let z = src.get_f64();
            let pitch = src.get_i8();
            let yaw = src.get_i8();
            let head_pitch = src.get_i8();
            let data = VarInt::decode(src)?;
            let velocity_x = src.get_i16();
            let velocity_y = src.get_i16();
            let velocity_z = src.get_i16();
            Ok(Self {
                entity_id,
                entity_uuid,
                entity_type,
                x,
                y,
                z,
                pitch,
                yaw,
                head_pitch,
                data,
                velocity_x,
                velocity_y,
                velocity_z,
            })
        }
    }

    raw_packet!(ClientboundSpawnExperienceOrb, 0x01);

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSpawnLivingEntity {
        pub entity_id: VarInt,
        pub entity_uuid: uuid::Uuid,
        pub entity_type: VarInt,
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub yaw: i8,
        pub pitch: i8,
        pub head_pitch: i8,
        pub velocity_x: i16,
        pub velocity_y: i16,
        pub velocity_z: i16,
    }

    impl PacketId for ClientboundSpawnLivingEntity {
        fn packet_id(_ver: u32) -> u8 {
            0x02
        }
    }

    impl Encode for ClientboundSpawnLivingEntity {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            let (hi, lo) = self.entity_uuid.as_u64_pair();
            dst.put_i64(hi as i64);
            dst.put_i64(lo as i64);
            self.entity_type.encode(dst)?;
            dst.put_f64(self.x);
            dst.put_f64(self.y);
            dst.put_f64(self.z);
            dst.put_i8(self.yaw);
            dst.put_i8(self.pitch);
            dst.put_i8(self.head_pitch);
            dst.put_i16(self.velocity_x);
            dst.put_i16(self.velocity_y);
            dst.put_i16(self.velocity_z);
            Ok(())
        }
    }

    impl Decode for ClientboundSpawnLivingEntity {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            let hi = src.get_i64();
            let lo = src.get_i64();
            let entity_uuid = uuid::Uuid::from_u64_pair(hi as u64, lo as u64);
            let entity_type = VarInt::decode(src)?;
            let x = src.get_f64();
            let y = src.get_f64();
            let z = src.get_f64();
            let yaw = src.get_i8();
            let pitch = src.get_i8();
            let head_pitch = src.get_i8();
            let velocity_x = src.get_i16();
            let velocity_y = src.get_i16();
            let velocity_z = src.get_i16();
            Ok(Self {
                entity_id,
                entity_uuid,
                entity_type,
                x,
                y,
                z,
                yaw,
                pitch,
                head_pitch,
                velocity_x,
                velocity_y,
                velocity_z,
            })
        }
    }

    raw_packet!(ClientboundSpawnPainting, 0x03);

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSpawnPlayer {
        pub entity_id: VarInt,
        pub player_uuid: uuid::Uuid,
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub yaw: i8,
        pub pitch: i8,
    }

    impl PacketId for ClientboundSpawnPlayer {
        fn packet_id(_ver: u32) -> u8 {
            0x04
        }
    }

    impl Encode for ClientboundSpawnPlayer {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            let (hi, lo) = self.player_uuid.as_u64_pair();
            dst.put_i64(hi as i64);
            dst.put_i64(lo as i64);
            dst.put_f64(self.x);
            dst.put_f64(self.y);
            dst.put_f64(self.z);
            dst.put_i8(self.yaw);
            dst.put_i8(self.pitch);
            Ok(())
        }
    }

    impl Decode for ClientboundSpawnPlayer {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            let hi = src.get_i64();
            let lo = src.get_i64();
            let player_uuid = uuid::Uuid::from_u64_pair(hi as u64, lo as u64);
            let x = src.get_f64();
            let y = src.get_f64();
            let z = src.get_f64();
            let yaw = src.get_i8();
            let pitch = src.get_i8();
            Ok(Self {
                entity_id,
                player_uuid,
                x,
                y,
                z,
                yaw,
                pitch,
            })
        }
    }

    raw_packet!(ClientboundEntityAnimation, 0x05);
    raw_packet!(ClientboundAwardStats, 0x06);
    raw_packet!(ClientboundAcknowledgePlayerDigging, 0x07);
    raw_packet!(ClientboundBlockBreakAnimation, 0x08);
    raw_packet!(ClientboundBlockEntityData, 0x09);
    raw_packet!(ClientboundBlockAction, 0x0A);

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundBlockChange {
        pub location: VarInt,
        pub block_state: VarInt,
    }

    impl PacketId for ClientboundBlockChange {
        fn packet_id(_ver: u32) -> u8 {
            0x0B
        }
    }

    impl Encode for ClientboundBlockChange {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.location.encode(dst)?;
            self.block_state.encode(dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundBlockChange {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let location = VarInt::decode(src)?;
            let block_state = VarInt::decode(src)?;
            Ok(Self {
                location,
                block_state,
            })
        }
    }

    raw_packet!(ClientboundBossBar, 0x0C);
    raw_packet!(ClientboundServerDifficulty, 0x0D);

    raw_packet!(ClientboundTabComplete, 0x0F);
    raw_packet!(ClientboundDeclareCommands, 0x10);
    raw_packet!(ClientboundWindowConfirmation, 0x11);

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundCloseWindow {
        pub window_id: u8,
    }

    impl PacketId for ClientboundCloseWindow {
        fn packet_id(_ver: u32) -> u8 {
            0x12
        }
    }

    impl Encode for ClientboundCloseWindow {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.window_id);
            Ok(())
        }
    }

    impl Decode for ClientboundCloseWindow {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1)?;
            let window_id = src.get_u8();
            Ok(Self { window_id })
        }
    }

    raw_packet!(ClientboundWindowProperty, 0x14);
    raw_packet!(ClientboundSetCooldown, 0x16);

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundWindowItems {
        pub window_id: u8,
        pub slots: Vec<crate::types::Slot>,
        pub carried_item: crate::types::Slot,
    }

    impl PacketId for ClientboundWindowItems {
        fn packet_id(_ver: u32) -> u8 {
            0x13
        }
    }

    impl Encode for ClientboundWindowItems {
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

    impl Decode for ClientboundWindowItems {
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
    pub struct ClientboundSetSlot {
        pub window_id: u8,
        pub slot: i16,
        pub slot_data: crate::types::Slot,
    }

    impl PacketId for ClientboundSetSlot {
        fn packet_id(_ver: u32) -> u8 {
            0x15
        }
    }

    impl Encode for ClientboundSetSlot {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.window_id);
            dst.put_i16(self.slot);
            self.slot_data.encode(dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundSetSlot {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1 + 2)?;
            let window_id = src.get_u8();
            let slot = src.get_i16();
            let slot_data = crate::types::Slot::decode(src)?;
            Ok(Self {
                window_id,
                slot,
                slot_data,
            })
        }
    }

    raw_packet!(ClientboundEntityStatus, 0x1A);
    raw_packet!(ClientboundExplosion, 0x1B);
    raw_packet!(ClientboundUnloadChunk, 0x1C);
    raw_packet!(ClientboundChangeGameState, 0x1D);
    raw_packet!(ClientboundOpenHorseWindow, 0x1E);

    raw_packet!(ClientboundInitializeWorldBorder, 0x20);
    raw_packet!(ClientboundChunkData, 0x21);
    raw_packet!(ClientboundEffect, 0x22);
    raw_packet!(ClientboundParticle, 0x23);
    raw_packet!(ClientboundUpdateLight, 0x24);

    raw_packet!(ClientboundMap, 0x25);
    raw_packet!(ClientboundTradeList, 0x26);

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundEntityPosition {
        pub entity_id: VarInt,
        pub delta_x: i16,
        pub delta_y: i16,
        pub delta_z: i16,
        pub on_ground: bool,
    }

    impl PacketId for ClientboundEntityPosition {
        fn packet_id(_ver: u32) -> u8 {
            0x27
        }
    }

    impl Encode for ClientboundEntityPosition {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_i16(self.delta_x);
            dst.put_i16(self.delta_y);
            dst.put_i16(self.delta_z);
            dst.put_u8(if self.on_ground { 1 } else { 0 });
            Ok(())
        }
    }

    impl Decode for ClientboundEntityPosition {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            let delta_x = src.get_i16();
            let delta_y = src.get_i16();
            let delta_z = src.get_i16();
            let on_ground = src.get_u8() != 0;
            Ok(Self {
                entity_id,
                delta_x,
                delta_y,
                delta_z,
                on_ground,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundEntityPositionAndRotation {
        pub entity_id: VarInt,
        pub delta_x: i16,
        pub delta_y: i16,
        pub delta_z: i16,
        pub yaw: i8,
        pub pitch: i8,
        pub on_ground: bool,
    }

    impl PacketId for ClientboundEntityPositionAndRotation {
        fn packet_id(_ver: u32) -> u8 {
            0x28
        }
    }

    impl Encode for ClientboundEntityPositionAndRotation {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_i16(self.delta_x);
            dst.put_i16(self.delta_y);
            dst.put_i16(self.delta_z);
            dst.put_i8(self.yaw);
            dst.put_i8(self.pitch);
            dst.put_u8(if self.on_ground { 1 } else { 0 });
            Ok(())
        }
    }

    impl Decode for ClientboundEntityPositionAndRotation {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            let delta_x = src.get_i16();
            let delta_y = src.get_i16();
            let delta_z = src.get_i16();
            let yaw = src.get_i8();
            let pitch = src.get_i8();
            let on_ground = src.get_u8() != 0;
            Ok(Self {
                entity_id,
                delta_x,
                delta_y,
                delta_z,
                yaw,
                pitch,
                on_ground,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundEntityRotation {
        pub entity_id: VarInt,
        pub yaw: i8,
        pub pitch: i8,
        pub on_ground: bool,
    }

    impl PacketId for ClientboundEntityRotation {
        fn packet_id(_ver: u32) -> u8 {
            0x29
        }
    }

    impl Encode for ClientboundEntityRotation {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_i8(self.yaw);
            dst.put_i8(self.pitch);
            dst.put_u8(if self.on_ground { 1 } else { 0 });
            Ok(())
        }
    }

    impl Decode for ClientboundEntityRotation {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            let yaw = src.get_i8();
            let pitch = src.get_i8();
            let on_ground = src.get_u8() != 0;
            Ok(Self {
                entity_id,
                yaw,
                pitch,
                on_ground,
            })
        }
    }

    raw_packet!(ClientboundVehicleMove, 0x2A);
    raw_packet!(ClientboundOpenBook, 0x2B);

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundOpenWindow {
        pub window_id: VarInt,
        pub window_type: VarInt,
        pub window_title: String,
    }

    impl PacketId for ClientboundOpenWindow {
        fn packet_id(_ver: u32) -> u8 {
            0x2C
        }
    }

    impl Encode for ClientboundOpenWindow {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.window_id.encode(dst)?;
            self.window_type.encode(dst)?;
            encode_string(&self.window_title, dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundOpenWindow {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let window_id = VarInt::decode(src)?;
            let window_type = VarInt::decode(src)?;
            let window_title = decode_string(src)?;
            Ok(Self {
                window_id,
                window_type,
                window_title,
            })
        }
    }

    raw_packet!(ClientboundOpenSignEditor, 0x2D);
    raw_packet!(ClientboundCraftRecipeResponse, 0x2E);

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundPlayerAbilities {
        pub flags: u8,
        pub flying_speed: f32,
        pub walking_speed: f32,
    }

    impl PacketId for ClientboundPlayerAbilities {
        fn packet_id(_ver: u32) -> u8 {
            0x2F
        }
    }

    impl Encode for ClientboundPlayerAbilities {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.flags);
            dst.put_f32(self.flying_speed);
            dst.put_f32(self.walking_speed);
            Ok(())
        }
    }

    impl Decode for ClientboundPlayerAbilities {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1 + 4 + 4)?;
            let flags = src.get_u8();
            let flying_speed = src.get_f32();
            let walking_speed = src.get_f32();
            Ok(Self {
                flags,
                flying_speed,
                walking_speed,
            })
        }
    }

    raw_packet!(ClientboundCombatEvent, 0x30);
    raw_packet!(ClientboundPlayerInfo, 0x31);
    raw_packet!(ClientboundFacePlayer, 0x31);

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundDestroyEntities {
        pub entity_ids: VarInt,
        pub count: VarInt,
        pub entities: Vec<VarInt>,
    }

    impl PacketId for ClientboundDestroyEntities {
        fn packet_id(_ver: u32) -> u8 {
            0x36
        }
    }

    impl Encode for ClientboundDestroyEntities {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_ids.encode(dst)?;
            for entity in &self.entities {
                entity.encode(dst)?;
            }
            Ok(())
        }
    }

    impl Decode for ClientboundDestroyEntities {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_ids = VarInt::decode(src)?;
            let count = entity_ids.0 as usize;
            let mut entities = Vec::with_capacity(count);
            for _ in 0..count {
                entities.push(VarInt::decode(src)?);
            }
            Ok(Self {
                entity_ids,
                count: entity_ids,
                entities,
            })
        }
    }

    raw_packet!(ClientboundRemoveEntityEffect, 0x37);
    raw_packet!(ClientboundResourcePackSend, 0x38);

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundEntityHeadLook {
        pub entity_id: VarInt,
        pub head_yaw: i8,
    }

    impl PacketId for ClientboundEntityHeadLook {
        fn packet_id(_ver: u32) -> u8 {
            0x3A
        }
    }

    impl Encode for ClientboundEntityHeadLook {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_i8(self.head_yaw);
            Ok(())
        }
    }

    impl Decode for ClientboundEntityHeadLook {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            let head_yaw = src.get_i8();
            Ok(Self {
                entity_id,
                head_yaw,
            })
        }
    }

    raw_packet!(ClientboundMultiBlockChange, 0x3B);
    raw_packet!(ClientboundSelectAdvancementsTab, 0x3C);
    raw_packet!(ClientboundWorldBorder, 0x3D);
    raw_packet!(ClientboundCamera, 0x3E);

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundHeldItemChange {
        pub slot: i8,
    }

    impl PacketId for ClientboundHeldItemChange {
        fn packet_id(_ver: u32) -> u8 {
            0x3F
        }
    }

    impl Encode for ClientboundHeldItemChange {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i8(self.slot);
            Ok(())
        }
    }

    impl Decode for ClientboundHeldItemChange {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1)?;
            let slot = src.get_i8();
            Ok(Self { slot })
        }
    }

    raw_packet!(ClientboundUpdateViewPosition, 0x40);
    raw_packet!(ClientboundUpdateViewDistance, 0x41);

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSpawnPosition {
        pub location: VarInt,
    }

    impl PacketId for ClientboundSpawnPosition {
        fn packet_id(_ver: u32) -> u8 {
            0x42
        }
    }

    impl Encode for ClientboundSpawnPosition {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.location.encode(dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundSpawnPosition {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let location = VarInt::decode(src)?;
            Ok(Self { location })
        }
    }

    raw_packet!(ClientboundDisplayScoreboard, 0x43);
    raw_packet!(ClientboundEntityMetadata, 0x44);
    raw_packet!(ClientboundAttachEntity, 0x45);

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundEntityVelocity {
        pub entity_id: VarInt,
        pub velocity_x: i16,
        pub velocity_y: i16,
        pub velocity_z: i16,
    }

    impl PacketId for ClientboundEntityVelocity {
        fn packet_id(_ver: u32) -> u8 {
            0x46
        }
    }

    impl Encode for ClientboundEntityVelocity {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_i16(self.velocity_x);
            dst.put_i16(self.velocity_y);
            dst.put_i16(self.velocity_z);
            Ok(())
        }
    }

    impl Decode for ClientboundEntityVelocity {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            let velocity_x = src.get_i16();
            let velocity_y = src.get_i16();
            let velocity_z = src.get_i16();
            Ok(Self {
                entity_id,
                velocity_x,
                velocity_y,
                velocity_z,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundEntityEquipment {
        pub entity_id: i32,
        pub slot: VarInt,
        pub item: crate::types::Slot,
    }

    impl PacketId for ClientboundEntityEquipment {
        fn packet_id(_ver: u32) -> u8 {
            0x47
        }
    }

    impl Encode for ClientboundEntityEquipment {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            VarInt(self.entity_id).encode(dst)?;
            self.slot.encode(dst)?;
            self.item.encode(dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundEntityEquipment {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?.0;
            let slot = VarInt::decode(src)?;
            let item = crate::types::Slot::decode(src)?;
            Ok(Self {
                entity_id,
                slot,
                item,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetExperience {
        pub experience_bar: f32,
        pub level: VarInt,
        pub total_experience: VarInt,
    }

    impl PacketId for ClientboundSetExperience {
        fn packet_id(_ver: u32) -> u8 {
            0x48
        }
    }

    impl Encode for ClientboundSetExperience {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_f32(self.experience_bar);
            self.level.encode(dst)?;
            self.total_experience.encode(dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundSetExperience {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4)?;
            let experience_bar = src.get_f32();
            let level = VarInt::decode(src)?;
            let total_experience = VarInt::decode(src)?;
            Ok(Self {
                experience_bar,
                level,
                total_experience,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundUpdateHealth {
        pub health: f32,
        pub food: VarInt,
        pub food_saturation: f32,
    }

    impl PacketId for ClientboundUpdateHealth {
        fn packet_id(_ver: u32) -> u8 {
            0x49
        }
    }

    impl Encode for ClientboundUpdateHealth {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_f32(self.health);
            self.food.encode(dst)?;
            dst.put_f32(self.food_saturation);
            Ok(())
        }
    }

    impl Decode for ClientboundUpdateHealth {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4)?;
            let health = src.get_f32();
            let food = VarInt::decode(src)?;
            let food_saturation = src.get_f32();
            Ok(Self {
                health,
                food,
                food_saturation,
            })
        }
    }

    raw_packet!(ClientboundScoreboardObjective, 0x4A);
    raw_packet!(ClientboundSetPassengers, 0x4B);
    raw_packet!(ClientboundTeams, 0x4C);
    raw_packet!(ClientboundUpdateScore, 0x4D);

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundTimeUpdate {
        pub world_age: i64,
        pub time_of_day: i64,
    }

    impl PacketId for ClientboundTimeUpdate {
        fn packet_id(_ver: u32) -> u8 {
            0x4E
        }
    }

    impl Encode for ClientboundTimeUpdate {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i64(self.world_age);
            dst.put_i64(self.time_of_day);
            Ok(())
        }
    }

    impl Decode for ClientboundTimeUpdate {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8 + 8)?;
            let world_age = src.get_i64();
            let time_of_day = src.get_i64();
            Ok(Self {
                world_age,
                time_of_day,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundTitle {
        pub action: VarInt,
        pub title_text: Option<String>,
        pub fade_in: i32,
        pub stay: i32,
        pub fade_out: i32,
    }

    impl PacketId for ClientboundTitle {
        fn packet_id(_ver: u32) -> u8 {
            0x4F
        }
    }

    impl Encode for ClientboundTitle {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.action.encode(dst)?;
            if let Some(ref text) = self.title_text {
                encode_string(text, dst)?;
                dst.put_i32(self.fade_in);
                dst.put_i32(self.stay);
                dst.put_i32(self.fade_out);
            }
            Ok(())
        }
    }

    impl Decode for ClientboundTitle {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let action = VarInt::decode(src)?;
            let mut title_text = None;
            let mut fade_in = 0;
            let mut stay = 0;
            let mut fade_out = 0;
            if action.0 == 0 || action.0 == 2 {
                title_text = Some(decode_string(src)?);
                fade_in = src.get_i32();
                stay = src.get_i32();
                fade_out = src.get_i32();
            }
            Ok(Self {
                action,
                title_text,
                fade_in,
                stay,
                fade_out,
            })
        }
    }

    raw_packet!(ClientboundEntitySoundEffect, 0x50);
    raw_packet!(ClientboundStopSound, 0x52);
    raw_packet!(ClientboundSoundEffect, 0x4D);

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundPlayerListHeaderAndFooter {
        pub header: String,
        pub footer: String,
    }

    impl PacketId for ClientboundPlayerListHeaderAndFooter {
        fn packet_id(_ver: u32) -> u8 {
            0x53
        }
    }

    impl Encode for ClientboundPlayerListHeaderAndFooter {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_string(&self.header, dst)?;
            encode_string(&self.footer, dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundPlayerListHeaderAndFooter {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let header = decode_string(src)?;
            let footer = decode_string(src)?;
            Ok(Self { header, footer })
        }
    }

    raw_packet!(ClientboundNbtQueryResponse, 0x54);
    raw_packet!(ClientboundCollectItem, 0x55);

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundEntityTeleport {
        pub entity_id: VarInt,
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub yaw: i8,
        pub pitch: i8,
        pub on_ground: bool,
    }

    impl PacketId for ClientboundEntityTeleport {
        fn packet_id(_ver: u32) -> u8 {
            0x56
        }
    }

    impl Encode for ClientboundEntityTeleport {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_f64(self.x);
            dst.put_f64(self.y);
            dst.put_f64(self.z);
            dst.put_i8(self.yaw);
            dst.put_i8(self.pitch);
            dst.put_u8(if self.on_ground { 1 } else { 0 });
            Ok(())
        }
    }

    impl Decode for ClientboundEntityTeleport {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            let x = src.get_f64();
            let y = src.get_f64();
            let z = src.get_f64();
            let yaw = src.get_i8();
            let pitch = src.get_i8();
            let on_ground = src.get_u8() != 0;
            Ok(Self {
                entity_id,
                x,
                y,
                z,
                yaw,
                pitch,
                on_ground,
            })
        }
    }

    raw_packet!(ClientboundAdvancements, 0x57);
    raw_packet!(ClientboundEntityProperties, 0x58);
    raw_packet!(ClientboundEntityEffect, 0x59);
    raw_packet!(ClientboundDeclareRecipes, 0x5A);
    raw_packet!(ClientboundTags, 0x5B);

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundTeleportConfirm {
        pub teleport_id: VarInt,
    }

    impl PacketId for ServerboundTeleportConfirm {
        fn packet_id(_ver: u32) -> u8 {
            0x00
        }
    }

    impl Encode for ServerboundTeleportConfirm {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.teleport_id.encode(dst)?;
            Ok(())
        }
    }

    impl Decode for ServerboundTeleportConfirm {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let teleport_id = VarInt::decode(src)?;
            Ok(Self { teleport_id })
        }
    }

    raw_packet!(ServerboundQueryBlockNbt, 0x01);
    raw_packet!(ServerboundSetDifficulty, 0x02);

    raw_packet!(ServerboundTabComplete, 0x06);
    raw_packet!(ServerboundClickWindowButton, 0x08);
    raw_packet!(ServerboundClickWindow, 0x09);
    raw_packet!(ServerboundCloseWindow, 0x0A);
    raw_packet!(ServerboundEditBook, 0x0C);
    raw_packet!(ServerboundQueryEntityNbt, 0x0D);

    raw_packet!(ServerboundGenerateStructure, 0x0F);

    raw_packet!(ServerboundLockDifficulty, 0x11);

    raw_packet!(ServerboundMovePlayerStatus, 0x15);
    raw_packet!(ServerboundVehicleMove, 0x16);
    raw_packet!(ServerboundSteerBoat, 0x17);
    raw_packet!(ServerboundPickItem, 0x18);
    raw_packet!(ServerboundCraftRecipeRequest, 0x19);
    raw_packet!(ServerboundPlayerAbilities, 0x1A);
    raw_packet!(ServerboundPlayerDigging, 0x1B);
    raw_packet!(ServerboundEntityAction, 0x1C);
    raw_packet!(ServerboundSteerVehicle, 0x1D);
    raw_packet!(ServerboundPlaceRecipe, 0x1F);
    raw_packet!(ServerboundMovePlayerPosRotSb, 0x20);
    raw_packet!(ServerboundResourcePackStatus, 0x21);
    raw_packet!(ServerboundSelectTrade, 0x22);
    raw_packet!(ServerboundSetBeaconEffect, 0x23);
    raw_packet!(ServerboundHeldItemChange, 0x25);
    raw_packet!(ServerboundUpdateCommandBlock, 0x26);
    raw_packet!(ServerboundUpdateCommandBlockMinecart, 0x27);
    raw_packet!(ServerboundCreativeModeSlot, 0x28);
    raw_packet!(ServerboundUpdateJigsawBlock, 0x29);
    raw_packet!(ServerboundUpdateStructureBlock, 0x2A);
    raw_packet!(ServerboundUpdateSign, 0x2B);
    raw_packet!(ServerboundSwingArm, 0x2C);
    raw_packet!(ServerboundSpectate, 0x2D);
    raw_packet!(ServerboundPlayerBlockPlacement, 0x2E);
    raw_packet!(ServerboundUseItem, 0x2F);

    raw_packet!(ServerboundUseEntity, 0x0E);

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundNamedSoundEffect {
        pub sound_name: String,
        pub sound_category: VarInt,
        pub effect_position_x: i32,
        pub effect_position_y: i32,
        pub effect_position_z: i32,
        pub volume: f32,
        pub pitch: f32,
    }

    impl PacketId for ClientboundNamedSoundEffect {
        fn packet_id(_ver: u32) -> u8 {
            0x19
        }
    }

    impl Encode for ClientboundNamedSoundEffect {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_string(&self.sound_name, dst)?;
            self.sound_category.encode(dst)?;
            dst.put_i32(self.effect_position_x);
            dst.put_i32(self.effect_position_y);
            dst.put_i32(self.effect_position_z);
            dst.put_f32(self.volume);
            dst.put_f32(self.pitch);
            Ok(())
        }
    }

    impl Decode for ClientboundNamedSoundEffect {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let sound_name = decode_string(src)?;
            let sound_category = VarInt::decode(src)?;
            if src.remaining() < 20 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ClientboundNamedSoundEffect position/volume/pitch",
                )));
            }
            let effect_position_x = src.get_i32();
            let effect_position_y = src.get_i32();
            let effect_position_z = src.get_i32();
            let volume = src.get_f32();
            let pitch = src.get_f32();
            Ok(Self {
                sound_name,
                sound_category,
                effect_position_x,
                effect_position_y,
                effect_position_z,
                volume,
                pitch,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundJoinGame {
        pub entity_id: i32,
        pub is_hardcore: bool,
        pub game_mode: u8,
        pub previous_game_mode: i8,
        pub world_names: Vec<String>,
        pub dimension: String,
        pub world_name: String,
        pub hashed_seed: i64,
        pub max_players: VarInt,
        pub view_distance: VarInt,
        pub reduced_debug_info: bool,
        pub enable_respawn_screen: bool,
        pub is_debug: bool,
        pub is_flat: bool,
    }

    impl PacketId for ClientboundJoinGame {
        fn packet_id(_ver: u32) -> u8 {
            0x24
        }
    }

    impl Encode for ClientboundJoinGame {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.entity_id);
            dst.put_u8(self.is_hardcore as u8);
            dst.put_u8(self.game_mode);
            dst.put_i8(self.previous_game_mode);
            VarInt(self.world_names.len() as i32).encode(dst)?;
            for name in &self.world_names {
                encode_string(name, dst)?;
            }
            encode_string(&self.dimension, dst)?;
            encode_string(&self.world_name, dst)?;
            dst.put_i64(self.hashed_seed);
            self.max_players.encode(dst)?;
            self.view_distance.encode(dst)?;
            dst.put_u8(self.reduced_debug_info as u8);
            dst.put_u8(self.enable_respawn_screen as u8);
            dst.put_u8(self.is_debug as u8);
            dst.put_u8(self.is_flat as u8);
            Ok(())
        }
    }

    impl Decode for ClientboundJoinGame {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            if src.remaining() < 4 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ClientboundJoinGame entity_id",
                )));
            }
            let entity_id = src.get_i32();
            if src.remaining() < 3 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ClientboundJoinGame flags",
                )));
            }
            let is_hardcore = src.get_u8() != 0;
            let game_mode = src.get_u8();
            let previous_game_mode = src.get_i8();
            let world_count = VarInt::decode(src)?.0 as usize;
            let mut world_names = Vec::with_capacity(world_count);
            for _ in 0..world_count {
                world_names.push(decode_string(src)?);
            }
            let dimension = decode_string(src)?;
            let world_name = decode_string(src)?;
            if src.remaining() < 8 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ClientboundJoinGame hashed_seed",
                )));
            }
            let hashed_seed = src.get_i64();
            let max_players = VarInt::decode(src)?;
            let view_distance = VarInt::decode(src)?;
            if src.remaining() < 4 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ClientboundJoinGame boolean flags",
                )));
            }
            let reduced_debug_info = src.get_u8() != 0;
            let enable_respawn_screen = src.get_u8() != 0;
            let is_debug = src.get_u8() != 0;
            let is_flat = src.get_u8() != 0;
            Ok(Self {
                entity_id,
                is_hardcore,
                game_mode,
                previous_game_mode,
                world_names,
                dimension,
                world_name,
                hashed_seed,
                max_players,
                view_distance,
                reduced_debug_info,
                enable_respawn_screen,
                is_debug,
                is_flat,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundRespawn {
        pub dimension: String,
        pub world_name: String,
        pub hashed_seed: i64,
        pub game_mode: u8,
        pub previous_game_mode: i8,
        pub is_debug: bool,
        pub is_flat: bool,
        pub copy_metadata: bool,
    }

    impl PacketId for ClientboundRespawn {
        fn packet_id(_ver: u32) -> u8 {
            0x39
        }
    }

    impl Encode for ClientboundRespawn {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_string(&self.dimension, dst)?;
            encode_string(&self.world_name, dst)?;
            dst.put_i64(self.hashed_seed);
            dst.put_u8(self.game_mode);
            dst.put_i8(self.previous_game_mode);
            dst.put_u8(self.is_debug as u8);
            dst.put_u8(self.is_flat as u8);
            dst.put_u8(self.copy_metadata as u8);
            Ok(())
        }
    }

    impl Decode for ClientboundRespawn {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let dimension = decode_string(src)?;
            let world_name = decode_string(src)?;
            if src.remaining() < 8 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ClientboundRespawn hashed_seed",
                )));
            }
            let hashed_seed = src.get_i64();
            if src.remaining() < 3 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ClientboundRespawn game modes",
                )));
            }
            let game_mode = src.get_u8();
            let previous_game_mode = src.get_i8();
            if src.remaining() < 3 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ClientboundRespawn flags",
                )));
            }
            let is_debug = src.get_u8() != 0;
            let is_flat = src.get_u8() != 0;
            let copy_metadata = src.get_u8() != 0;
            Ok(Self {
                dimension,
                world_name,
                hashed_seed,
                game_mode,
                previous_game_mode,
                is_debug,
                is_flat,
                copy_metadata,
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
            0x34
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
            if src.remaining() < 8 * 3 + 4 * 2 + 1 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ClientboundPlayerPosition",
                )));
            }
            let x = src.get_f64();
            let y = src.get_f64();
            let z = src.get_f64();
            let yaw = src.get_f32();
            let pitch = src.get_f32();
            let flags = src.get_u8();
            let teleport_id = VarInt::decode(src)?;
            Ok(Self {
                x,
                y,
                z,
                yaw,
                pitch,
                flags,
                teleport_id,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundKeepAlive {
        pub keep_alive_id: i64,
    }

    impl PacketId for ClientboundKeepAlive {
        fn packet_id(_ver: u32) -> u8 {
            0x1F
        }
    }

    impl Encode for ClientboundKeepAlive {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i64(self.keep_alive_id);
            Ok(())
        }
    }

    impl Decode for ClientboundKeepAlive {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            if src.remaining() < 8 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ClientboundKeepAlive",
                )));
            }
            Ok(Self {
                keep_alive_id: src.get_i64(),
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundKeepAlive {
        pub keep_alive_id: i64,
    }

    impl PacketId for ServerboundKeepAlive {
        fn packet_id(_ver: u32) -> u8 {
            0x10
        }
    }

    impl Encode for ServerboundKeepAlive {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i64(self.keep_alive_id);
            Ok(())
        }
    }

    impl Decode for ServerboundKeepAlive {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            if src.remaining() < 8 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ServerboundKeepAlive",
                )));
            }
            Ok(Self {
                keep_alive_id: src.get_i64(),
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundChatMessage {
        pub json_message: String,
        pub position: u8,
        pub sender: uuid::Uuid,
    }

    impl PacketId for ClientboundChatMessage {
        fn packet_id(_ver: u32) -> u8 {
            0x0E
        }
    }

    impl Encode for ClientboundChatMessage {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_string(&self.json_message, dst)?;
            dst.put_u8(self.position);
            let (hi, lo) = self.sender.as_u64_pair();
            dst.put_i64(hi as i64);
            dst.put_i64(lo as i64);
            Ok(())
        }
    }

    impl Decode for ClientboundChatMessage {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let json_message = decode_string(src)?;
            if src.remaining() < 1 + 16 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ClientboundChatMessage position/sender",
                )));
            }
            let position = src.get_u8();
            let hi = src.get_i64() as u64;
            let lo = src.get_i64() as u64;
            let sender = uuid::Uuid::from_u64_pair(hi, lo);
            Ok(Self {
                json_message,
                position,
                sender,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundChatMessage {
        pub message: String,
    }

    impl PacketId for ServerboundChatMessage {
        fn packet_id(_ver: u32) -> u8 {
            0x03
        }
    }

    impl Encode for ServerboundChatMessage {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_string(&self.message, dst)
        }
    }

    impl Decode for ServerboundChatMessage {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                message: decode_string(src)?,
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
            0x12
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
            if src.remaining() < 8 * 3 + 1 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ServerboundMovePlayerPos",
                )));
            }
            Ok(Self {
                x: src.get_f64(),
                feet_y: src.get_f64(),
                z: src.get_f64(),
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
            0x13
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
            if src.remaining() < 4 * 2 + 1 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ServerboundMovePlayerRot",
                )));
            }
            Ok(Self {
                yaw: src.get_f32(),
                pitch: src.get_f32(),
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
            0x14
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
            if src.remaining() < 8 * 3 + 4 * 2 + 1 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ServerboundMovePlayerPosRot",
                )));
            }
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
            0x0E
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
                    if src.remaining() < 12 {
                        return Err(ProtocolError::Io(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "Missing bytes for InteractAt",
                        )));
                    }
                    InteractAction::InteractAt {
                        target_x: src.get_f32(),
                        target_y: src.get_f32(),
                        target_z: src.get_f32(),
                        hand: VarInt::decode(src)?,
                    }
                },
                _ => {
                    return Err(ProtocolError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Unknown InteractAction type",
                    )))
                },
            };
            if src.remaining() < 1 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for sneaking",
                )));
            }
            Ok(Self {
                entity_id,
                action,
                sneaking: src.get_u8() != 0,
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
            encode_string(&self.channel, dst)?;
            dst.put_slice(&self.data);
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
    pub struct ServerboundPluginMessage {
        pub channel: String,
        pub data: Vec<u8>,
    }

    impl PacketId for ServerboundPluginMessage {
        fn packet_id(_ver: u32) -> u8 {
            0x0B
        }
    }

    impl Encode for ServerboundPluginMessage {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_string(&self.channel, dst)?;
            dst.put_slice(&self.data);
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

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundDisconnect {
        pub reason: String,
    }

    impl PacketId for ClientboundDisconnect {
        fn packet_id(_ver: u32) -> u8 {
            0x19
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
    fn join_game_roundtrip() {
        roundtrip!(
            ClientboundJoinGame,
            ClientboundJoinGame {
                entity_id: 1,
                is_hardcore: false,
                game_mode: 0,
                previous_game_mode: -1,
                world_names: vec!["minecraft:overworld".to_string()],
                dimension: "minecraft:overworld".to_string(),
                world_name: "minecraft:overworld".to_string(),
                hashed_seed: 12345678,
                max_players: VarInt(20),
                view_distance: VarInt(10),
                reduced_debug_info: false,
                enable_respawn_screen: true,
                is_debug: false,
                is_flat: false,
            }
        );
    }

    #[test]
    fn respawn_roundtrip() {
        roundtrip!(
            ClientboundRespawn,
            ClientboundRespawn {
                dimension: "minecraft:the_nether".to_string(),
                world_name: "minecraft:the_nether".to_string(),
                hashed_seed: 0,
                game_mode: 0,
                previous_game_mode: -1,
                is_debug: false,
                is_flat: false,
                copy_metadata: true,
            }
        );
    }

    #[test]
    fn player_position_roundtrip() {
        roundtrip!(
            ClientboundPlayerPosition,
            ClientboundPlayerPosition {
                x: 10.0,
                y: 64.0,
                z: -10.0,
                yaw: 0.0,
                pitch: 0.0,
                flags: 0,
                teleport_id: VarInt(1),
            }
        );
    }

    #[test]
    fn keepalive_roundtrip() {
        roundtrip!(
            ClientboundKeepAlive,
            ClientboundKeepAlive {
                keep_alive_id: 111222333_i64
            }
        );
        roundtrip!(
            ServerboundKeepAlive,
            ServerboundKeepAlive {
                keep_alive_id: 111222333_i64
            }
        );
    }

    #[test]
    fn chat_roundtrip() {
        roundtrip!(
            ClientboundChatMessage,
            ClientboundChatMessage {
                json_message: r#"{"text":"hi"}"#.to_string(),
                position: 1,
                sender: uuid::Uuid::nil(),
            }
        );
        roundtrip!(
            ServerboundChatMessage,
            ServerboundChatMessage {
                message: "hello".to_string()
            }
        );
    }

    #[test]
    fn move_roundtrips() {
        roundtrip!(
            ServerboundMovePlayerPos,
            ServerboundMovePlayerPos {
                x: 0.0,
                feet_y: 64.0,
                z: 0.0,
                on_ground: true
            }
        );
        roundtrip!(
            ServerboundMovePlayerRot,
            ServerboundMovePlayerRot {
                yaw: 45.0,
                pitch: -20.0,
                on_ground: false
            }
        );
        roundtrip!(
            ServerboundMovePlayerPosRot,
            ServerboundMovePlayerPosRot {
                x: 5.0,
                feet_y: 65.0,
                z: -5.0,
                yaw: 90.0,
                pitch: 0.0,
                on_ground: true
            }
        );
    }

    #[test]
    fn interact_roundtrips() {
        roundtrip!(
            ServerboundInteract,
            ServerboundInteract {
                entity_id: VarInt(3),
                action: InteractAction::Attack,
                sneaking: true
            }
        );
        roundtrip!(
            ServerboundInteract,
            ServerboundInteract {
                entity_id: VarInt(4),
                action: InteractAction::Interact { hand: VarInt(0) },
                sneaking: false
            }
        );
        roundtrip!(
            ServerboundInteract,
            ServerboundInteract {
                entity_id: VarInt(5),
                action: InteractAction::InteractAt {
                    target_x: 0.5,
                    target_y: 1.0,
                    target_z: 0.5,
                    hand: VarInt(1)
                },
                sneaking: false,
            }
        );
    }

    #[test]
    fn plugin_message_roundtrip() {
        roundtrip!(
            ClientboundPluginMessage,
            ClientboundPluginMessage {
                channel: "minecraft:brand".to_string(),
                data: b"velocity".to_vec()
            }
        );
        roundtrip!(
            ServerboundPluginMessage,
            ServerboundPluginMessage {
                channel: "minecraft:brand".to_string(),
                data: b"vanilla".to_vec()
            }
        );
    }

    #[test]
    fn disconnect_roundtrip() {
        roundtrip!(
            ClientboundDisconnect,
            ClientboundDisconnect {
                reason: r#"{"text":"cya"}"#.to_string()
            }
        );
    }

    #[test]
    fn raw_stubs_roundtrip() {
        roundtrip!(
            ClientboundChunkData,
            ClientboundChunkData {
                raw: vec![0xFF; 32]
            }
        );
        roundtrip!(
            ClientboundEntityMetadata,
            ClientboundEntityMetadata { raw: vec![] }
        );
        roundtrip!(
            ServerboundPlayerDigging,
            ServerboundPlayerDigging { raw: vec![0x00; 8] }
        );
        roundtrip!(
            ServerboundPlayerBlockPlacement,
            ServerboundPlayerBlockPlacement {
                raw: vec![0xAB, 0xCD]
            }
        );
    }

    #[test]
    fn packet_ids() {
        assert_eq!(ClientboundJoinGame::packet_id(754), 0x24);
        assert_eq!(ClientboundRespawn::packet_id(754), 0x39);
        assert_eq!(ClientboundPlayerPosition::packet_id(754), 0x34);
        assert_eq!(ClientboundKeepAlive::packet_id(754), 0x1F);
        assert_eq!(ServerboundKeepAlive::packet_id(754), 0x10);
        assert_eq!(ClientboundChatMessage::packet_id(754), 0x0E);
        assert_eq!(ServerboundChatMessage::packet_id(754), 0x03);
        assert_eq!(ServerboundMovePlayerPos::packet_id(754), 0x12);
        assert_eq!(ServerboundMovePlayerRot::packet_id(754), 0x13);
        assert_eq!(ServerboundMovePlayerPosRot::packet_id(754), 0x14);
        assert_eq!(ServerboundInteract::packet_id(754), 0x0E);
        assert_eq!(ClientboundPluginMessage::packet_id(754), 0x17);
        assert_eq!(ServerboundPluginMessage::packet_id(754), 0x0B);
        assert_eq!(ClientboundDisconnect::packet_id(754), 0x19);

        assert_eq!(ClientboundSpawnEntity::packet_id(754), 0x00);
        assert_eq!(ClientboundSpawnPlayer::packet_id(754), 0x04);
        assert_eq!(ClientboundBlockChange::packet_id(754), 0x0B);
        assert_eq!(ClientboundChunkData::packet_id(754), 0x21);
        assert_eq!(ClientboundEntityVelocity::packet_id(754), 0x46);
        assert_eq!(ClientboundUpdateHealth::packet_id(754), 0x49);
        assert_eq!(ClientboundDeclareRecipes::packet_id(754), 0x5A);
        assert_eq!(ClientboundTags::packet_id(754), 0x5B);
        assert_eq!(ServerboundTeleportConfirm::packet_id(754), 0x00);
        assert_eq!(ServerboundPlayerDigging::packet_id(754), 0x1B);
        assert_eq!(ServerboundPlayerBlockPlacement::packet_id(754), 0x2E);
        assert_eq!(ServerboundUseItem::packet_id(754), 0x2F);
    }
}
