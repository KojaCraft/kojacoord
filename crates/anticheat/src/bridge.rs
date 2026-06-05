pub struct BridgeClient {
    http: reqwest::Client,
    endpoint: String,
}

impl BridgeClient {
    pub fn new(endpoint: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            endpoint,
        }
    }

    pub async fn report(&self, v: &crate::violation::Violation) -> Result<(), reqwest::Error> {
        self.http
            .post(format!("{}/violations", self.endpoint))
            .json(v)
            .send()
            .await?;
        Ok(())
    }
}
