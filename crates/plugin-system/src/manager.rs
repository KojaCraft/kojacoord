//! Loaded-plugin registry and event/packet hook dispatcher.
//!
//! Owns one `Box<dyn Plugin>` per loaded plugin plus the supporting
//! state (command receivers, packet-hook table, per-plugin permission
//! grants). Held inside `std::sync::RwLock` at the proxy level so the
//! relay can grab a read guard for every packet hook without crossing
//! an await — see the field comment on `ProxyState::plugin_manager`.

use crate::api::{
    PacketData, PacketEvent, PacketHookResult, Plugin, PluginCommand, PluginContext, PluginEvent,
    PluginMetadata, PluginPermission, PluginResponse,
};
use crate::sandbox::{apply_sandbox, SandboxConfig};
use crate::wasm_loader::{WasmLoader, WasmPluginAdapter};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};

/// A loaded plugin shared across the proxy. The `Mutex` provides the interior
/// mutability the `Plugin` trait requires (`handle_event`/`on_*` take `&mut self`)
/// while still allowing the instance to be shared between tasks.
pub type SharedPlugin = Arc<Mutex<Box<dyn Plugin>>>;

pub struct PluginManager {
    wasm_loader: Arc<WasmLoader>,
    plugins: HashMap<String, (SharedPlugin, PluginMetadata)>,
    plugin_configs: HashMap<String, PluginContext>,
    packet_hooks: Arc<RwLock<Vec<PacketEvent>>>,
    sandbox_enabled: bool,
    sandbox_config: SandboxConfig,
    /// Allowed permissions per plugin (from config)
    allowed_permissions: HashMap<String, Vec<PluginPermission>>,
    /// Receivers for plugin command channels. Each plugin gets its own
    /// channel so the proxy can route responses per plugin if needed.
    pub command_receivers:
        std::sync::Mutex<HashMap<String, tokio::sync::mpsc::UnboundedReceiver<PluginCommand>>>,
}

impl PluginManager {
    pub fn new() -> Result<Self, anyhow::Error> {
        let wasm_loader = Arc::new(WasmLoader::new()?);
        Ok(Self {
            wasm_loader,
            plugins: HashMap::new(),
            plugin_configs: HashMap::new(),
            packet_hooks: Arc::new(RwLock::new(Vec::new())),
            sandbox_enabled: true,
            sandbox_config: SandboxConfig::default(),
            allowed_permissions: HashMap::new(),
            command_receivers: std::sync::Mutex::new(HashMap::new()),
        })
    }

    /// Create a new PluginManager with a custom WASM loader
    pub fn with_wasm_loader(wasm_loader: WasmLoader) -> Result<Self, anyhow::Error> {
        Ok(Self {
            wasm_loader: Arc::new(wasm_loader),
            plugins: HashMap::new(),
            plugin_configs: HashMap::new(),
            packet_hooks: Arc::new(RwLock::new(Vec::new())),
            sandbox_enabled: true,
            sandbox_config: SandboxConfig::default(),
            allowed_permissions: HashMap::new(),
            command_receivers: std::sync::Mutex::new(HashMap::new()),
        })
    }

    /// Set the allowed permissions for a specific plugin from config
    pub fn set_allowed_permissions(
        &mut self,
        plugin_name: String,
        permissions: Vec<PluginPermission>,
    ) {
        self.allowed_permissions.insert(plugin_name, permissions);
    }

    /// Check if a plugin has a specific permission
    pub fn has_permission(&self, plugin_name: &str, permission: PluginPermission) -> bool {
        if let Some(allowed) = self.allowed_permissions.get(plugin_name) {
            allowed.contains(&permission)
        } else {
            // If no permissions are configured, deny by default
            false
        }
    }

    pub fn enable_sandbox(&mut self, enabled: bool) {
        self.sandbox_enabled = enabled;
        if enabled {
            if let Err(e) = apply_sandbox(&self.sandbox_config) {
                log::error!("Failed to apply sandbox: {}", e);
            }
        }
    }

    pub fn set_sandbox_config(&mut self, config: SandboxConfig) {
        self.sandbox_config = config;
        if self.sandbox_enabled {
            if let Err(e) = apply_sandbox(&self.sandbox_config) {
                log::error!("Failed to apply updated sandbox config: {}", e);
            }
        }
    }

