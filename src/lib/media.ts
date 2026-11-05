// Files shown as a preview instead of text. The extensions match what the
// Rust side serves over media:// (src-tauri/src/media.rs).
import { convertFileSrc } from "@tauri-apps/api/core";

export type MediaKind = "image" | "video" | "audio" | "pdf";

const KINDS: Record<string, MediaKind> = {
  png: "image", jpg: "image", jpeg: "image", gif: "image", webp: "image", avif: "image", bmp: "image", ico: "image", svg: "image",
  mp4: "video", m4v: "video", webm: "video", mov: "video", ogv: "video",
  mp3: "audio", wav: "audio", ogg: "audio", oga: "audio", m4a: "audio", aac: "audio", flac: "audio",
  pdf: "pdf",
};

export function mediaKind(path: string): MediaKind | null {
  const dot = path.lastIndexOf(".");
  return dot < 0 ? null : (KINDS[path.slice(dot + 1).toLowerCase()] ?? null);
}

/** Text files that also have a preview; they open as a preview with a way to the source. */
export const hasSource = (path: string) => path.toLowerCase().endsWith(".svg");

/** The URL the window loads the file from; `revision` changes when the file does. */
export const mediaUrl = (path: string, revision: number) => `${convertFileSrc(path, "media")}?v=${revision}`;
