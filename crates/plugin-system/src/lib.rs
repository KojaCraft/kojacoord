#![deny(clippy::all)]

pub mod api;
pub mod loader;
pub mod manager;
pub mod sandbox;

pub use api::{
    PacketData, PacketDirection, PacketEvent, PacketFilter, PacketHookResult, Plugin,
    PluginContext, PluginEvent, PluginMetadata, PluginResponse,
};
pub use loader::PluginLoader;
pub use manager::PluginManager;
pub use sandbox::{apply_sandbox, validate_plugin_permissions, SandboxConfig};
