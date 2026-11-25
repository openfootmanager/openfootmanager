import { act, fireEvent, screen, waitFor, within } from "@testing-library/react";
import type { invoke } from "@tauri-apps/api/core";
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
  vi.restoreAllMocks();
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
  const search = () => {
    const workspace = root.querySelector<HTMLElement>("[data-workspace]");
    if (!workspace) throw new Error("Expected a dashboard workspace");
    return within(workspace).getByRole("textbox", { name: "Search" });
  };
  await view.findByRole("combobox", { name: "Pos" });
  return { view, position, search, status };
}

async function openFilteredSquad() {
  const squad = await openSquad();
  const { view, position, search, status } = squad;
  fireEvent.click(position());
  fireEvent.click(await screen.findByRole("option", { name: "GK" }));
  fireEvent.click(status());
  fireEvent.click(await screen.findByRole("option", { name: "Starting XI" }));

  const table = within(view.getByRole("table"));
  const player = within(table.getAllByRole("row")[1]).getAllByRole("button")[0];
  const name = player.getAttribute("aria-label");
  if (!name) throw new Error("Expected an accessible player name in the starting XI");
  fireEvent.change(search(), { target: { value: name } });

  return { ...squad, name };
}

async function returnWithFitFilter() {
  const squad = await openSquad();
  const rows = () => within(squad.view.getByRole("table")).getAllByRole("row");
  const unfilteredRows = rows().length;
  fireEvent.click(squad.status());
  fireEvent.click(await screen.findByRole("option", { name: "Needs cover" }));
  expect(squad.status()).toHaveTextContent("Needs cover");
  const filteredRows = rows().length;
  expect(filteredRows).toBeLessThan(unfilteredRows);
  fireEvent.click(within(rows()[1]).getAllByRole("button")[0]);
  await squad.view.findByRole("tab", { name: "Overview" });

  const { __TAURI_INTERNALS__: ipc } = window as Window & {
    __TAURI_INTERNALS__?: { invoke: typeof invoke };
  };
  if (!ipc) throw new Error("Expected the page harness IPC mock");
  const invokeFixture = ipc.invoke;
  let resolve = () => {};
  let reject = (_error: Error) => {};
  const pending = new Promise<void>((onSuccess, onError) => {
    resolve = onSuccess;
    reject = onError;
  });
  const returning = vi.fn();
  vi.spyOn(ipc, "invoke").mockImplementation((...args) => {
    if (args[0] !== "get_squad") return invokeFixture(...args);
    returning();
    return pending.then(() => invokeFixture(...args));
  });
  fireEvent.click(squad.view.getByRole("button", { name: /^Back$/ }));
  await waitFor(() => expect(returning).toHaveBeenCalled());
  return { ...squad, rows, filteredRows, unfilteredRows, resolve, reject };
}

describe("Squad filter navigation", () => {
  /**
   * Given the Squad roster is narrowed by name, position and status
   * When the manager visits the remaining player's profile and goes back
   * Then the same filters and result return and Clear resets them.
   */
  it("retains the squad filters when returning from a player profile", async () => {
    const squad = await openFilteredSquad();
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
    const squad = await openFilteredSquad();
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

  /** Given Needs cover is selected, when Squad reloads after Back, then Status shows that filter. */
  it("shows the retained fit filter while the returning squad request is pending", async () => {
    const squad = await returnWithFitFilter();
    expect(squad.rows()).toHaveLength(squad.filteredRows);
    expect(squad.status()).toHaveTextContent("Needs cover");

    fireEvent.click(squad.status());
    const active = screen.getByRole("option", { name: "Needs cover", selected: true });
    expect(active).toBeDisabled();
    expect(screen.queryByRole("option", { name: "Natural position" })).toBeNull();
    expect(screen.queryByRole("option", { name: "Out of position" })).toBeNull();
    expect(screen.getByRole("option", { name: "All Players" })).toBeEnabled();

    await act(async () => squad.resolve());
    expect(squad.status()).toHaveTextContent("Needs cover");
    expect(screen.getByRole("option", { name: "Needs cover", selected: true })).toBeEnabled();
    expect(screen.getByRole("option", { name: "Natural position" })).toBeEnabled();
    expect(screen.getByRole("option", { name: "Out of position" })).toBeEnabled();
  });

  /** Given Squad has a retained fit filter, when the reload fails, then Clear still resets it. */
  it("shows the retained fit filter after the returning squad request fails", async () => {
    const squad = await returnWithFitFilter();
    const error = new Error("fixture squad request failed");
    const log = vi.spyOn(console, "error").mockImplementation(() => {});
    await act(async () => squad.reject(error));
    expect(log).toHaveBeenCalledWith("Failed to fetch squad", error);
    expect(squad.rows()).toHaveLength(squad.filteredRows);
    expect(squad.status()).toHaveTextContent("Needs cover");

    fireEvent.click(squad.view.getByRole("button", { name: "Clear" }));
    expect(squad.status()).toHaveTextContent("All Players");
    expect(squad.rows()).toHaveLength(squad.unfilteredRows);
  });
});
