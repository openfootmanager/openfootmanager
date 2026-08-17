import { render, screen } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import i18n, { i18nReady } from "../../i18n";
import SavesList from "./SavesList";

// Uses the real i18n instance rather than a mocked `t`, because two of these
// tests are about what i18next itself does with the value: resolving a
// backend-generated key, and escaping player text before it reaches
// `dangerouslySetInnerHTML`.
beforeAll(async () => {
  await i18nReady;
  await i18n.changeLanguage("en");
});

function save(overrides: Partial<{ id: string; name: string }> = {}) {
  return {
    id: "save-1",
    name: "Old Save",
    manager_name: "Jane Doe",
    team_name: "Alpha FC",
    db_filename: "save-1.db",
    checksum: "abc",
    created_at: "2026-08-01T00:00:00Z",
    last_played_at: "2026-08-02T00:00:00Z",
    ...overrides,
  };
}

function renderList(
  saves: ReturnType<typeof save>[],
  confirmDeleteId: string | null = null,
) {
  render(
    <SavesList
      saves={saves}
      isLoading={false}
      confirmDeleteId={confirmDeleteId}
      onLoad={vi.fn()}
      onDelete={vi.fn()}
      onConfirmDelete={vi.fn()}
      onClose={vi.fn()}
    />,
  );
}

describe("SavesList", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
  });

  it("resolves a save name the game generated as a translation key", () => {
    renderList([save({ name: "be.save.defaultName?manager=Jane%20Doe" })]);

    expect(screen.getByText("Jane Doe's Career")).toBeInTheDocument();
  });

  it("translates that name into the player's language", async () => {
    await i18n.changeLanguage("fr");
    renderList([save({ name: "be.save.defaultName?manager=Jane%20Doe" })]);

    expect(screen.getByText("Carrière de Jane Doe")).toBeInTheDocument();
  });

  // Saves written before the key existed, and any the player renamed, are
  // plain text and must survive untouched.
  it("shows a plain save name unchanged", () => {
    renderList([save({ name: "My 1997 run" })]);

    expect(screen.getByText("My 1997 run")).toBeInTheDocument();
  });

  // `menu.deleteConfirm` carries its own <strong> markup, so it is rendered
  // through dangerouslySetInnerHTML. The save name inside it is player text.
  it("escapes markup in a save name instead of rendering it", () => {
    const { container } = render(
      <SavesList
        saves={[save({ name: '<img src=x onerror="alert(1)">' })]}
        isLoading={false}
        confirmDeleteId="save-1"
        onLoad={vi.fn()}
        onDelete={vi.fn()}
        onConfirmDelete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(container.querySelector("img")).toBeNull();
    // The <strong> the translation itself provides must still be real markup.
    expect(container.querySelector("strong")).not.toBeNull();
    expect(container.textContent).toContain('<img src=x onerror="alert(1)">');
  });
});
