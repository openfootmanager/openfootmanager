import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { useRecentProjects } from "./useRecentProjects";

const KEY = "worldEditor.recentProjects";

describe("useRecentProjects", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  afterEach(() => {
    localStorage.clear();
  });

  it("starts empty when localStorage has nothing", () => {
    const { result } = renderHook(() => useRecentProjects());
    expect(result.current.recentProjects).toEqual([]);
  });

  it("hydrates initial state from localStorage", () => {
    localStorage.setItem(
      KEY,
      JSON.stringify([{ path: "/a", name: "A", openedAt: "2020-01-01T00:00:00.000Z" }]),
    );
    const { result } = renderHook(() => useRecentProjects());
    expect(result.current.recentProjects).toHaveLength(1);
    expect(result.current.recentProjects[0].path).toBe("/a");
  });

  it("addRecentProject prepends and persists to localStorage", () => {
    const { result } = renderHook(() => useRecentProjects());

    act(() => {
      result.current.addRecentProject("/a", "A");
    });
    act(() => {
      result.current.addRecentProject("/b", "B");
    });

    expect(result.current.recentProjects.map((p) => p.path)).toEqual(["/b", "/a"]);
    const stored = JSON.parse(localStorage.getItem(KEY) as string) as { path: string }[];
    expect(stored.map((p) => p.path)).toEqual(["/b", "/a"]);
  });

  it("de-duplicates by path, moving the re-added entry to the front", () => {
    const { result } = renderHook(() => useRecentProjects());

    act(() => {
      result.current.addRecentProject("/a", "A");
    });
    act(() => {
      result.current.addRecentProject("/b", "B");
    });
    act(() => {
      result.current.addRecentProject("/a", "A renamed");
    });

    expect(result.current.recentProjects.map((p) => p.path)).toEqual(["/a", "/b"]);
    expect(result.current.recentProjects[0].name).toBe("A renamed");
  });

  it("caps the list at 8 entries (newest first)", () => {
    const { result } = renderHook(() => useRecentProjects());

    act(() => {
      for (let i = 0; i < 10; i++) {
        result.current.addRecentProject(`/p${i}`, `P${i}`);
      }
    });

    expect(result.current.recentProjects).toHaveLength(8);
    expect(result.current.recentProjects[0].path).toBe("/p9");
    expect(result.current.recentProjects[7].path).toBe("/p2");
  });
});
