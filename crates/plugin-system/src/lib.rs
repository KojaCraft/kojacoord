#![deny(clippy::all)]

pub mod api;
pub mod integrity;
pub mod loader;
pub mod manager;
pub mod sandbox;

pub use api::{
    PacketData, PacketDirection, PacketEvent, PacketFilter, PacketHookFn, PacketHookResult, Plugin,
    PluginCommand, PluginContext, PluginEvent, PluginMetadata, PluginResponse,
};
pub use integrity::PluginVerifier;
pub use loader::PluginLoader;
pub use manager::PluginManager;
pub use sandbox::{apply_sandbox, validate_plugin_permissions, SandboxConfig};
