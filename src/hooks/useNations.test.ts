import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useNations } from "./useNations";
import { resetNationsCache } from "../services/nationsService";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

const CATALOG = [
  { code: "BR", name: "Brazil", region: "south-america" },
  { code: "ENG", name: "England", region: "europe" },
];

beforeEach(() => {
  resetNationsCache();
  invoke.mockReset();
});

describe("useNations", () => {
  it("is null until the catalog answers", () => {
    // The three states are not interchangeable: `null` means "not known yet",
    // and callers use it to withhold a picker that would otherwise be built
    // from an empty catalog and read as authoritative.
    invoke.mockImplementation(() => new Promise(() => {}));

    const { result } = renderHook(() => useNations());

    expect(result.current).toBeNull();
  });

  it("resolves to the catalog the backend returns", async () => {
    invoke.mockResolvedValue(CATALOG);

    const { result } = renderHook(() => useNations());

    await waitFor(() => expect(result.current).toEqual(CATALOG));
  });

  it("falls back to an empty catalog when the call fails", async () => {
    // Empty rather than stuck at `null`: nothing can be called built-in without
    // a catalog, so callers fall back to authoring by hand — which shows the
    // author their own data untouched, the safe direction to fail in.
    const errors = vi.spyOn(console, "error").mockImplementation(() => {});
    invoke.mockRejectedValue(new Error("backend is not there"));

    try {
      const { result } = renderHook(() => useNations());

      await waitFor(() => expect(result.current).toEqual([]));
      // And it is distinguishable from a genuinely empty catalog.
      expect(errors).toHaveBeenCalled();
    } finally {
      errors.mockRestore();
    }
  });

  it("does not set state after the caller has gone away", async () => {
    // The catalog can answer after the form was closed. Setting state then is a
    // React warning at best and a leak at worst.
    const errors = vi.spyOn(console, "error").mockImplementation(() => {});
    let answer: (list: typeof CATALOG) => void = () => {};
    invoke.mockImplementation(
      () =>
        new Promise((resolve) => {
          answer = resolve as (list: typeof CATALOG) => void;
        }),
    );

    try {
      const { unmount } = renderHook(() => useNations());
      unmount();
      answer(CATALOG);
      await Promise.resolve();

      expect(errors).not.toHaveBeenCalled();
    } finally {
      errors.mockRestore();
    }
  });
});
