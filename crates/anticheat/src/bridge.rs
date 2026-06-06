pub struct BridgeClient {
    http: reqwest::Client,
    endpoint: String,
    token: String,
}

impl BridgeClient {
    pub fn new(endpoint: String, token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            endpoint,
            token,
        }
    }

    pub async fn report(
        &self,
        username: &str,
        v: &crate::violation::Violation,
    ) -> Result<(), reqwest::Error> {
        let payload = serde_json::json!({
            "username": username,
            "source": "SERVERAC",
            "detection_type": v.check_name,
            "details": Some(format!(
                "Category: {}, Value: {:.2}, Threshold: {:.2}",
                v.check_category.human_name(),
                v.value,
                v.threshold
            )),
            "ban": false,
        });

        self.http
            .post(format!("{}/v1/anticheat/report", self.endpoint))
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&payload)
            .send()
            .await?;
        Ok(())
    }
}
