import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";

import { useAssetDataUrl, evictAssetDataUrl } from "./useAssetDataUrl";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);

describe("useAssetDataUrl", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation(async () => "data:image/png;base64,AAA");
  });

  it("returns null without fetching when no path or projectDir is given", () => {
    const { result } = renderHook(() => useAssetDataUrl(null, "/proj"));
    expect(result.current).toBeNull();
    expect(mockedInvoke).not.toHaveBeenCalled();
  });

  it("resolves a relative path to a data URL via read_file_as_data_url", async () => {
    const { result } = renderHook(() => useAssetDataUrl("a/logo.png", "/proj1"));
    await waitFor(() => expect(result.current).toBe("data:image/png;base64,AAA"));
    expect(mockedInvoke).toHaveBeenCalledWith("read_file_as_data_url", {
      path: "/proj1/a/logo.png",
      baseDir: "/proj1",
    });
  });

  it("serves the same path from cache without a second IPC call", async () => {
    const first = renderHook(() => useAssetDataUrl("b/logo.png", "/proj2"));
    await waitFor(() => expect(first.result.current).toBe("data:image/png;base64,AAA"));
    expect(mockedInvoke).toHaveBeenCalledTimes(1);

    // A second consumer of the same asset resolves synchronously from cache.
    const second = renderHook(() => useAssetDataUrl("b/logo.png", "/proj2"));
    expect(second.result.current).toBe("data:image/png;base64,AAA");
    expect(mockedInvoke).toHaveBeenCalledTimes(1);
  });

  it("re-fetches after the cached entry is evicted (asset replaced in place)", async () => {
    const first = renderHook(() => useAssetDataUrl("c/logo.png", "/proj3"));
    await waitFor(() => expect(first.result.current).not.toBeNull());
    expect(mockedInvoke).toHaveBeenCalledTimes(1);

    evictAssetDataUrl("/proj3", "c/logo.png");
    mockedInvoke.mockImplementation(async () => "data:image/png;base64,BBB");

    const second = renderHook(() => useAssetDataUrl("c/logo.png", "/proj3"));
    await waitFor(() => expect(second.result.current).toBe("data:image/png;base64,BBB"));
    expect(mockedInvoke).toHaveBeenCalledTimes(2);
  });

  it("re-fetches when refreshKey changes even if the path is unchanged", async () => {
    evictAssetDataUrl("/proj5", "e/logo.png"); // ensure a clean cache for this key
    const { result, rerender } = renderHook(
      ({ key }: { key: number }) => useAssetDataUrl("e/logo.png", "/proj5", key),
      { initialProps: { key: 0 } },
    );
    await waitFor(() => expect(result.current).toBe("data:image/png;base64,AAA"));
    expect(mockedInvoke).toHaveBeenCalledTimes(1);

    // Same path, but the file was replaced in place: evict + bump refreshKey.
    evictAssetDataUrl("/proj5", "e/logo.png");
    mockedInvoke.mockImplementation(async () => "data:image/png;base64,CCC");
    rerender({ key: 1 });

    await waitFor(() => expect(result.current).toBe("data:image/png;base64,CCC"));
    expect(mockedInvoke).toHaveBeenCalledTimes(2);
  });

  it("returns null when the IPC call rejects", async () => {
    mockedInvoke.mockImplementation(async () => { throw new Error("missing"); });
    const { result } = renderHook(() => useAssetDataUrl("d/missing.png", "/proj4"));
    // Stays null; one failed attempt, no crash.
    await waitFor(() => expect(mockedInvoke).toHaveBeenCalledTimes(1));
    expect(result.current).toBeNull();
  });
});
