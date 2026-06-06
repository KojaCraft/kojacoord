use crate::player_state::{PlayerAnticheatState, TRUSTED_MODS};

pub struct ModCompatibility {
    enabled: bool,
    strict_mode: bool,
    known_cheat_mods: Vec<String>,
}

impl ModCompatibility {
    pub fn new(enabled: bool, strict_mode: bool) -> Self {
        let known_cheat_mods = vec![
            "wurst".to_string(),
            "huzuni".to_string(),
            "sigma".to_string(),
            "meteor".to_string(),
            "impact".to_string(),
            "liquidbounce".to_string(),
            "bloom".to_string(),
            "novoline".to_string(),
            "kronos".to_string(),
            "reach".to_string(),
            "autoclicker".to_string(),
            "flyhack".to_string(),
            "speedhack".to_string(),
            "killaura".to_string(),
            "triggerbot".to_string(),
            "aimbot".to_string(),
            "esp".to_string(),
            "xray".to_string(),
            "fullbright".to_string(),
            "noslow".to_string(),
            "scaffold".to_string(),
        ];

        Self {
            enabled,
            strict_mode,
            known_cheat_mods,
        }
    }

    pub fn detect_mods_from_brand(&self, brand: &str) -> Vec<String> {
        if !self.enabled {
            return Vec::new();
        }

        let brand_lower = brand.to_lowercase();
        let mut detected = Vec::new();

        for cheat_mod in &self.known_cheat_mods {
            if brand_lower.contains(cheat_mod) {
                detected.push(cheat_mod.clone());
            }
        }

        for trusted_mod in TRUSTED_MODS {
            if brand_lower.contains(trusted_mod) {
                detected.push(format!("trusted:{}", trusted_mod));
            }
        }

        detected
    }

    pub fn is_cheat_mod(&self, mod_name: &str) -> bool {
        if !self.enabled {
            return false;
        }

        let mod_lower = mod_name.to_lowercase();
        self.known_cheat_mods
            .iter()
            .any(|cheat| mod_lower.contains(cheat))
    }

    pub fn is_trusted_mod(&self, mod_name: &str) -> bool {
        if !self.enabled {
            return true;
        }

        let mod_lower = mod_name.to_lowercase();
        TRUSTED_MODS
            .iter()
            .any(|trusted| mod_lower.contains(trusted))
    }

    pub fn should_suppress_check(&self, state: &PlayerAnticheatState, check_name: &str) -> bool {
        if !self.enabled {
            return false;
        }

        if self.strict_mode {
            // In strict mode only suppress a check when the specific trusted mod
            // that could legitimately cause the anomaly is present.
            let has_only_trusted = state
                .detected_mods
                .iter()
                .all(|m| m.starts_with("trusted:") || self.is_trusted_mod(m));

            if has_only_trusted {
                match check_name {
                    // Performance mods (Sodium/Lithium) can alter tick-rate
                    // behaviour slightly on older hardware → minor speed jitter.
                    // ReplayMod can also send packets at non-standard rates.
                    "Speed" | "Timer" => state.detected_mods.iter().any(|m| {
                        m.contains("sodium")
                            || m.contains("lithium")
                            || m.contains("rubidium")
                            || m.contains("replaymod")
                    }),
                    // OptiFine/Iris change entity rendering distance which can
                    // affect the reported hit-distance in some versions.
                    "Reach" => state
                        .detected_mods
                        .iter()
                        .any(|m| m.contains("optifine") || m.contains("iris")),
                    // No other checks should be suppressed for trusted mods.
                    _ => false,
                }
            } else {
                false
            }
        } else {
            // Lenient mode: suppress all checks for any player with a trusted mod.
            state.detected_mods.iter().any(|m| self.is_trusted_mod(m))
        }
    }
}
