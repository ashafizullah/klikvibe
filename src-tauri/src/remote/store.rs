//! What Remote remembers between runs, in `remote.json` in the app data folder
//! (only the user can read it: it holds each phone's key and the computer's
//! token for the KlikVibe server).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    /// Remote is on: phones on the same network can connect.
    pub enabled: bool,
    /// The LAN port last used, tried first next time so paired phones find it again.
    pub port: Option<u16>,
    /// This computer on the KlikVibe server, once a phone has added it to an account.
    pub desktop_id: Option<String>,
    pub device_token: Option<String>,
    /// The account's email (or name), shown in the Remote dialog.
    pub account: Option<String>,
    pub pro: bool,
    pub phones: Vec<Phone>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Phone {
    /// Its id on the KlikVibe server (a UUID).
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub platform: Option<String>,
    /// The Noise pre-shared key from its pairing QR code, base64url.
    pub psk: String,
}

pub fn path(dir: &Path) -> PathBuf {
    dir.join("remote.json")
}

pub fn load(dir: &Path) -> Config {
    std::fs::read(path(dir)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

pub fn save(dir: &Path, config: &Config) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let file = path(dir);
    let tmp = file.with_extension("json.tmp");
    let _ = std::fs::remove_file(&tmp);
    // Created readable by the user only from the first byte (it holds the phones' keys).
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    use std::io::Write;
    opts.open(&tmp)?.write_all(&serde_json::to_vec_pretty(config)?)?;
    std::fs::rename(tmp, file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_loads() {
        let dir = std::env::temp_dir().join(format!("klikvibe-remote-{}", uuid::Uuid::new_v4()));
        assert_eq!(load(&dir), Config::default());
        let c = Config {
            enabled: true,
            port: Some(47615),
            phones: vec![Phone { id: "p".into(), name: "iPhone".into(), platform: Some("ios".into()), psk: "k".into() }],
            ..Config::default()
        };
        save(&dir, &c).unwrap();
        assert_eq!(load(&dir), c);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(path(&dir)).unwrap().permissions().mode() & 0o777, 0o600);
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}
