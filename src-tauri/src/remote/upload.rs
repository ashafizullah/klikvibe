//! `file.put`: an image from the phone, saved on this computer so the phone can
//! hand its path to an agent. It comes in chunks (one Noise message holds at most
//! 64 KiB) and lands in a private folder under the system's temporary folder.

use base64::Engine;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// The largest image the phone may send.
pub const MAX_SIZE: u64 = 20 * 1024 * 1024;
/// Raw bytes per chunk (base64 makes it a third larger; it still fits a Noise message).
pub const MAX_CHUNK: usize = 36 * 1024;
/// Uploads in progress at once, per phone connection.
const MAX_OPEN: usize = 2;
/// Older uploads are deleted when a new one starts: by then the agent has read them.
const KEEP: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "webp", "heic"];
const MAX_NAME: usize = 80;

pub fn default_dir() -> PathBuf {
    std::env::temp_dir().join("klikvibe-remote")
}

/// One `file.put` message.
pub struct Chunk {
    pub up: String,
    pub name: String,
    pub size: u64,
    pub offset: u64,
    /// Base64.
    pub data: String,
    pub last: bool,
}

struct Upload {
    path: PathBuf,
    size: u64,
    written: u64,
    /// Out while a write runs off the async workers.
    file: Option<File>,
}

/// One session's uploads in progress. What is left of them when the session ends is deleted.
pub struct Uploads {
    dir: PathBuf,
    open: HashMap<String, Upload>,
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> io::Result<T> + Send + 'static) -> Result<T, String> {
    tokio::task::spawn_blocking(f).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())
}

impl Uploads {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir, open: HashMap::new() }
    }

    /// Takes one chunk; the file's path once the last one is written. On an error the partial
    /// file is deleted, and the phone starts over.
    pub async fn put(&mut self, c: Chunk) -> Result<Option<PathBuf>, String> {
        let r = self.write(&c).await;
        if r.is_err()
            && let Some(u) = self.open.remove(&c.up)
        {
            let _ = blocking(move || {
                drop(u.file);
                std::fs::remove_file(&u.path)
            })
            .await;
        }
        r
    }

    async fn write(&mut self, c: &Chunk) -> Result<Option<PathBuf>, String> {
        if !(8..=40).contains(&c.up.len()) || !c.up.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-') {
            return Err("Invalid upload id".into());
        }
        let data = base64::engine::general_purpose::STANDARD.decode(&c.data).map_err(|_| "Invalid data".to_string())?;
        if data.len() > MAX_CHUNK {
            return Err("Chunk too large".into());
        }
        if !self.open.contains_key(&c.up) {
            if c.offset != 0 {
                return Err("Unknown upload".into());
            }
            if self.open.len() >= MAX_OPEN {
                return Err("Too many uploads at once".into());
            }
            let name = file_name(&c.name)?;
            if c.size == 0 {
                return Err("The file is empty".into());
            }
            if c.size > MAX_SIZE {
                return Err("The file is too large (at most 20 MB)".into());
            }
            let dir = self.dir.clone();
            let (path, file) = blocking(move || create(&dir, &name)).await?;
            self.open.insert(c.up.clone(), Upload { path, size: c.size, written: 0, file: Some(file) });
        }
        let u = self.open.get_mut(&c.up).expect("just checked");
        if c.size != u.size || c.offset != u.written {
            return Err("Chunks out of order".into());
        }
        let end = u.written + data.len() as u64;
        if end > u.size {
            return Err("More data than the file's size".into());
        }
        if c.last && end != u.size {
            return Err("The file is incomplete".into());
        }
        let mut file = u.file.take().ok_or("Busy")?;
        let file = blocking(move || {
            file.write_all(&data)?;
            Ok(file)
        })
        .await?;
        u.file = Some(file);
        u.written = end;
        if !c.last {
            return Ok(None);
        }
        let u = self.open.remove(&c.up).expect("just written");
        let file = u.file;
        blocking(move || file.map_or(Ok(()), |f| f.sync_all())).await?;
        Ok(Some(u.path))
    }
}

impl Drop for Uploads {
    fn drop(&mut self) {
        let paths: Vec<PathBuf> = self.open.drain().map(|(_, u)| u.path).collect();
        if paths.is_empty() {
            return;
        }
        let remove = move || {
            for p in paths {
                let _ = std::fs::remove_file(p);
            }
        };
        match tokio::runtime::Handle::try_current() {
            Ok(rt) => drop(rt.spawn_blocking(remove)),
            Err(_) => remove(),
        }
    }
}

/// The name the file is saved under: an image extension (lowercased), and only characters
/// that are safe in a path and in a shell (the phone may type the path into a terminal).
fn file_name(name: &str) -> Result<String, String> {
    let not_image = || "Only images can be sent (PNG, JPEG, GIF, WebP, HEIC)".to_string();
    let (stem, ext) = name.rsplit_once('.').ok_or_else(not_image)?;
    let ext = ext.to_ascii_lowercase();
    if !EXTENSIONS.contains(&ext.as_str()) {
        return Err(not_image());
    }
    let stem: String = stem.chars().map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '_' }).collect();
    let stem: String = stem.trim_start_matches('.').chars().take(MAX_NAME - 1 - ext.len()).collect();
    Ok(format!("{}.{ext}", if stem.is_empty() { "image" } else { &stem }))
}

/// A new file in the upload folder, readable by this user only.
fn create(dir: &Path, name: &str) -> io::Result<(PathBuf, File)> {
    prepare(dir)?;
    sweep(dir);
    let mut random = [0u8; 4];
    getrandom::fill(&mut random).map_err(io::Error::other)?;
    let hex: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let path = dir.join(format!("{hex}-{name}"));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let file = options.open(&path)?;
    Ok((path, file))
}

/// Makes the folder (private). Under a shared /tmp (Linux) another user could have made it
/// first: it must be a real folder, and only its owner can make it private again.
fn prepare(dir: &Path) -> io::Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder.create(dir)?;
    if !std::fs::symlink_metadata(dir)?.is_dir() {
        return Err(io::Error::other("The upload folder is not a folder"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// Deletes uploads older than a week.
fn sweep(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let now = SystemTime::now();
    for e in entries.flatten() {
        let Ok(meta) = e.metadata() else { continue };
        let old = meta.modified().ok().and_then(|m| now.duration_since(m).ok()).is_some_and(|age| age > KEEP);
        if meta.is_file() && old {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(file_name("photo.JPG").unwrap(), "photo.jpg");
        assert_eq!(file_name("../../etc/x.png").unwrap(), "_.._etc_x.png");
        assert_eq!(file_name(".hidden.heic").unwrap(), "hidden.heic");
        assert_eq!(file_name("写真 1.webp").unwrap(), "___1.webp");
        assert_eq!(file_name(".png").unwrap(), "image.png");
        assert_eq!(file_name(&format!("{}.jpeg", "a".repeat(200))).unwrap().len(), MAX_NAME);
        assert!(file_name("notes.txt").is_err());
        assert!(file_name("x.png/evil").is_err());
        assert!(file_name("png").is_err());
    }
}
