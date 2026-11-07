//! The KlikVibe server (klikvibe.ashafizullah.com): adding a phone with a QR
//! code, the heartbeat that keeps the account's phone list in sync, and push
//! notifications. The server never sees a phone's key or a terminal.

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

pub const SERVER: &str = "https://klikvibe.ashafizullah.com";

/// The server's address; `KLIKVIBE_SERVER` points a development build at a local one.
pub fn base() -> String {
    std::env::var("KLIKVIBE_SERVER").unwrap_or_else(|_| SERVER.to_string())
}

#[derive(Debug, PartialEq)]
pub enum CloudError {
    /// The computer was removed from the account (or the account deleted).
    Unlinked,
    /// The pairing code expired.
    Expired,
    /// Anything else: offline, the server down, refused.
    Other(String),
}

impl std::fmt::Display for CloudError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CloudError::Unlinked => f.write_str("This computer is no longer linked to an account"),
            CloudError::Expired => f.write_str("The code expired"),
            CloudError::Other(m) => f.write_str(m),
        }
    }
}

#[derive(Deserialize)]
struct Envelope<T> {
    data: Option<T>,
    error: Option<ErrorBody>,
}

#[derive(Deserialize)]
struct ErrorBody {
    code: String,
    message: String,
}

async fn call<T: DeserializeOwned>(
    method: reqwest::Method,
    path: &str,
    auth: Option<String>,
    body: Option<Value>,
) -> Result<T, CloudError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent(concat!("KlikVibe/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| CloudError::Other(e.to_string()))?;
    let mut req = client.request(method, format!("{}{path}", base()));
    if let Some(a) = auth {
        req = req.header("authorization", a);
    }
    if let Some(b) = body {
        req = req.json(&b);
    }
    let res = req.send().await.map_err(|e| CloudError::Other(e.to_string()))?;
    let env: Envelope<T> = res.json().await.map_err(|e| CloudError::Other(e.to_string()))?;
    match (env.data, env.error) {
        (_, Some(e)) if e.code == "DEVICE_UNLINKED" => Err(CloudError::Unlinked),
        (_, Some(e)) if e.code == "PAIR_EXPIRED" => Err(CloudError::Expired),
        (_, Some(e)) => Err(CloudError::Other(server_message(&e.code, e.message))),
        (Some(d), None) => Ok(d),
        (None, None) => Err(CloudError::Other("Empty answer".into())),
    }
}

/// The server writes its messages in Indonesian; the ones the desktop can meet are said in English
/// here, by code (others keep the server's words).
fn server_message(code: &str, message: String) -> String {
    match code {
        "RATE_LIMITED" => "Too many attempts. Try again in a minute.".into(),
        "VALIDATION_ERROR" => "The KlikVibe server did not accept the request.".into(),
        "PRO_REQUIRED" => "This needs KlikVibe Pro.".into(),
        "INTERNAL" => "Something went wrong on the KlikVibe server. Try again.".into(),
        _ => message,
    }
}

/// What the server wants to know about this computer.
pub fn info(name: &str, version: &str) -> Value {
    let os = if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(windows) {
        "windows"
    } else {
        "linux"
    };
    json!({ "name": name, "os": os, "appVersion": version })
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PairStart {
    pub code: String,
    pub poll_token: String,
    pub expires_at: String,
}

pub async fn pair_start(info: Value, device_token: Option<&str>) -> Result<PairStart, CloudError> {
    call(reqwest::Method::POST, "/v1/pair/start", device_token.map(|t| format!("Device {t}")), Some(info)).await
}

#[derive(Deserialize, Debug, Clone)]
pub struct Named {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub platform: Option<String>,
}

#[derive(Deserialize, Debug)]
// `rename_all_fields`, not `rename_all`: on an enum, `rename_all` renames only the variants, so
// `deviceToken` was never read and every new computer stayed without its token.
#[serde(rename_all_fields = "camelCase", tag = "status")]
pub enum PairStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "linked")]
    Linked {
        desktop: Named,
        phone: Named,
        device_token: Option<String>,
        /// The account the phone is signed in to, shown when the user is asked to allow it.
        #[serde(default)]
        account: Option<String>,
    },
}

pub async fn pair_status(poll_token: &str) -> Result<PairStatus, CloudError> {
    call(reqwest::Method::GET, "/v1/pair/status", Some(format!("Pair {poll_token}")), None).await
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Heartbeat {
    pub desktop: Named,
    pub account: Option<String>,
    pub pro: bool,
    pub phones: Vec<Named>,
}

pub async fn heartbeat(device_token: &str, info: Value) -> Result<Heartbeat, CloudError> {
    call(reqwest::Method::POST, "/v1/desktop/heartbeat", Some(format!("Device {device_token}")), Some(info)).await
}

/// A push to the account's phones. Its body stays empty: what the agent said never leaves the computer.
pub async fn notify(device_token: &str, title: &str, term: &str, agent: Option<&str>, kind: Option<&str>) -> Result<Value, CloudError> {
    call(reqwest::Method::POST, "/v1/desktop/notify", Some(format!("Device {device_token}")), Some(notice(title, term, agent, kind))).await
}

/// The push's JSON: `agent` only when the terminal runs one, `kind` (what happened, never what the
/// agent said) when known. An older server drops the keys it doesn't know.
fn notice(title: &str, term: &str, agent: Option<&str>, kind: Option<&str>) -> Value {
    let mut v = json!({ "title": title, "body": "", "term": term });
    if let Some(a) = agent {
        v["agent"] = json!(a);
    }
    if let Some(k) = kind {
        v["kind"] = json!(k);
    }
    v
}

pub async fn unlink(device_token: &str) -> Result<Value, CloudError> {
    call(reqwest::Method::DELETE, "/v1/desktop", Some(format!("Device {device_token}")), None).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_push_names_the_terminal_only() {
        assert_eq!(
            notice("Claude · app", "t1", Some("claude"), Some("waiting")),
            json!({ "title": "Claude · app", "body": "", "term": "t1", "agent": "claude", "kind": "waiting" })
        );
        assert_eq!(notice("Terminal · app", "t2", None, None), json!({ "title": "Terminal · app", "body": "", "term": "t2" }));
    }

    #[test]
    fn statuses_parse() {
        let p: PairStatus = serde_json::from_value(json!({ "status": "pending", "expiresAt": "x" })).unwrap();
        assert!(matches!(p, PairStatus::Pending));
        let l: PairStatus = serde_json::from_value(json!({
            "status": "linked",
            "desktop": { "id": "d", "name": "Mac" },
            "phone": { "id": "p", "name": "iPhone", "platform": "ios" },
            "deviceToken": "tok",
            "account": "a***@example.com",
        }))
        .unwrap();
        assert!(matches!(&l, PairStatus::Linked { device_token: Some(t), phone, account: Some(a), .. }
            if t == "tok" && phone.name == "iPhone" && a == "a***@example.com"));
        // A computer that already had a token gets none.
        let again: PairStatus = serde_json::from_value(json!({
            "status": "linked",
            "desktop": { "id": "d", "name": "Mac" },
            "phone": { "id": "p", "name": "iPhone" },
            "deviceToken": null,
        }))
        .unwrap();
        assert!(matches!(again, PairStatus::Linked { device_token: None, .. }));
    }
}