    pub async fn load_plugin<P: AsRef<Path>>(
        &mut self,
        path: P,
        config: HashMap<String, String>,
    ) -> anyhow::Result<PluginMetadata> {
        let path = path.as_ref();
        let plugin_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel::<PluginCommand>();

        let context = PluginContext {
            plugin_id: plugin_name.clone(),
            version: "1.0.0".to_string(),
            config,
            command_tx: Some(cmd_tx),
        };

        // Only load WASM plugins
        let wasm_bytes = std::fs::read(path).context("Failed to read WASM file")?;

        let wasm_instance = self
            .wasm_loader
            .load_plugin(
                plugin_name.clone(),
                "1.0.0".to_string(),
                wasm_bytes,
                &context,
            )
            .await
            .context("Failed to load WASM plugin")?;

        let adapter = WasmPluginAdapter::new(wasm_instance.clone(), self.wasm_loader.clone());
        let metadata = {
            let guard = wasm_instance.lock().unwrap();
            guard.metadata.clone()
        };
        let mut plugin = Box::new(adapter) as Box<dyn Plugin>;

        if metadata.name != plugin_name {
            log::warn!(
                "Plugin name '{}' in metadata doesn't match filename '{}'",
                metadata.name,
                plugin_name
            );
        }

        // Enforce permission sandboxing: check that requested permissions are allowed
        for requested_perm in &metadata.permissions {
            if !self.has_permission(&metadata.name, requested_perm.clone()) {
                log::error!(
                    "Plugin '{}' requested permission {:?} which is not allowed in config. Refusing to load.",
                    metadata.name,
                    requested_perm
                );
                return Err(anyhow::anyhow!(
                    "Plugin requested permission {:?} which is not allowed",
                    requested_perm
                ));
            }
        }

        // Activate the plugin and collect its packet hooks now, while we still
        // own the Box exclusively, so the hooks take effect in `process_packet`.
        if let Err(e) = plugin.on_enable() {
            log::warn!("Plugin '{}' on_enable failed: {}", metadata.name, e);
        }
        let hooks = plugin.register_packet_hooks();
        if !hooks.is_empty() {
            let count = hooks.len();
            let mut hooks_lock = self.packet_hooks.write().unwrap_or_else(|e| e.into_inner());
            hooks_lock.extend(hooks);
            // Sort hooks by priority (descending) so higher priority hooks execute first
            // Higher priority runs first — sort descending.
            hooks_lock.sort_by_key(|h| std::cmp::Reverse(h.priority()));
            log::info!(
                "Registered {} packet hook(s) from plugin '{}'",
                count,
                metadata.name
            );
        }

        self.plugins.insert(
            plugin_name.clone(),
            (Arc::new(Mutex::new(plugin)), metadata.clone()),
        );
        self.plugin_configs.insert(plugin_name.clone(), context);
        {
            let mut rx_lock = self
                .command_receivers
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            rx_lock.insert(plugin_name.clone(), cmd_rx);
        }

        log::info!(
            "Loaded WASM plugin: {} v{} by {}",
            metadata.name,
            metadata.version,
            metadata.author
        );
        Ok(metadata)
    }

    pub async fn load_plugins_from_dir<P: AsRef<Path>>(
        &mut self,
        dir: P,
        configs: HashMap<String, HashMap<String, String>>,
    ) -> anyhow::Result<()> {
        let dir = dir.as_ref();

        // Prevent path traversal attacks by rejecting paths containing '..'.
        if dir
            .components()
            .any(|c| c == std::path::Component::ParentDir)
        {
            return Err(anyhow::anyhow!("Invalid input: {}", dir.display()));
        }

        if !dir.exists() {
            log::warn!("Plugin directory does not exist: {:?}", dir);
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            // Only load WASM plugins
            if path.extension().is_some_and(|ext| ext == "wasm") {
                let plugin_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let config = configs.get(&plugin_name).cloned().unwrap_or_default();

                if let Err(e) = self.load_plugin(&path, config).await {
                    log::error!("Failed to load WASM plugin {:?}: {}", path, e);
                }
            }
        }

        Ok(())
    }

