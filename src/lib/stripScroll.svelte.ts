// The ‹ › buttons beside a tab strip that has more tabs than fit: the project
// tabs, the terminal tabs and the file tabs. Create one while a component
// starts, bind its `el` to the strip, and show `ScrollButton`s when `overflow`.

export class StripScroll {
  el = $state<HTMLElement>();
  /** The tabs are wider than the strip. */
  overflow = $state(false);
  /** There are tabs hidden to the left / to the right. */
  canLeft = $state(false);
  canRight = $state(false);

  constructor() {
    $effect(() => {
      const el = this.el;
      if (!el) return;
      const update = () => {
        const max = el.scrollWidth - el.clientWidth;
        this.overflow = max > 1;
        this.canLeft = el.scrollLeft > 1;
        this.canRight = el.scrollLeft < max - 1;
      };
      update();
      // The strip resizes with the window; its width needed changes as tabs open, close or rename.
      const size = new ResizeObserver(update);
      size.observe(el);
      const content = new MutationObserver(update);
      content.observe(el, { childList: true, subtree: true, characterData: true });
      el.addEventListener("scroll", update, { passive: true });
      return () => {
        size.disconnect();
        content.disconnect();
        el.removeEventListener("scroll", update);
      };
    });
  }

  /** Moves most of a strip's width to the left (-1) or right (1). */
  by(dir: -1 | 1) {
    const el = this.el;
    if (el) el.scrollBy({ left: dir * Math.max(80, el.clientWidth * 0.8), behavior: "smooth" });
  }
}
