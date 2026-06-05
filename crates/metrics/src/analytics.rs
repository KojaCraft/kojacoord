use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AnalyticsEngine {
    events: Arc<RwLock<Vec<AnalyticsEvent>>>,
    aggregates: Arc<RwLock<AnalyticsAggregates>>,
    retention_hours: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    PlayerJoin,
    PlayerLeave,
    Violation,
    ServerStatusChange,
    ConnectionError,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsAggregates {
    pub total_players: u64,
    pub peak_players: u64,
    pub total_violations: u64,
    pub violations_by_type: HashMap<String, u64>,
    pub uptime_seconds: u64,
    pub start_time: DateTime<Utc>,
}

impl AnalyticsEngine {
    pub fn new(retention_hours: u64) -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            aggregates: Arc::new(RwLock::new(AnalyticsAggregates {
                total_players: 0,
                peak_players: 0,
                total_violations: 0,
                violations_by_type: HashMap::new(),
                uptime_seconds: 0,
                start_time: Utc::now(),
            })),
            retention_hours,
        }
    }

    pub async fn record_event(&self, event: AnalyticsEvent) {
        let mut events = self.events.write().await;
        events.push(event.clone());

        let mut aggregates = self.aggregates.write().await;
        match &event.event_type {
            EventType::PlayerJoin => {
                aggregates.total_players += 1;
                if aggregates.total_players > aggregates.peak_players {
                    aggregates.peak_players = aggregates.total_players;
                }
            },
            EventType::PlayerLeave => {
                aggregates.total_players = aggregates.total_players.saturating_sub(1);
            },
            EventType::Violation => {
                aggregates.total_violations += 1;
                if let Some(check_name) = event.data.get("check_name").and_then(|v| v.as_str()) {
                    *aggregates
                        .violations_by_type
                        .entry(check_name.to_string())
                        .or_insert(0) += 1;
                }
            },
            _ => {},
        }

        let cutoff = Utc::now() - chrono::Duration::hours(self.retention_hours as i64);
        events.retain(|e| e.timestamp > cutoff);
    }

    pub async fn get_events(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<AnalyticsEvent> {
        let events = self.events.read().await;
        events
            .iter()
            .filter(|e| e.timestamp >= start && e.timestamp <= end)
            .cloned()
            .collect()
    }

    pub async fn get_aggregates(&self) -> AnalyticsAggregates {
        let mut aggregates = self.aggregates.write().await;
        aggregates.uptime_seconds = (Utc::now() - aggregates.start_time).num_seconds() as u64;
        aggregates.clone()
    }

    pub async fn get_violation_stats(&self) -> HashMap<String, u64> {
        let aggregates = self.aggregates.read().await;
        aggregates.violations_by_type.clone()
    }

    pub async fn get_player_history(&self, hours: u64) -> Vec<(DateTime<Utc>, u64)> {
        let events = self.events.read().await;
        let mut history = Vec::new();

        let now = Utc::now();
        for i in 0..hours {
            let hour_start = now - chrono::Duration::hours(i as i64 + 1);
            let hour_end = now - chrono::Duration::hours(i as i64);

            let joins = events
                .iter()
                .filter(|e| e.timestamp >= hour_start && e.timestamp < hour_end)
                .filter(|e| matches!(e.event_type, EventType::PlayerJoin))
                .count() as u64;

            let leaves = events
                .iter()
                .filter(|e| e.timestamp >= hour_start && e.timestamp < hour_end)
                .filter(|e| matches!(e.event_type, EventType::PlayerLeave))
                .count() as u64;

            history.push((hour_start, joins.saturating_sub(leaves)));
        }

        history.reverse();
        history
    }
}

impl Default for AnalyticsEngine {
    fn default() -> Self {
        Self::new(24)
    }
}
