/**
 * The question a later layout regression check will ask of a mounted page: what reaches past the
 * right edge of the window (round 2's rule: no sideways scroll at 960×640, every UI scale). A pure
 * function over boxes, so it is tested without a layout engine and run by the harness in a browser.
 */
export interface Box {
  label: string;
  right: number;
  width: number;
  /** Inside a container that scrolls sideways on purpose, such as a wide table's scroller. */
  inScroller: boolean;
}

/** Labels of the drawn boxes that pass `viewportWidth` by more than a sub-pixel rounding. */
export function boxesPastEdge(boxes: Box[], viewportWidth: number): string[] {
  return boxes
    .filter((box) => box.width > 0 && !box.inScroller && box.right > viewportWidth + 1)
    .map((box) => box.label);
}

/** Tag, role and a few words of its name or text, enough to find it on the page. */
function describeElement(element: Element): string {
  const role = element.getAttribute("role");
  const name = (element.getAttribute("aria-label") ?? element.textContent ?? "")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, 40);
  return `${element.tagName.toLowerCase()}${role ? `[role=${role}]` : ""}${name ? ` "${name}"` : ""}`;
}

function scrollsSideways(element: Element): boolean {
  const overflowX = getComputedStyle(element).overflowX;
  return overflowX === "auto" || overflowX === "scroll";
}

/**
 * The outermost elements under `root` that reach past the window's right edge, outside sideways
 * scrollers: an element inside one already reported is left out, so one overflow is one line.
 */
export function elementsPastEdge(root: Element, viewportWidth = window.innerWidth): string[] {
  const elements = [...root.querySelectorAll("*")];
  const boxes = elements.map((element, index) => {
    const rect = element.getBoundingClientRect();
    let inScroller = false;
    for (let parent = element.parentElement; parent && !inScroller; parent = parent.parentElement) {
      inScroller = scrollsSideways(parent);
    }
    return {
      label: String(index),
      right: rect.right,
      width: rect.width,
      inScroller,
    };
  });
  const past = new Set(boxesPastEdge(boxes, viewportWidth).map((index) => elements[Number(index)]));
  return [...past]
    .filter((element) => ![...past].some((other) => other !== element && other.contains(element)))
    .map(describeElement);
}
