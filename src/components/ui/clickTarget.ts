import type { MouseEvent } from "react";

/** Native controls, and elements that take a control's ARIA role. */
const CONTROL = [
  "button",
  "a[href]",
  "input",
  "select",
  "textarea",
  "summary",
  "[contenteditable='true']",
  ...[
    "button",
    "link",
    "checkbox",
    "radio",
    "switch",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "option",
    "tab",
    "slider",
    "spinbutton",
    "combobox",
    "textbox",
  ].map((role) => `[role='${role}']`),
].join(", ");

/**
 * Whether a click landed on one of a row's or card's own controls, native or by ARIA role. Such a
 * click is that control's, not a request to open the whole row or card, which DataTable and
 * EntityCard offer only as a pointer convenience beside their name button. Only controls inside
 * the row or card count: a role on an ancestor of it doesn't swallow its own opening click.
 */
export function isOnControl(event: MouseEvent<HTMLElement>): boolean {
  const control = (event.target as Element).closest(CONTROL);
  return (
    control !== null && control !== event.currentTarget && event.currentTarget.contains(control)
  );
}
