/**
 * Where a right-click's menu opens: at the pointer. The Menu key and Shift+F10 send their
 * right-click at 0,0, the viewport's corner, so a menu opened from the keyboard opens under the
 * anchor (the row, or the control that has focus) instead.
 */
export function contextMenuPoint(
  event: { clientX: number; clientY: number },
  anchor: Element,
): { x: number; y: number } {
  if (event.clientX !== 0 || event.clientY !== 0) return { x: event.clientX, y: event.clientY };
  const rect = anchor.getBoundingClientRect();
  return { x: rect.left, y: rect.bottom };
}
