use anyhow::Context;
use serde::Deserialize;
use std::sync::Arc;

use crate::s3::S3Client;

pub struct ModpackDownloader {
    http: reqwest::Client,
    s3: Arc<S3Client>,
}

#[derive(Debug, Clone)]
pub struct ModpackInfo {
    pub name: String,
    pub version: String,
    pub minecraft_version: String,
    pub loader: String,
    pub files: Vec<ModpackFile>,
}

#[derive(Debug, Clone)]
pub struct ModpackFile {
    pub path: String,
    pub url: String,
    pub size: u64,
}

impl ModpackDownloader {
    pub fn new(http: reqwest::Client, s3: Arc<S3Client>) -> Self {
        Self { http, s3 }
    }

    pub async fn download_modrinth(
        &self,
        template_name: &str,
        project_id: &str,
        version_id: &str,
    ) -> anyhow::Result<ModpackInfo> {
        // Validate project_id is not empty
        if project_id.is_empty() {
            anyhow::bail!("project_id cannot be empty");
        }

        tracing::info!(
            template = template_name,
            project_id = project_id,
            version_id = version_id,
            "Downloading Modrinth modpack"
        );
        #[derive(Deserialize)]
        struct ModrinthVersion {
            name: String,
            version_number: String,
            game_versions: Vec<String>,
            loaders: Vec<String>,
            files: Vec<ModrinthFile>,
        }
        #[derive(Deserialize)]
        struct ModrinthFile {
            url: String,
            filename: String,
            size: u64,
        }

        let url = format!("https://api.modrinth.com/v2/version/{}", version_id);
        let ver: ModrinthVersion = self
            .http
            .get(&url)
            .header("User-Agent", "KojacoordNetwork/1.0")
            .send()
            .await?
            .json()
            .await
            .context("failed to parse Modrinth version")?;

        let mc_version = ver.game_versions.first().cloned().unwrap_or_default();
        let loader = ver.loaders.first().cloned().unwrap_or_default();

        let mut info_files = Vec::new();
        for f in &ver.files {
            let bytes = self.http.get(&f.url).send().await?.bytes().await?;
            let s3_key = format!("modpacks/{}/files/{}", template_name, f.filename);
            self.s3.upload_bytes(&s3_key, bytes.to_vec()).await?;
            info_files.push(ModpackFile {
                path: f.filename.clone(),
                url: f.url.clone(),
                size: f.size,
            });
        }

        Ok(ModpackInfo {
            name: ver.name,
            version: ver.version_number,
            minecraft_version: mc_version,
            loader,
            files: info_files,
        })
    }

    pub async fn download_curseforge(
        &self,
        template_name: &str,
        project_id: u32,
        file_id: u32,
        api_key: &str,
    ) -> anyhow::Result<ModpackInfo> {
        #[derive(Deserialize)]
        struct CfResponse {
            data: CfFile,
        }
        #[derive(Deserialize)]
        struct CfFile {
            #[serde(rename = "fileName")]
            file_name: String,
            #[serde(rename = "downloadUrl")]
            download_url: Option<String>,
            #[serde(rename = "fileLength")]
            file_length: u64,
            #[serde(rename = "displayName")]
            display_name: String,
            #[serde(rename = "gameVersions")]
            game_versions: Vec<String>,
        }

        let url = format!(
            "https://api.curseforge.com/v1/mods/{}/files/{}",
            project_id, file_id
        );
        let resp: CfResponse = self
            .http
            .get(&url)
            .header("x-api-key", api_key)
            .send()
            .await?
            .json()
            .await
            .context("CurseForge API error")?;

        let file = resp.data;
        let dl_url = file
            .download_url
            .context("CurseForge file has no download URL — CDN key required")?;

        let bytes = self.http.get(&dl_url).send().await?.bytes().await?;
        let s3_key = format!("modpacks/{}/files/{}", template_name, file.file_name);
        self.s3.upload_bytes(&s3_key, bytes.to_vec()).await?;

        let mc_version = file
            .game_versions
            .iter()
            .find(|v| v.starts_with("1."))
            .cloned()
            .unwrap_or_default();

        Ok(ModpackInfo {
            name: file.display_name,
            version: file_id.to_string(),
            minecraft_version: mc_version,
            loader: "forge".into(),
            files: vec![ModpackFile {
                path: file.file_name,
                url: dl_url,
                size: file.file_length,
            }],
        })
    }
}
