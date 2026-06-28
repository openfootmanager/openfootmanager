import { describe, it, expect, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useNamesPoolEditor } from "./useNamesPoolEditor";
import type { NamesDefinition } from "../components/menu/PackageEditor/types";

function makeHook(names: NamesDefinition) {
  const setNames = vi.fn();
  const captureHistory = vi.fn();
  const saveNames = vi.fn().mockResolvedValue(undefined);
  const onClose = vi.fn();
  const hook = renderHook(() =>
    useNamesPoolEditor({
      names,
      setNames,
      autoSave: false,
      captureHistory,
      saveNames,
      onOpen: vi.fn(),
      onClose,
      setIsBusy: vi.fn(),
    }),
  );
  return { hook, setNames, captureHistory, onClose };
}

const NAMES: NamesDefinition = {
  version: 1,
  description: "",
  pools: {
    ENG: { first_names: ["John"], last_names: ["Smith"] },
    BRA: { first_names: ["Joao"], last_names: ["Silva"] },
  },
};

describe("useNamesPoolEditor", () => {
  it("rejects renaming a pool onto another existing key (no data loss)", () => {
    const { hook, setNames } = makeHook(NAMES);
    // Open ENG, then try to rename it to BRA (which already exists).
    act(() => { hook.result.current.handleSelectPool("ENG"); });
    act(() => {
      void hook.result.current.handleSavePool("BRA", { first_names: ["X"], last_names: ["Y"] });
    });
    // Save is blocked → no setNames write that would collapse the two pools.
    expect(setNames).not.toHaveBeenCalled();
  });

  it("allows saving a pool under a new, unused key", () => {
    const { hook, setNames } = makeHook(NAMES);
    act(() => { hook.result.current.handleSelectPool("ENG"); });
    act(() => {
      void hook.result.current.handleSavePool("WAL", { first_names: ["Dafydd"], last_names: ["Jones"] });
    });
    expect(setNames).toHaveBeenCalledTimes(1);
    const written = setNames.mock.calls[0][0] as NamesDefinition;
    // The original BRA pool must still be present.
    expect(written.pools.BRA).toBeDefined();
    expect(written.pools.WAL).toBeDefined();
  });

  it("syncEditing refreshes the open pool buffer and bumps revision", () => {
    const { hook } = makeHook(NAMES);
    act(() => { hook.result.current.handleSelectPool("ENG"); });
    const before = hook.result.current.revision;
    const reverted: NamesDefinition = {
      ...NAMES,
      pools: { ...NAMES.pools, ENG: { first_names: ["Reverted"], last_names: ["Name"] } },
    };
    act(() => { hook.result.current.syncEditing(reverted); });
    expect(hook.result.current.revision).toBeGreaterThan(before);
    expect(hook.result.current.editingPool).toEqual({ first_names: ["Reverted"], last_names: ["Name"] });
  });

  it("syncEditing closes the editor when the restored snapshot removed the active key", () => {
    const { hook, onClose } = makeHook(NAMES);
    act(() => { hook.result.current.handleSelectPool("ENG"); });
    // An undo that dropped the ENG pool entirely. Without closing, a later save
    // would recreate the key the user just reverted.
    const withoutEng: NamesDefinition = {
      ...NAMES,
      pools: { BRA: NAMES.pools.BRA },
    };
    act(() => { hook.result.current.syncEditing(withoutEng); });
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
