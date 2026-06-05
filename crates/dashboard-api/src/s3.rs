use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::Client;

pub struct S3Client {
    inner: Client,
    bucket: String,
}

#[derive(Debug, serde::Serialize)]
pub struct S3Object {
    pub key: String,
    pub size: i64,
    pub last_modified: Option<String>,
}

impl S3Client {
    pub async fn new(cfg: &crate::config::S3Config) -> anyhow::Result<Self> {
        let credentials = Credentials::new(
            &cfg.access_key_id,
            &cfg.secret_access_key,
            None,
            None,
            "hetzner-s3",
        );

        let mut builder = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_config::Region::new(cfg.region.clone()))
            .credentials_provider(credentials);

        if let Some(ep) = &cfg.endpoint_url {
            builder = builder.endpoint_url(ep);
        }

        let aws_cfg = builder.load().await;

        let s3_config = aws_sdk_s3::config::Builder::from(&aws_cfg)
            .force_path_style(true)
            .build();

        Ok(Self {
            inner: Client::from_conf(s3_config),
            bucket: cfg.bucket.clone(),
        })
    }

    pub async fn list(&self, prefix: &str) -> anyhow::Result<Vec<String>> {
        let resp = self
            .inner
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .send()
            .await?;

        Ok(resp
            .contents()
            .iter()
            .filter_map(|o| o.key().map(str::to_owned))
            .collect())
    }

    pub async fn list_detailed(&self, prefix: &str) -> anyhow::Result<Vec<S3Object>> {
        let resp = self
            .inner
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .send()
            .await?;

        Ok(resp
            .contents()
            .iter()
            .filter_map(|o| {
                o.key().map(|k| S3Object {
                    key: k.to_owned(),
                    size: o.size().unwrap_or(0),
                    last_modified: o
                        .last_modified()
                        .and_then(|t| t.fmt(aws_sdk_s3::primitives::DateTimeFormat::DateTime).ok()),
                })
            })
            .collect())
    }

    pub async fn presign_get(&self, key: &str, expires_secs: u64) -> anyhow::Result<String> {
        let cfg = aws_sdk_s3::presigning::PresigningConfig::expires_in(
            std::time::Duration::from_secs(expires_secs),
        )?;
        let req = self
            .inner
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(cfg)
            .await?;
        Ok(req.uri().to_string())
    }

    pub async fn upload_bytes(&self, key: &str, data: Vec<u8>) -> anyhow::Result<()> {
        self.inner
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(aws_sdk_s3::primitives::ByteStream::from(data))
            .send()
            .await?;
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> anyhow::Result<()> {
        self.inner
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;
        Ok(())
    }
}
