// Dropping paths on the window: files and folders from Finder or Explorer (the
// native drop, which knows their paths) and entries dragged from the file tree.
// Over a terminal the paths are typed there; elsewhere a file opens in the
// editor, editable as a project file is, and a folder opens as a project.
//
// The native drop takes the HTML5 drag events, so dragging from the tree is
// done with pointer events here.
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { errorText, ipc } from "./ipc";
import { baseName } from "./paths";
import { currentProject, openProject } from "./project.svelte";
import { isWindows, toast } from "./ui.svelte";
import { t } from "./i18n/index.svelte";

/** Elements that take dropped paths (a terminal) carry this attribute and get a PATHS_EVENT (detail: the paths). */
export const DROP_ATTR = "data-path-drop";
export const PATHS_EVENT = "klik-paths";
/** The class on the element the paths would drop on. */
const HOVER = "drop-target";

let hovered: Element | null = null;

function hover(x: number, y: number): Element | null {
  const el = document.elementFromPoint(x, y)?.closest(`[${DROP_ATTR}]`) ?? null;
  if (el !== hovered) {
    hovered?.classList.remove(HOVER);
    el?.classList.add(HOVER);
    hovered = el;
  }
  return el;
}

function unhover() {
  hovered?.classList.remove(HOVER);
  hovered = null;
}

/** Gives `paths` to what takes them at (x, y); false when nothing there does. */
function dropOn(x: number, y: number, paths: string[]): boolean {
  const el = hover(x, y);
  unhover();
  el?.dispatchEvent(new CustomEvent(PATHS_EVENT, { detail: paths }));
  return !!el;
}

async function openDropped(path: string) {
  try {
    const dropped = await ipc.openDropped(path);
    if (dropped.kind === "folder") return await openProject(dropped.project);
    const project = currentProject();
    if (!project) return toast(t("drop.openProjectFirst"), "error");
    await project.openFile(dropped.path);
  } catch (e) {
    toast(errorText(e), "error");
  }
}

/** Listens for files dropped from outside the app. Returns the unlisten promise. */
export function startFileDrop() {
  // macOS and Linux report the position in CSS pixels, Windows in device pixels.
  const scale = isWindows ? devicePixelRatio : 1;
  // Never let the page itself take a dropped file: WebKit would open it in place of the app.
  const stop = (e: DragEvent) => e.preventDefault();
  window.addEventListener("dragover", stop);
  window.addEventListener("drop", stop);
  return getCurrentWebview().onDragDropEvent(async ({ payload }) => {
    if (payload.type === "leave") return unhover();
    const x = payload.position.x / scale;
    const y = payload.position.y / scale;
    if (payload.type !== "drop") return void hover(x, y);
    if (dropOn(x, y, payload.paths)) return;
    for (const path of payload.paths) await openDropped(path);
  });
}

/** Pixels the pointer moves before a press becomes a drag (a smaller move is still a click). */
const THRESHOLD = 5;

/** `use:dragPath={path}` on a tree entry: drag it onto a terminal to type its path there. */
export function dragPath(node: HTMLElement, path: string) {
  let current = path;
  let start: { x: number; y: number; id: number } | null = null;
  let ghost: HTMLElement | null = null;

  function down(e: PointerEvent) {
    if (e.button !== 0) return;
    start = { x: e.clientX, y: e.clientY, id: e.pointerId };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", finish);
  }

  function move(e: PointerEvent) {
    if (!start || e.pointerId !== start.id) return;
    if (!ghost) {
      if (Math.hypot(e.clientX - start.x, e.clientY - start.y) < THRESHOLD) return;
      ghost = document.createElement("div");
      ghost.className = "path-ghost";
      ghost.textContent = baseName(current);
      document.body.append(ghost);
      document.body.classList.add("dragging-path");
    }
    ghost.style.transform = `translate(${e.clientX + 12}px, ${e.clientY + 10}px)`;
    hover(e.clientX, e.clientY);
  }

  function finish() {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
    window.removeEventListener("pointercancel", finish);
    ghost?.remove();
    ghost = null;
    start = null;
    document.body.classList.remove("dragging-path");
    unhover();
  }

  function up(e: PointerEvent) {
    if (!start || e.pointerId !== start.id) return;
    const dragged = !!ghost;
    finish();
    if (!dragged) return;
    // Let go on the entry itself: the click that follows doesn't open it.
    if (node.contains(e.target as Node)) window.addEventListener("click", (c) => c.stopPropagation(), { capture: true, once: true });
    dropOn(e.clientX, e.clientY, [current]);
  }

  node.addEventListener("pointerdown", down);
  return {
    update(p: string) {
      current = p;
    },
    destroy() {
      node.removeEventListener("pointerdown", down);
      finish();
    },
  };
}
