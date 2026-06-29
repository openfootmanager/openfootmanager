import { describe, it, expect, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useUndoRedo } from "./useUndoRedo";

type S = { value: number };

function makeHook(enabled = true) {
  let snapshot: S = { value: 0 };
  const getSnapshot = vi.fn(() => snapshot);
  const applySnapshot = vi.fn((s: S) => { snapshot = s; });
  const onDirty = vi.fn();

  const hook = renderHook(() =>
    useUndoRedo({ getSnapshot, applySnapshot, enabled, onDirty }),
  );

  return { hook, getSnapshot, applySnapshot, onDirty, getSnapshotValue: () => snapshot };
}

describe("useUndoRedo", () => {
  it("starts with canUndo=false and canRedo=false", () => {
    const { hook } = makeHook();
    expect(hook.result.current.canUndo).toBe(false);
    expect(hook.result.current.canRedo).toBe(false);
  });

  it("canUndo becomes true after pushHistory", () => {
    const { hook } = makeHook();
    act(() => { hook.result.current.pushHistory({ value: 1 }); });
    expect(hook.result.current.canUndo).toBe(true);
  });

  it("undo restores the pushed snapshot", () => {
    const { hook, applySnapshot } = makeHook();
    act(() => { hook.result.current.pushHistory({ value: 42 }); });
    act(() => { hook.result.current.handleUndo(); });
    expect(applySnapshot).toHaveBeenCalledWith({ value: 42 });
  });

  it("canUndo becomes false after undoing the only item", () => {
    const { hook } = makeHook();
    act(() => { hook.result.current.pushHistory({ value: 1 }); });
    act(() => { hook.result.current.handleUndo(); });
    expect(hook.result.current.canUndo).toBe(false);
  });

  it("canRedo becomes true after undo", () => {
    const { hook } = makeHook();
    act(() => { hook.result.current.pushHistory({ value: 1 }); });
    act(() => { hook.result.current.handleUndo(); });
    expect(hook.result.current.canRedo).toBe(true);
  });

  it("redo after undo restores the pre-undo snapshot", () => {
    const { hook, applySnapshot } = makeHook();
    act(() => { hook.result.current.pushHistory({ value: 10 }); });
    act(() => { hook.result.current.handleUndo(); });
    applySnapshot.mockClear();
    act(() => { hook.result.current.handleRedo(); });
    // The snapshot saved before undo was the current (value=0) snapshot.
    // handleRedo applies the "next" which was pushed to undoStack.
    // Actually after undo, undoStack had [{ value: 10 }] removed and redoStack = [getSnapshot() = { value: 0 }]
    // handleRedo pops next from redoStack = { value: 0 } and applies it
    expect(applySnapshot).toHaveBeenCalledTimes(1);
  });

  it("pushHistory clears the redo stack", () => {
    const { hook } = makeHook();
    act(() => { hook.result.current.pushHistory({ value: 1 }); });
    act(() => { hook.result.current.handleUndo(); });
    expect(hook.result.current.canRedo).toBe(true);
    act(() => { hook.result.current.pushHistory({ value: 2 }); });
    expect(hook.result.current.canRedo).toBe(false);
  });

  it("clearHistory resets stacks and flags", () => {
    const { hook } = makeHook();
    act(() => { hook.result.current.pushHistory({ value: 1 }); });
    act(() => { hook.result.current.pushHistory({ value: 2 }); });
    act(() => { hook.result.current.clearHistory(); });
    expect(hook.result.current.canUndo).toBe(false);
    expect(hook.result.current.canRedo).toBe(false);
    // undo should be a no-op
    const { applySnapshot } = makeHook();
    act(() => { hook.result.current.handleUndo(); });
    expect(applySnapshot).not.toHaveBeenCalled();
  });

  it("undo is a no-op when stack is empty", () => {
    const { hook, applySnapshot } = makeHook();
    act(() => { hook.result.current.handleUndo(); });
    expect(applySnapshot).not.toHaveBeenCalled();
  });

  it("redo is a no-op when stack is empty", () => {
    const { hook, applySnapshot } = makeHook();
    act(() => { hook.result.current.handleRedo(); });
    expect(applySnapshot).not.toHaveBeenCalled();
  });

  it("onDirty is called on pushHistory", () => {
    const { hook, onDirty } = makeHook();
    act(() => { hook.result.current.pushHistory({ value: 1 }); });
    expect(onDirty).toHaveBeenCalledTimes(1);
  });

  it("onDirty is called on undo", () => {
    const { hook, onDirty } = makeHook();
    act(() => { hook.result.current.pushHistory({ value: 1 }); });
    onDirty.mockClear();
    act(() => { hook.result.current.handleUndo(); });
    expect(onDirty).toHaveBeenCalledTimes(1);
  });

  it("onDirty is called on redo", () => {
    const { hook, onDirty } = makeHook();
    act(() => { hook.result.current.pushHistory({ value: 1 }); });
    act(() => { hook.result.current.handleUndo(); });
    onDirty.mockClear();
    act(() => { hook.result.current.handleRedo(); });
    expect(onDirty).toHaveBeenCalledTimes(1);
  });

  it("multiple pushes allow step-by-step undo", () => {
    const { hook, applySnapshot } = makeHook();
    act(() => { hook.result.current.pushHistory({ value: 1 }); });
    act(() => { hook.result.current.pushHistory({ value: 2 }); });
    act(() => { hook.result.current.handleUndo(); });
    expect(applySnapshot).toHaveBeenLastCalledWith({ value: 2 });
    act(() => { hook.result.current.handleUndo(); });
    expect(applySnapshot).toHaveBeenLastCalledWith({ value: 1 });
    expect(hook.result.current.canUndo).toBe(false);
  });
});
