import { act, fireEvent, screen, waitFor, within } from "@testing-library/react";
import { clearMocks } from "@tauri-apps/api/mocks";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";

import { i18nReady } from "../../i18n";
import { useGameStore } from "../../store/gameStore";
import { readStyleGuideOptions } from "../styleGuide/styleGuideOptions";
import { mountPageHarness } from "./mount";

beforeAll(async () => {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    })),
  });
  await i18nReady;
});

let mounted: Awaited<ReturnType<typeof mountPageHarness>> | null = null;

afterEach(async () => {
  await act(async () => mounted?.unmount());
  mounted = null;
  clearMocks();
  useGameStore.getState().clearGame();
  document.body.innerHTML = "";
});

async function openSquad() {
  const root = document.createElement("div");
  document.body.append(root);
  await act(async () => {
    mounted = await mountPageHarness(root, "squad", readStyleGuideOptions(""), "en");
  });
  const view = within(root);
  const position = () => view.getByRole("combobox", { name: "Pos" });
  const status = () => view.getByRole("combobox", { name: "Status" });
  const search = () => view.getByPlaceholderText("Filter by player name or position");
  await view.findByRole("combobox", { name: "Pos" });
  fireEvent.click(position());
  fireEvent.click(await screen.findByRole("option", { name: "GK" }));
  fireEvent.click(status());
  fireEvent.click(await screen.findByRole("option", { name: "Starting XI" }));

  const table = within(view.getByRole("table"));
  const player = within(table.getAllByRole("row")[1]).getAllByRole("button")[0];
  const name = player.getAttribute("aria-label");
  if (!name) throw new Error("Expected an accessible player name in the starting XI");
  fireEvent.change(search(), { target: { value: name } });

  return { view, name, position, search, status };
}

describe("Squad filter navigation", () => {
  /**
   * Given the Squad roster is narrowed by name, position and status
   * When the manager visits the remaining player's profile and goes back
   * Then the same filters and result return and Clear resets them.
   */
  it("retains the squad filters when returning from a player profile", async () => {
    const squad = await openSquad();
    const player = () =>
      within(squad.view.getByRole("table")).getByRole("button", { name: squad.name });
    fireEvent.click(player());
    await squad.view.findByRole("tab", { name: "Overview" });
    expect(squad.view.queryByRole("combobox", { name: "Pos" })).toBeNull();
    fireEvent.click(squad.view.getByRole("button", { name: /^Back$/ }));

    await waitFor(() => expect(squad.position()).toHaveTextContent("GK"));
    expect(squad.status()).toHaveTextContent("Starting XI");
    expect(squad.search()).toHaveValue(squad.name);
    expect(within(squad.view.getByRole("table")).getAllByRole("row")).toHaveLength(2);
    expect(player()).toBeInTheDocument();

    const clear = squad.view.getByRole("button", { name: "Clear" });
    fireEvent.click(clear);
    expect(squad.position()).toHaveTextContent("All");
    expect(squad.status()).toHaveTextContent("All Players");
    expect(squad.search()).toHaveValue("");
    expect(clear).toBeDisabled();
  });

  /** Given existing filters, when a different career is installed, then Squad opens unfiltered. */
  it("starts with default filters for a different active career", async () => {
    const squad = await openSquad();
    expect(squad.search()).toHaveValue(squad.name);
    const store = useGameStore.getState();
    if (!store.gameState) throw new Error("Expected an active fixture career");
    const gameState = store.gameState;

    await act(async () => {
      store.setGameState({ ...gameState, career_token: (gameState.career_token ?? 0) + 1 });
    });

    expect(squad.position()).toHaveTextContent("All");
    expect(squad.status()).toHaveTextContent("All Players");
    expect(squad.search()).toHaveValue("");
  });
});
