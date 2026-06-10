//! Metrics and analytics.
//!
//! [`collector::MetricsCollector`] is the live counter store the
//! proxy writes to; [`exporter::MetricsExporter`] serves them at the
//! `[metrics] bind` address in Prometheus text-exposition format;
//! [`analytics::AnalyticsEngine`] keeps a rolling event log for
//! diagnostics and the management dashboard.

#![deny(clippy::all)]

pub mod analytics;
pub mod collector;
pub mod exporter;

pub use analytics::{AnalyticsEngine, AnalyticsEvent, EventType};
pub use collector::MetricsCollector;
pub use exporter::MetricsExporter;