    pub fn unload_plugin(&mut self, name: &str) -> anyhow::Result<PluginMetadata> {
        if let Some((plugin, metadata)) = self.plugins.remove(name) {
            // Run lifecycle teardown before dropping the instance.
            {
                let mut guard = plugin.lock().unwrap_or_else(|e| e.into_inner());
                if let Err(e) = guard.on_disable() {
                    log::warn!("Plugin '{}' on_disable failed: {}", name, e);
                }
                if let Err(e) = guard.on_unload() {
                    log::warn!("Plugin '{}' on_unload failed: {}", name, e);
                }
            }

            self.plugin_configs.remove(name);

            self.packet_hooks
                .write()
                .unwrap_or_else(|e| e.into_inner())
                .clear();

            log::info!("Unloaded WASM plugin: {}", name);
            return Ok(metadata);
        }
        Err(anyhow::anyhow!("Plugin not found: {}", name))
    }

    /// Atomically unload then reload a single plugin from the same path.
    /// Used by the proxy's hot-reload watcher when it sees a plugin WASM file
    /// change on disk. Idempotent — if the plugin isn't currently loaded
    /// this is the same as calling [`Self::load_plugin`].
    pub async fn reload_plugin<P: AsRef<Path>>(
        &mut self,
        path: P,
        config: HashMap<String, String>,
    ) -> anyhow::Result<PluginMetadata> {
        let path_ref = path.as_ref();
        let plugin_name = path_ref
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        // Best-effort unload — ignore "not found" so first-time loads work.
        let _ = self.unload_plugin(&plugin_name);
        self.load_plugin(path, config).await
    }

    pub fn unload_all(&mut self) {
        for name in self.plugins.keys().cloned().collect::<Vec<String>>() {
            let _ = self.unload_plugin(&name);
        }
    }

    /// Deliver an event to every loaded plugin's `handle_event` and collect the
    /// non-`None` responses for the caller to act on. Each plugin is locked only
    /// for the duration of its handler.
    ///
    /// If any plugin returns `PluginResponse::Cancel`, event propagation stops
    /// immediately and the Cancel response is returned. This allows plugins to
    /// veto events (e.g., prevent a player from joining, block a chat message).
    pub fn broadcast_event(&self, event: &PluginEvent) -> Vec<PluginResponse> {
        let mut responses = Vec::new();

        for (name, (plugin, _)) in &self.plugins {
            let mut guard = plugin.lock().unwrap_or_else(|e| e.into_inner());
            match guard.handle_event(event) {
                Ok(Some(PluginResponse::Cancel)) => {
                    log::debug!("Plugin '{}' cancelled event propagation", name);
                    return vec![PluginResponse::Cancel];
                },
                Ok(Some(response)) => responses.push(response),
                Ok(None) => {},
                Err(e) => log::error!("Plugin '{}' handle_event error: {}", name, e),
            }
        }

        responses
    }

    pub fn process_packet(&self, packet: &PacketData) -> PacketHookResult {
        let hooks = self.packet_hooks.read().unwrap_or_else(|e| e.into_inner());

        for hook in hooks.iter() {
            if hook.matches(packet) {
                match hook.execute(packet) {
                    Ok(PacketHookResult::Drop) => return PacketHookResult::Drop,
                    Ok(PacketHookResult::Replace { packet_id, data }) => {
                        return PacketHookResult::Replace { packet_id, data };
                    },
                    Ok(PacketHookResult::Modify(data)) => {
                        let mut modified_packet = packet.clone();
                        modified_packet.data = data;
                        return self.process_packet(&modified_packet);
                    },
                    Ok(PacketHookResult::Forward) => continue,
                    Err(e) => {
                        log::error!("Packet hook error: {}", e);
                        continue;
                    },
                }
            }
        }

        PacketHookResult::Forward
    }

    pub fn loaded_plugins(&self) -> Vec<(String, PluginMetadata)> {
        self.plugins
            .iter()
            .map(|(name, (_, metadata))| (name.clone(), metadata.clone()))
            .collect()
    }

    pub fn get_plugin_metadata(&self, name: &str) -> Option<PluginMetadata> {
        self.plugins.get(name).map(|(_, metadata)| metadata.clone())
    }

    pub fn is_loaded(&self, name: &str) -> bool {
        self.plugins.contains_key(name)
    }

    pub fn get_packet_hooks(&self) -> Vec<HookInfo> {
        let hooks = self.packet_hooks.read().unwrap_or_else(|e| e.into_inner());
        hooks
            .iter()
            .map(|hook| HookInfo {
                packet_id: hook.filter().packet_id,
                protocol_version: hook.filter().protocol_version,
                direction: format!("{:?}", hook.filter().direction),
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct HookInfo {
    pub packet_id: Option<i32>,
    pub protocol_version: Option<u32>,
    pub direction: String,
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new().expect("Failed to create PluginManager")
    }
}
