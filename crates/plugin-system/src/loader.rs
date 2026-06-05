use crate::api::{Plugin, PluginContext, PluginMetadata};
use anyhow::{Context, Result};
use libloading::{Library, Symbol};
use std::path::Path;
use std::sync::Arc;

pub struct PluginLoader {
    libraries: Vec<(String, Library)>,
}

impl PluginLoader {
    pub fn new() -> Self {
        Self {
            libraries: Vec::new(),
        }
    }

    pub fn load_plugin<P: AsRef<Path>>(
        &mut self,
        path: P,
        context: &PluginContext,
    ) -> Result<(Arc<dyn Plugin>, PluginMetadata)> {
        let path = path.as_ref();

        unsafe {
            let library = Library::new(path).context("Failed to load plugin library")?;

            let get_metadata: Symbol<unsafe extern "C" fn() -> PluginMetadata> = library
                .get(b"get_metadata")
                .context("Missing get_metadata symbol")?;

            let metadata = get_metadata();

            if !self.check_version_compatibility(&metadata.min_proxy_version) {
                return Err(anyhow::anyhow!(
                    "Plugin requires proxy version {}, current is {}",
                    metadata.min_proxy_version,
                    env!("CARGO_PKG_VERSION")
                ));
            }

            let create_plugin: Symbol<unsafe extern "C" fn() -> *mut dyn Plugin> = library
                .get(b"create_plugin")
                .context("Missing create_plugin symbol")?;

            let plugin_ptr = create_plugin();
            let mut plugin = Arc::from_raw(plugin_ptr);

            Arc::get_mut(&mut plugin)
                .ok_or_else(|| anyhow::anyhow!("Failed to get mutable reference to plugin"))?
                .on_load(context)?;

            self.libraries.push((metadata.name.clone(), library));

            Ok((plugin, metadata))
        }
    }

    pub fn unload_all(&mut self) {
        self.libraries.clear();
    }

    fn check_version_compatibility(&self, required: &str) -> bool {
        let current = env!("CARGO_PKG_VERSION");
        current >= required
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}
