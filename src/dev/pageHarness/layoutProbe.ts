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

function describeElement(element: Element): string {
  const id = element.id ? `#${element.id}` : "";
  const role = element.getAttribute("role");
  return `${element.tagName.toLowerCase()}${id}${role ? `[role=${role}]` : ""}`;
}

function scrollsSideways(element: Element): boolean {
  const overflowX = getComputedStyle(element).overflowX;
  return overflowX === "auto" || overflowX === "scroll";
}

/** Every element under `root` that reaches past the window's right edge, outside sideways scrollers. */
export function elementsPastEdge(root: Element, viewportWidth = window.innerWidth): string[] {
  const boxes = [...root.querySelectorAll("*")].map((element) => {
    const rect = element.getBoundingClientRect();
    let inScroller = false;
    for (let parent = element.parentElement; parent && !inScroller; parent = parent.parentElement) {
      inScroller = scrollsSideways(parent);
    }
    return { label: describeElement(element), right: rect.right, width: rect.width, inScroller };
  });
  return boxesPastEdge(boxes, viewportWidth);
}
