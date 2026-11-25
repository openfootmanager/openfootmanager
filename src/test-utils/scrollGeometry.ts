import { vi } from "vitest";

interface GeometryOptions {
  /** The scroller's visible height in px. */
  viewportHeight: number;
  /** How far below the top of the scroller's content the list starts (a toolbar above it), in px. */
  listOffset?: number;
}

/**
 * Gives jsdom the one piece of layout a scrolling list needs: where the list sits in its scroller.
 *
 * jsdom lays nothing out, so every `getBoundingClientRect()` is all zeros and every `clientHeight` is
 * 0. For a list in a scroller that means the list's top never moves as the scroller scrolls, and a
 * window computed from it is wrong in a way no real browser shows (the list never leaves its first
 * rows). This restores the real relationship for elements marked `data-scroll-container` (the
 * scroller: top 0, `clientHeight` as given) and `role="list"` (the list: its top is `listOffset`
 * minus the scroller's `scrollTop`, as in a browser); every other element keeps jsdom's zeros.
 *
 * Returns a function that undoes it. Call it in the test, and again in `afterEach`.
 */
export function installScrollGeometry({
  viewportHeight,
  listOffset = 0,
}: GeometryOptions): () => void {
  const scrollerOf = (el: Element) => el.closest<HTMLElement>("[data-scroll-container]");

  const rect = vi
    .spyOn(HTMLElement.prototype, "getBoundingClientRect")
    .mockImplementation(function (this: HTMLElement) {
      let top = 0;
      if (this.getAttribute("role") === "list") {
        top = listOffset - (scrollerOf(this)?.scrollTop ?? 0);
      }
      return {
        x: 0,
        y: top,
        top,
        left: 0,
        right: 0,
        bottom: top,
        width: 0,
        height: 0,
        toJSON: () => ({}),
      } as DOMRect;
    });

  const original = Object.getOwnPropertyDescriptor(HTMLElement.prototype, "clientHeight");
  Object.defineProperty(HTMLElement.prototype, "clientHeight", {
    configurable: true,
    get(this: HTMLElement) {
      return this.hasAttribute("data-scroll-container") ? viewportHeight : 0;
    },
  });

  return () => {
    rect.mockRestore();
    if (original) Object.defineProperty(HTMLElement.prototype, "clientHeight", original);
    else delete (HTMLElement.prototype as unknown as Record<string, unknown>).clientHeight;
  };
}
