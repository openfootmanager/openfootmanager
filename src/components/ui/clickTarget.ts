import type { MouseEvent } from "react";

/**
 * Whether a click landed on one of a row's or card's own controls. Such a click is that control's,
 * not a request to open the whole row or card, which DataTable and EntityCard offer only as a
 * pointer convenience beside their name button.
 */
export function isOnControl(event: MouseEvent<HTMLElement>): boolean {
  return (event.target as Element).closest("button, a, input, select, textarea") !== null;
}
