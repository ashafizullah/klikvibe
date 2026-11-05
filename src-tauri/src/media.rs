//! Serves images, video, audio and PDFs from the open projects to the window
//! as `media://` URLs, so they show in place instead of as "binary". Like the
//! file commands, only files inside an open project are reachable, and only
//! the media types listed here are served.
use crate::projects::Projects;
use percent_encoding::percent_decode_str;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use tauri::http::{Request, Response, StatusCode, header};

/// Longest slice sent for one range request; a client asking for more gets this much and asks again.
const CHUNK: u64 = 4 * 1024 * 1024;
/// Largest file sent whole, without a range request.
const MAX_WHOLE: u64 = 256 * 1024 * 1024;

/// The media type of a file KlikVibe can show, by extension.
pub fn mime(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "svg" => "image/svg+xml",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "ogv" => "video/ogg",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" | "oga" => "audio/ogg",
        "m4a" => "audio/mp4",
        "aac" => "audio/aac",
        "flac" => "audio/flac",
        "pdf" => "application/pdf",
        _ => return None,
    })
}

/// The inclusive byte range a `Range: bytes=…` header asks for, within `size`.
fn range(value: &str, size: u64) -> Option<(u64, u64)> {
    let last = size.checked_sub(1)?;
    let spec = value.strip_prefix("bytes=")?.split(',').next()?.trim();
    let (a, b) = spec.split_once('-')?;
    let (start, end) = if a.is_empty() {
        // `bytes=-N`: the last N bytes.
        let n: u64 = b.parse().ok()?;
        (size.saturating_sub(n), last)
    } else {
        let start: u64 = a.parse().ok()?;
        let end: u64 = if b.is_empty() { u64::MAX } else { b.parse().ok()? };
        (start, end.min(start.saturating_add(CHUNK - 1)).min(last))
    };
    (start <= end).then_some((start, end))
}

pub fn serve(projects: &Projects, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    respond(projects, request).unwrap_or_else(|status| Response::builder().status(status).body(Vec::new()).unwrap())
}

fn respond(projects: &Projects, request: &Request<Vec<u8>>) -> Result<Response<Vec<u8>>, StatusCode> {
    let raw = request.uri().path().trim_start_matches('/');
    let path = percent_decode_str(raw).decode_utf8().map_err(|_| StatusCode::BAD_REQUEST)?;
    let file = projects.existing(&path).map_err(|_| StatusCode::FORBIDDEN)?;
    let mime = mime(&file).ok_or(StatusCode::FORBIDDEN)?;
    let mut f = std::fs::File::open(&file).map_err(|_| StatusCode::NOT_FOUND)?;
    let size = f.metadata().map_err(|_| StatusCode::NOT_FOUND)?.len();
    let io = |_| StatusCode::INTERNAL_SERVER_ERROR;
    let builder = Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::ACCEPT_RANGES, "bytes")
        // Agents rewrite files; the window asks again rather than cache.
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff");

    if let Some(value) = request.headers().get(header::RANGE).and_then(|v| v.to_str().ok()) {
        let Some((start, end)) = range(value, size) else {
            return builder
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(header::CONTENT_RANGE, format!("bytes */{size}"))
                .body(Vec::new())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
        };
        let mut body = vec![0; (end - start + 1) as usize];
        f.seek(SeekFrom::Start(start)).map_err(io)?;
        f.read_exact(&mut body).map_err(io)?;
        return builder
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{size}"))
            .header(header::CONTENT_LENGTH, body.len())
            .body(body)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    }

    if size > MAX_WHOLE {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    let mut body = Vec::with_capacity(size as usize);
    f.read_to_end(&mut body).map_err(io)?;
    builder.status(StatusCode::OK).header(header::CONTENT_LENGTH, body.len()).body(body).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges() {
        assert_eq!(range("bytes=0-99", 1000), Some((0, 99)));
        assert_eq!(range("bytes=900-", 1000), Some((900, 999)));
        assert_eq!(range("bytes=0-", 100 * CHUNK), Some((0, CHUNK - 1)));
        // A whole huge file asked for at once still comes in chunks.
        assert_eq!(range(&format!("bytes=0-{}", 100 * CHUNK - 1), 100 * CHUNK), Some((0, CHUNK - 1)));
        assert_eq!(range("bytes=-100", 1000), Some((900, 999)));
        assert_eq!(range("bytes=0-5000", 1000), Some((0, 999)));
        assert_eq!(range("bytes=1000-", 1000), None);
        assert_eq!(range("bytes=0-0", 0), None);
        assert_eq!(range("items=0-1", 1000), None);
    }

    #[test]
    fn only_media_is_served() {
        let dir = std::env::temp_dir().join(format!("klikvibe-media-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.PNG"), b"png").unwrap();
        std::fs::write(dir.join("run.sh"), b"echo").unwrap();
        let projects = Projects::new(dir.join("data"));
        let get = |name: &str, range: Option<&str>| {
            let mut req = Request::builder().uri(format!("media://localhost/{}", dir.join(name).display().to_string().replace('/', "%2F")));
            if let Some(r) = range {
                req = req.header(header::RANGE, r);
            }
            serve(&projects, &req.body(Vec::new()).unwrap())
        };
        // Nothing is reachable before the folder is opened.
        assert_eq!(get("a.PNG", None).status(), StatusCode::FORBIDDEN);
        projects.admit(&dir).unwrap();
        let ok = get("a.PNG", None);
        assert_eq!(ok.status(), StatusCode::OK);
        assert_eq!(ok.headers()[header::CONTENT_TYPE], "image/png");
        assert_eq!(ok.body(), b"png");
        let part = get("a.PNG", Some("bytes=1-"));
        assert_eq!(part.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(part.body(), b"ng");
        assert_eq!(get("run.sh", None).status(), StatusCode::FORBIDDEN);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
