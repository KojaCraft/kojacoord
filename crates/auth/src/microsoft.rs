//! Microsoft / Xbox Live authentication flow.
//!
//! Three-step token exchange: Xbox Live → XSTS → Minecraft Services,
//! ending in a bearer token that lets us pull the
//! `/minecraft/profile` endpoint and resolve the player's UUID.
//! Used when the operator wires the proxy to its own Microsoft
//! account (for hosted-by-proxy auth flows) — the per-connection
//! online-mode check still goes through `session::verify_session`
//! against `sessionserver.mojang.com`.

use serde::Deserialize;

const XBL_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_AUTH_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MC_AUTH_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

pub struct MicrosoftAuthClient {
    http: reqwest::Client,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MicrosoftGameProfile {
    pub id: String,
    pub name: String,
    pub skins: Vec<McSkin>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct McSkin {
    pub url: String,
    pub variant: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct XblResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: XblDisplayClaims,
}

#[derive(Deserialize)]
struct XblDisplayClaims {
    xui: Vec<XblXui>,
}

#[derive(Deserialize)]
struct XblXui {
    uhs: String,
}

#[derive(Deserialize)]
struct XstsResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: XblDisplayClaims,
}

#[derive(Deserialize)]
struct McTokenResponse {
    access_token: String,
}

impl MicrosoftAuthClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    pub async fn authenticate(
        &self,
        msa_access_token: &str,
    ) -> Result<MicrosoftGameProfile, anyhow::Error> {
        let xbl_token = self.authenticate_xbl(msa_access_token).await?;
        let (xsts_token, user_hash) = self.authenticate_xsts(&xbl_token).await?;
        let mc_token = self.authenticate_minecraft(&xsts_token, &user_hash).await?;
        self.fetch_profile(&mc_token).await
    }

    async fn authenticate_xbl(&self, msa_token: &str) -> Result<String, anyhow::Error> {
        let body = serde_json::json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName":   "user.auth.xboxlive.com",
                "RpsTicket":  format!("d={}", msa_token),
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType":    "JWT",
        });

        let resp: XblResponse = self
            .http
            .post(XBL_AUTH_URL)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(resp.token)
    }

    async fn authenticate_xsts(&self, xbl_token: &str) -> Result<(String, String), anyhow::Error> {
        let body = serde_json::json!({
            "Properties": {
                "SandboxId":  "RETAIL",
                "UserTokens": [xbl_token],
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType":    "JWT",
        });

        let resp: XstsResponse = self
            .http
            .post(XSTS_AUTH_URL)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let uhs = resp
            .display_claims
            .xui
            .into_iter()
            .next()
            .map(|x| x.uhs)
            .ok_or_else(|| anyhow::anyhow!("XSTS response missing user hash"))?;

        Ok((resp.token, uhs))
    }

    async fn authenticate_minecraft(
        &self,
        xsts_token: &str,
        user_hash: &str,
    ) -> Result<String, anyhow::Error> {
        let body = serde_json::json!({
            "identityToken": format!("XBL3.0 x={};{}", user_hash, xsts_token),
        });

        let resp: McTokenResponse = self
            .http
            .post(MC_AUTH_URL)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(resp.access_token)
    }

    pub async fn fetch_profile(
        &self,
        mc_token: &str,
    ) -> Result<MicrosoftGameProfile, anyhow::Error> {
        let resp: MicrosoftGameProfile = self
            .http
            .get(MC_PROFILE_URL)
            .bearer_auth(mc_token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(resp)
    }
}

impl Default for MicrosoftAuthClient {
    fn default() -> Self {
        Self::new()
    }
}
