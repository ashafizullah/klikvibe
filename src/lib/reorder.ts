// Dragging tabs sideways to put them in another order: the project tabs in
// the title bar and the file tabs above the editor. `use:reorder` goes on the
// strip, `data-reorder` on each tab (its direct children), and
// `data-no-reorder` on parts that only click (a close button).

/** Moves `list[from]` to position `to` (counted without it). */
export function moveItem<T>(list: T[], from: number, to: number) {
  const [item] = list.splice(from, 1);
  list.splice(to, 0, item);
}

/** Pixels the pointer moves before a press becomes a drag (a smaller move is still a click). */
const THRESHOLD = 5;

export function reorder(strip: HTMLElement, onmove: (from: number, to: number) => void) {
  let move = onmove;
  let drag: { el: HTMLElement; from: number; x0: number; id: number; moved: boolean } | null = null;
  const items = () => [...strip.children].filter((c): c is HTMLElement => c instanceof HTMLElement && c.dataset.reorder !== undefined);

  function down(e: PointerEvent) {
    const target = e.target as HTMLElement;
    const el = target.closest<HTMLElement>("[data-reorder]");
    if (e.button !== 0 || !el || el.parentElement !== strip || target.closest("[data-no-reorder]")) return;
    drag = { el, from: items().indexOf(el), x0: e.clientX, id: e.pointerId, moved: false };
  }

  function pointermove(e: PointerEvent) {
    if (!drag || e.pointerId !== drag.id) return;
    const dx = e.clientX - drag.x0;
    if (!drag.moved) {
      if (Math.abs(dx) < THRESHOLD) return;
      drag.moved = true;
      strip.setPointerCapture(e.pointerId);
      drag.el.classList.add("dragging");
    }
    drag.el.style.transform = `translateX(${dx}px)`;
  }

  function end(e: PointerEvent, drop: boolean) {
    if (!drag || e.pointerId !== drag.id) return;
    const d = drag;
    drag = null;
    if (!d.moved) return;
    d.el.classList.remove("dragging");
    d.el.style.transform = "";
    if (strip.hasPointerCapture(e.pointerId)) strip.releasePointerCapture(e.pointerId);
    if (drop) {
      // The new place: after every other tab whose middle the pointer passed.
      const to = items().filter((it, i) => {
        if (i === d.from) return false;
        const r = it.getBoundingClientRect();
        return e.clientX > r.left + r.width / 2;
      }).length;
      if (to !== d.from) move(d.from, to);
    }
    // The click that ends a drag doesn't also select the tab.
    const swallow = (c: Event) => {
      c.stopPropagation();
      c.preventDefault();
    };
    window.addEventListener("click", swallow, { capture: true, once: true });
    setTimeout(() => window.removeEventListener("click", swallow, { capture: true }));
  }

  const up = (e: PointerEvent) => end(e, true);
  const cancel = (e: PointerEvent) => end(e, false);
  strip.addEventListener("pointerdown", down);
  strip.addEventListener("pointermove", pointermove);
  strip.addEventListener("pointerup", up);
  strip.addEventListener("pointercancel", cancel);
  return {
    update(next: (from: number, to: number) => void) {
      move = next;
    },
    destroy() {
      strip.removeEventListener("pointerdown", down);
      strip.removeEventListener("pointermove", pointermove);
      strip.removeEventListener("pointerup", up);
      strip.removeEventListener("pointercancel", cancel);
    },
  };
}
