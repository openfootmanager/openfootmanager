import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

import { useAssetPicker } from "./useAssetPicker";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);
const mockedOpen = vi.mocked(open);

// Resolve copy_package_asset to a fixed path and read_file_as_data_url (called
// by the embedded useAssetDataUrl) to a placeholder, so the hook exercises its
// full flow without a backend.
function stubBackend(copiedPath = "assets/images/man-utd.png") {
  mockedInvoke.mockImplementation(async (cmd: string) => {
    if (cmd === "copy_package_asset") return copiedPath;
    if (cmd === "read_file_as_data_url") return "data:image/png;base64,AAA";
    return null;
  });
}

function setup(overrides: Partial<Parameters<typeof useAssetPicker>[0]> = {}) {
  const commit = vi.fn();
  const onError = vi.fn();
  const opts = {
    relPath: null as string | null,
    projectDir: "/proj" as string | undefined,
    entityId: () => "man-utd",
    commit,
    onError,
    ...overrides,
  };
  const hook = renderHook(() => useAssetPicker(opts));
  return { hook, commit, onError, opts };
}

describe("useAssetPicker", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedOpen.mockReset();
    stubBackend();
  });

  it("copies the chosen file and commits the returned path", async () => {
    mockedOpen.mockResolvedValue("/home/me/crest.png");
    const { hook, commit, onError } = setup();

    await act(async () => { await hook.result.current.pick(); });

    expect(mockedInvoke).toHaveBeenCalledWith("copy_package_asset", {
      dir: "/proj",
      entityId: "man-utd",
      srcPath: "/home/me/crest.png",
    });
    expect(commit).toHaveBeenCalledWith("assets/images/man-utd.png");
    expect(onError).not.toHaveBeenCalled();
  });

  it("reports a copy failure through onError instead of swallowing it", async () => {
    mockedOpen.mockResolvedValue("/home/me/crest.png");
    mockedInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "copy_package_asset") throw new Error("be.error.invalidPath");
      return "data:image/png;base64,AAA";
    });
    const { hook, commit, onError } = setup();

    await act(async () => { await hook.result.current.pick(); });

    expect(onError).toHaveBeenCalledTimes(1);
    expect(commit).not.toHaveBeenCalled();
  });

  it("reports a dialog failure through onError, not as an unhandled rejection", async () => {
    // Callers invoke this as `void pick()`, so an open() rejection that escaped
    // the try/catch would never reach onError.
    mockedOpen.mockRejectedValue(new Error("dialog crashed"));
    const { hook, commit, onError } = setup();

    await act(async () => { await hook.result.current.pick(); });

    expect(onError).toHaveBeenCalledTimes(1);
    expect(commit).not.toHaveBeenCalled();
  });

  it("does nothing when the dialog is cancelled", async () => {
    mockedOpen.mockResolvedValue(null);
    const { hook, commit, onError } = setup();

    await act(async () => { await hook.result.current.pick(); });

    expect(mockedInvoke).not.toHaveBeenCalledWith("copy_package_asset", expect.anything());
    expect(commit).not.toHaveBeenCalled();
    expect(onError).not.toHaveBeenCalled();
  });

  it("commits null when cleared", () => {
    const { hook, commit } = setup();
    act(() => { hook.result.current.clear(); });
    expect(commit).toHaveBeenCalledWith(null);
  });

  it("reads entityId lazily, at pick time", async () => {
    mockedOpen.mockResolvedValue("/home/me/crest.png");
    const entityId = vi.fn(() => "late-id");
    const { hook } = setup({ entityId });

    // Not consulted merely by rendering the hook.
    expect(entityId).not.toHaveBeenCalled();
    await act(async () => { await hook.result.current.pick(); });

    expect(mockedInvoke).toHaveBeenCalledWith(
      "copy_package_asset",
      expect.objectContaining({ entityId: "late-id" }),
    );
  });

  it("exposes the current asset as a data URL", async () => {
    const { hook } = setup({ relPath: "assets/images/man-utd.png" });
    await waitFor(() => expect(hook.result.current.dataUrl).toBe("data:image/png;base64,AAA"));
  });
});
