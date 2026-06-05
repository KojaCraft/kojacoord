use crate::violation::Violation;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AlertConfig {
    pub enabled: bool,
    pub broadcast_to_ops: bool,
    pub broadcast_to_all: bool,
    pub log_to_console: bool,
    pub min_violation_level: f64,
    pub cooldown_seconds: u64,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            broadcast_to_ops: true,
            broadcast_to_all: false,
            log_to_console: true,
            min_violation_level: 1.0,
            cooldown_seconds: 5,
        }
    }
}

pub struct AlertSystem {
    config: Arc<RwLock<AlertConfig>>,
    anticheat_name: String,
    last_alerts: Arc<RwLock<std::collections::HashMap<Uuid, std::time::Instant>>>,
}

impl AlertSystem {
    pub fn new(config: AlertConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            anticheat_name: "Kojacoord Guardian".to_string(),
            last_alerts: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.anticheat_name = name;
        self
    }

    pub async fn send_alert(
        &self,
        violation: &Violation,
        player_name: Option<&str>,
    ) -> Vec<String> {
        let config = self.config.read().await;
        if !config.enabled {
            return Vec::new();
        }

        let severity = violation.value / violation.threshold;
        if severity < config.min_violation_level {
            return Vec::new();
        }

        let uuid = violation.player_uuid;
        let now = std::time::Instant::now();
        let mut last_alerts = self.last_alerts.write().await;

        if let Some(&last) = last_alerts.get(&uuid) {
            if now.duration_since(last).as_secs() < config.cooldown_seconds {
                drop(last_alerts);
                return Vec::new();
            }
        }
        last_alerts.insert(uuid, now);
        drop(last_alerts);

        let messages = self.format_alert(violation, player_name, severity);

        if config.log_to_console {
            for msg in &messages {
                tracing::warn!("[{}] {}", self.anticheat_name, msg);
            }
        }

        messages
    }

    fn format_alert(
        &self,
        violation: &Violation,
        player_name: Option<&str>,
        severity: f64,
    ) -> Vec<String> {
        let name = player_name.unwrap_or("Unknown");
        let check_name = self.humanize_check_name(&violation.check_name);
        let category = violation.check_category.human_name();
        let severity_text = self.get_severity_text(severity);

        let mut messages = Vec::new();

        let main_alert = format!(
            "§c[{}] §f{} §7failed §c{} §7({}) §8[{}x]",
            self.anticheat_name,
            name,
            check_name,
            category,
            format!("{:.1}", severity_text),
        );
        messages.push(main_alert);

        if severity >= 2.0 {
            let detail = format!(
                "§7  ├─ Value: §e{:.2} §7| Threshold: §a{:.2}",
                violation.value, violation.threshold
            );
            messages.push(detail);
        }

        if severity >= 5.0 {
            let warning = format!("§7  └─ §c⚠ HIGH VIOLATION - Possible cheat detected!");
            messages.push(warning);
        }

        messages
    }

    fn humanize_check_name(&self, check_name: &str) -> String {
        match check_name {
            "Speed" => "Speed".to_string(),
            "Killaura" => "Kill Aura".to_string(),
            "Flight" => "Flight".to_string(),
            "NoFall" => "No Fall".to_string(),
            "Reach" => "Reach".to_string(),
            "Timer" => "Timer".to_string(),
            "AutoSprint" => "Auto Sprint".to_string(),
            "Scaffold" => "Scaffold".to_string(),
            "Aimbot" => "Aimbot".to_string(),
            "AutoClicker" => "Auto Clicker".to_string(),
            "Criticals" => "Criticals".to_string(),
            "Jesus" => "Jesus/Water Walk".to_string(),
            "Spider" => "Spider/Climb".to_string(),
            "NoSlow" => "No Slow".to_string(),
            "FastUse" => "Fast Use".to_string(),
            _ => {
                let chars: Vec<char> = check_name.chars().collect();
                let mut result = String::new();
                for (i, c) in chars.iter().enumerate() {
                    if i == 0 || (i > 0 && chars[i - 1].is_uppercase()) {
                        result.extend(c.to_uppercase());
                    } else {
                        result.push(*c);
                    }
                }
                result
            },
        }
    }

    fn get_severity_text(&self, severity: f64) -> &'static str {
        if severity >= 10.0 {
            "EXTREME"
        } else if severity >= 5.0 {
            "HIGH"
        } else if severity >= 2.0 {
            "MEDIUM"
        } else {
            "LOW"
        }
    }

    pub async fn update_config(&self, new_config: AlertConfig) {
        *self.config.write().await = new_config;
    }

    pub fn get_anticheat_name(&self) -> &str {
        &self.anticheat_name
    }
}
