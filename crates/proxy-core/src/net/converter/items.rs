use kojacoord_protocol::types::slot::{LegacySlot, LegacySlotData, Slot};
use kojacoord_protocol::ProtocolVersion;

pub fn is_legacy_slot(ver: ProtocolVersion) -> bool {
    matches!(
        ver,
        ProtocolVersion::V1_6_4
            | ProtocolVersion::V1_7_10
            | ProtocolVersion::V1_8
            | ProtocolVersion::V1_12_2
    )
}

pub fn modern_slot_parsable(ver: ProtocolVersion) -> bool {
    matches!(ver, ProtocolVersion::V1_16_5 | ProtocolVersion::V1_19_4)
}

pub fn has_state_id(ver: ProtocolVersion) -> bool {
    matches!(
        ver,
        ProtocolVersion::V1_19_4 | ProtocolVersion::V1_20_4 | ProtocolVersion::V1_21
    )
}

pub fn modern_slot_to_legacy(slot: &Slot) -> LegacySlot {
    match &slot.0 {
        None => LegacySlot(None),
        Some(d) => LegacySlot(Some(LegacySlotData {
            item_id: d.item_id as i16,
            count: d.count,
            damage: 0,
            nbt: d.nbt.clone(),
        })),
    }
}

pub fn map_equipment_slot(modern_idx: u8) -> Option<i16> {
    match modern_idx {
        0 => Some(0),
        2 => Some(1),
        3 => Some(2),
        4 => Some(3),
        5 => Some(4),
        _ => None,
    }
}
