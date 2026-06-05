#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtocolVersion {
    V1_6_4,

    V1_7_10,

    V1_8,

    V1_12_2,

    V1_16_5,

    V1_19_4,

    V1_20_4,

    V1_21,

    Unknown(u32),
}

impl ProtocolVersion {
    pub fn from_id(id: u32) -> Self {
        match id {
            5 => ProtocolVersion::V1_7_10,
            47 => ProtocolVersion::V1_8,
            78 => ProtocolVersion::V1_6_4,
            340 => ProtocolVersion::V1_12_2,
            754 => ProtocolVersion::V1_16_5,
            762 => ProtocolVersion::V1_19_4,
            765 => ProtocolVersion::V1_20_4,
            767 => ProtocolVersion::V1_21,
            x => ProtocolVersion::Unknown(x),
        }
    }

    pub fn id(&self) -> u32 {
        match self {
            ProtocolVersion::V1_6_4 => 78,
            ProtocolVersion::V1_7_10 => 5,
            ProtocolVersion::V1_8 => 47,
            ProtocolVersion::V1_12_2 => 340,
            ProtocolVersion::V1_16_5 => 754,
            ProtocolVersion::V1_19_4 => 762,
            ProtocolVersion::V1_20_4 => 765,
            ProtocolVersion::V1_21 => 767,
            ProtocolVersion::Unknown(x) => *x,
        }
    }

    pub fn is_supported(&self) -> bool {
        !matches!(self, ProtocolVersion::Unknown(_))
    }
}

pub struct VersionRegistry;

impl VersionRegistry {
    const SUPPORTED: &'static [u32] = &[5, 47, 78, 340, 754, 762, 765, 767];

    pub fn nearest(protocol_id: u32) -> ProtocolVersion {
        let exact = ProtocolVersion::from_id(protocol_id);
        if exact.is_supported() {
            return exact;
        }

        let best = Self::SUPPORTED
            .iter()
            .copied()
            .min_by_key(|&s| (s as i64 - protocol_id as i64).unsigned_abs())
            .unwrap_or(767);

        ProtocolVersion::from_id(best)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_versions_roundtrip() {
        for &id in VersionRegistry::SUPPORTED {
            let v = ProtocolVersion::from_id(id);
            assert!(v.is_supported());
            assert_eq!(v.id(), id);
        }
    }

    #[test]
    fn v1_7_10_recognized() {
        assert_eq!(ProtocolVersion::from_id(5), ProtocolVersion::V1_7_10);
    }

    #[test]
    fn v1_6_4_recognized() {
        assert_eq!(ProtocolVersion::from_id(78), ProtocolVersion::V1_6_4);
    }

    #[test]
    fn nearest_exact() {
        assert_eq!(VersionRegistry::nearest(47), ProtocolVersion::V1_8);
        assert_eq!(VersionRegistry::nearest(5), ProtocolVersion::V1_7_10);
    }

    #[test]
    fn nearest_between_versions() {
        let v = VersionRegistry::nearest(400);
        assert_eq!(v, ProtocolVersion::V1_12_2);
    }
}
