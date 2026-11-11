import { StrictMode, useRef, useState } from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, type MockInstance, vi } from "vitest";

import ContextMenu, { type ContextMenuHandle } from "./ContextMenu";

/**
 * A menu's owner, as every consumer wires one: the trigger's aria-expanded follows the menu
 * through `onOpenChange`, and each report is recorded.
 */
function Owner({ name, reports }: { name: string; reports: boolean[] }) {
  const menu = useRef<ContextMenuHandle>(null);
  const [open, setOpen] = useState(false);
  return (
    <ContextMenu
      ref={menu}
      items={[{ label: `${name} item`, onClick: () => {} }]}
      onOpenChange={(next) => {
        reports.push(next);
        setOpen(next);
      }}
    >
      <button
        type="button"
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={(event) => {
          // A real click would also reach the window, where an open menu closes on a click.
          event.stopPropagation();
          menu.current?.open(10, 10);
        }}
      >
        {name}
      </button>
    </ContextMenu>
  );
}

describe("ContextMenu", () => {
  let consoleError: MockInstance<typeof console.error>;

  beforeEach(() => {
    consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
  });

  afterEach(() => {
    consoleError.mockRestore();
  });

  /** React's warning for a component's state set while another component renders. */
  const updatesDuringRender = () =>
    consoleError.mock.calls.filter((call) => String(call[0]).includes("Cannot update a component"));

  /**
   * Given an owner whose trigger's aria-expanded follows the menu
   * When the menu is opened and closed with Escape
   * Then aria-expanded follows it, and the owner is never updated while the menu renders.
   */
  it("tells its owner it opened and closed without updating the owner during its own render", async () => {
    const reports: boolean[] = [];
    render(<Owner name="Actions" reports={reports} />);
    const trigger = screen.getByRole("button", { name: "Actions" });

    fireEvent.click(trigger);
    expect(await screen.findByRole("menuitem", { name: "Actions item" })).toBeInTheDocument();
    expect(trigger).toHaveAttribute("aria-expanded", "true");

    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
    await waitFor(() => expect(screen.queryByRole("menu")).toBeNull());
    expect(trigger).toHaveAttribute("aria-expanded", "false");

    expect(reports).toEqual([true, false]);
    expect(updatesDuringRender()).toEqual([]);
  });

  /**
   * Given the same owner under StrictMode, which runs a state updater twice to expose side effects
   * When the menu is opened and closed
   * Then the owner hears each change once, and is never updated while the menu renders.
   */
  it("reports each change once under StrictMode", async () => {
    const reports: boolean[] = [];
    render(
      <StrictMode>
        <Owner name="Actions" reports={reports} />
      </StrictMode>,
    );
    const trigger = screen.getByRole("button", { name: "Actions" });

    fireEvent.click(trigger);
    expect(await screen.findByRole("menuitem", { name: "Actions item" })).toBeInTheDocument();
    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
    await waitFor(() => expect(screen.queryByRole("menu")).toBeNull());

    expect(reports).toEqual([true, false]);
    expect(trigger).toHaveAttribute("aria-expanded", "false");
    expect(updatesDuringRender()).toEqual([]);
  });

  /**
   * Given three menus, one open
   * When a second is opened, which closes every other menu
   * Then the first hears it closed, the second that it opened, and the third, never opened,
   *      hears nothing: a closed menu is not told it closed.
   */
  it("tells only the menus whose state changed when another opens", async () => {
    const first: boolean[] = [];
    const second: boolean[] = [];
    const third: boolean[] = [];
    render(
      <>
        <Owner name="First" reports={first} />
        <Owner name="Second" reports={second} />
        <Owner name="Third" reports={third} />
      </>,
    );

    fireEvent.click(screen.getByRole("button", { name: "First" }));
    expect(await screen.findByRole("menuitem", { name: "First item" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Second" }));
    expect(await screen.findByRole("menuitem", { name: "Second item" })).toBeInTheDocument();

    expect(first).toEqual([true, false]);
    expect(second).toEqual([true]);
    expect(third).toEqual([]);
    expect(screen.getByRole("button", { name: "First" })).toHaveAttribute("aria-expanded", "false");
    expect(screen.getByRole("button", { name: "Second" })).toHaveAttribute("aria-expanded", "true");
  });
});
