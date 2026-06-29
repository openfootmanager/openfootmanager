import { describe, it, expect, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useEntityEditor } from "./useEntityEditor";

type Item = { id: string; name: string };

const emptyItem = (): Item => ({ id: "", name: "" });

function makeHook(overrides: Partial<Parameters<typeof useEntityEditor<Item>>[0]> = {}) {
  const setItems = vi.fn();
  const captureHistory = vi.fn();
  const saveItems = vi.fn().mockResolvedValue(undefined);
  const onOpen = vi.fn();
  const onClose = vi.fn();
  const setIsBusy = vi.fn();

  const defaults = {
    items: [] as Item[],
    setItems,
    empty: emptyItem,
    captureHistory,
    saveItems,
    autoSave: false,
    onOpen,
    onClose,
    setIsBusy,
  };

  const hook = renderHook((props: Parameters<typeof useEntityEditor<Item>>[0]) =>
    useEntityEditor(props), { initialProps: { ...defaults, ...overrides } });

  return { hook, setItems, captureHistory, saveItems, onOpen, onClose, setIsBusy };
}

describe("useEntityEditor", () => {
  describe("revision (remount signal)", () => {
    it("bumps on select/add/syncEditing but not on updateField", () => {
      const items: Item[] = [{ id: "a", name: "Alpha" }, { id: "b", name: "Beta" }];
      const { hook } = makeHook({ items });
      const start = hook.result.current.revision;

      act(() => { hook.result.current.handleSelect(0); });
      const afterSelect = hook.result.current.revision;
      expect(afterSelect).toBeGreaterThan(start);

      // Editing a field must NOT bump revision (keeps the form mounted/focused).
      act(() => { hook.result.current.updateField("name", "Edited"); });
      expect(hook.result.current.revision).toBe(afterSelect);

      act(() => { hook.result.current.handleAdd(); });
      const afterAdd = hook.result.current.revision;
      expect(afterAdd).toBeGreaterThan(afterSelect);

      // undo/redo sync of the open record bumps it so the form remounts.
      act(() => { hook.result.current.handleSelect(1); });
      const beforeSync = hook.result.current.revision;
      act(() => {
        hook.result.current.syncEditing([{ id: "a", name: "Alpha" }, { id: "b", name: "Reverted" }]);
      });
      expect(hook.result.current.revision).toBeGreaterThan(beforeSync);
      expect(hook.result.current.editing).toEqual({ id: "b", name: "Reverted" });
    });
  });

  describe("handleSelect", () => {
    it("sets editing to a copy of the item at index", () => {
      const items: Item[] = [{ id: "a", name: "Alpha" }, { id: "b", name: "Beta" }];
      const { hook } = makeHook({ items });
      act(() => { hook.result.current.handleSelect(1); });
      expect(hook.result.current.editing).toEqual({ id: "b", name: "Beta" });
    });

    it("sets editingIndex to the given index", () => {
      const items: Item[] = [{ id: "x", name: "X" }];
      const { hook } = makeHook({ items });
      act(() => { hook.result.current.handleSelect(0); });
      expect(hook.result.current.editingIndex).toBe(0);
    });

    it("calls onOpen", () => {
      const items: Item[] = [{ id: "x", name: "X" }];
      const { hook, onOpen } = makeHook({ items });
      act(() => { hook.result.current.handleSelect(0); });
      expect(onOpen).toHaveBeenCalledTimes(1);
    });

    it("makes a shallow copy so mutations do not affect the original array", () => {
      const original: Item = { id: "x", name: "Original" };
      const { hook } = makeHook({ items: [original] });
      act(() => { hook.result.current.handleSelect(0); });
      expect(hook.result.current.editing).not.toBe(original);
    });
  });

  describe("handleAdd", () => {
    it("sets editing to an empty item", () => {
      const { hook } = makeHook();
      act(() => { hook.result.current.handleAdd(); });
      expect(hook.result.current.editing).toEqual(emptyItem());
    });

    it("sets editingIndex to null", () => {
      const items: Item[] = [{ id: "x", name: "X" }];
      const { hook } = makeHook({ items });
      act(() => { hook.result.current.handleSelect(0); });
      act(() => { hook.result.current.handleAdd(); });
      expect(hook.result.current.editingIndex).toBeNull();
    });

    it("calls onOpen", () => {
      const { hook, onOpen } = makeHook();
      act(() => { hook.result.current.handleAdd(); });
      expect(onOpen).toHaveBeenCalledTimes(1);
    });
  });

  describe("updateField", () => {
    it("updates the named field in editing", () => {
      const { hook } = makeHook();
      act(() => { hook.result.current.updateField("name", "New Name"); });
      expect(hook.result.current.editing.name).toBe("New Name");
    });

    it("does not replace other fields", () => {
      const { hook } = makeHook({ items: [{ id: "kept", name: "X" }] });
      act(() => { hook.result.current.handleSelect(0); });
      act(() => { hook.result.current.updateField("name", "Changed"); });
      expect(hook.result.current.editing.id).toBe("kept");
    });
  });

  describe("handleDelete", () => {
    it("calls setItems with the item removed", () => {
      const items: Item[] = [{ id: "a", name: "A" }, { id: "b", name: "B" }];
      const { hook, setItems } = makeHook({ items });
      act(() => { hook.result.current.handleDelete(0); });
      expect(setItems).toHaveBeenCalledWith([{ id: "b", name: "B" }]);
    });

    it("calls captureHistory before removing", () => {
      const items: Item[] = [{ id: "a", name: "A" }];
      const { hook, captureHistory } = makeHook({ items });
      act(() => { hook.result.current.handleDelete(0); });
      expect(captureHistory).toHaveBeenCalledTimes(1);
    });

    it("calls onClose when the deleted index matches editingIndex", () => {
      const items: Item[] = [{ id: "a", name: "A" }];
      const { hook, onClose } = makeHook({ items });
      act(() => { hook.result.current.handleSelect(0); });
      act(() => { hook.result.current.handleDelete(0); });
      expect(onClose).toHaveBeenCalledTimes(1);
    });

    it("does NOT call onClose when a different item is deleted", () => {
      const items: Item[] = [{ id: "a", name: "A" }, { id: "b", name: "B" }];
      const { hook, onClose } = makeHook({ items });
      act(() => { hook.result.current.handleSelect(0); });
      act(() => { hook.result.current.handleDelete(1); });
      expect(onClose).not.toHaveBeenCalled();
    });

    it("decrements editingIndex when an item before it is deleted", () => {
      const items: Item[] = [{ id: "a", name: "A" }, { id: "b", name: "B" }, { id: "c", name: "C" }];
      const { hook } = makeHook({ items });
      act(() => { hook.result.current.handleSelect(2); }); // editing index 2 (C)
      act(() => { hook.result.current.handleDelete(1); }); // delete index 1 (B)
      expect(hook.result.current.editingIndex).toBe(1); // C is now at index 1
    });

    it("does NOT decrement editingIndex when an item after it is deleted", () => {
      const items: Item[] = [{ id: "a", name: "A" }, { id: "b", name: "B" }, { id: "c", name: "C" }];
      const { hook } = makeHook({ items });
      act(() => { hook.result.current.handleSelect(0); }); // editing index 0 (A)
      act(() => { hook.result.current.handleDelete(2); }); // delete index 2 (C)
      expect(hook.result.current.editingIndex).toBe(0); // A is still at index 0
    });

    it("calls saveItems when autoSave is true", async () => {
      const items: Item[] = [{ id: "a", name: "A" }];
      const { hook, saveItems } = makeHook({ items, autoSave: true });
      await act(async () => { hook.result.current.handleDelete(0); });
      expect(saveItems).toHaveBeenCalledTimes(1);
    });

    it("does NOT call saveItems when autoSave is false", async () => {
      const items: Item[] = [{ id: "a", name: "A" }];
      const { hook, saveItems } = makeHook({ items, autoSave: false });
      await act(async () => { hook.result.current.handleDelete(0); });
      expect(saveItems).not.toHaveBeenCalled();
    });
  });

  describe("handleSave", () => {
    it("appends item to array when editingIndex is null (new item)", async () => {
      const items: Item[] = [{ id: "a", name: "A" }];
      const { hook, setItems } = makeHook({ items });
      act(() => { hook.result.current.handleAdd(); });
      act(() => { hook.result.current.updateField("id", "b"); });
      act(() => { hook.result.current.updateField("name", "B"); });
      await act(async () => { await hook.result.current.handleSave(); });
      expect(setItems).toHaveBeenCalledWith([
        { id: "a", name: "A" },
        { id: "b", name: "B" },
      ]);
    });

    it("replaces the item at editingIndex when editing an existing item", async () => {
      const items: Item[] = [{ id: "a", name: "Old" }, { id: "b", name: "B" }];
      const { hook, setItems } = makeHook({ items });
      act(() => { hook.result.current.handleSelect(0); });
      act(() => { hook.result.current.updateField("name", "New"); });
      await act(async () => { await hook.result.current.handleSave(); });
      expect(setItems).toHaveBeenCalledWith([{ id: "a", name: "New" }, { id: "b", name: "B" }]);
    });

    it("sets editingIndex to the newly appended index after adding", async () => {
      const items: Item[] = [{ id: "a", name: "A" }];
      const { hook } = makeHook({ items });
      act(() => { hook.result.current.handleAdd(); });
      await act(async () => { await hook.result.current.handleSave(); });
      expect(hook.result.current.editingIndex).toBe(1);
    });

    it("keeps editingIndex the same after editing", async () => {
      const items: Item[] = [{ id: "a", name: "A" }, { id: "b", name: "B" }];
      const { hook } = makeHook({ items });
      act(() => { hook.result.current.handleSelect(1); });
      await act(async () => { await hook.result.current.handleSave(); });
      expect(hook.result.current.editingIndex).toBe(1);
    });

    it("calls captureHistory before saving", async () => {
      const { hook, captureHistory } = makeHook();
      await act(async () => { await hook.result.current.handleSave(); });
      expect(captureHistory).toHaveBeenCalledTimes(1);
    });

    it("calls saveItems with updated array when autoSave is true", async () => {
      const items: Item[] = [{ id: "a", name: "A" }];
      const { hook, saveItems } = makeHook({ items, autoSave: true });
      act(() => { hook.result.current.handleAdd(); });
      act(() => { hook.result.current.updateField("id", "b"); });
      await act(async () => { await hook.result.current.handleSave(); });
      expect(saveItems).toHaveBeenCalledWith([
        { id: "a", name: "A" },
        { id: "b", name: "" },  // name not updated but id is
      ]);
    });

    it("does NOT call saveItems when autoSave is false", async () => {
      const { hook, saveItems } = makeHook({ autoSave: false });
      await act(async () => { await hook.result.current.handleSave(); });
      expect(saveItems).not.toHaveBeenCalled();
    });

    it("sets isBusy true then false during autoSave", async () => {
      const { hook, setIsBusy } = makeHook({ autoSave: true });
      await act(async () => { await hook.result.current.handleSave(); });
      expect(setIsBusy).toHaveBeenCalledWith(true);
      expect(setIsBusy).toHaveBeenLastCalledWith(false);
    });
  });

  describe("syncEditing", () => {
    it("refreshes editing from newItems at the current editingIndex", () => {
      const items: Item[] = [{ id: "a", name: "Old" }];
      const { hook } = makeHook({ items });
      act(() => { hook.result.current.handleSelect(0); });
      act(() => { hook.result.current.syncEditing([{ id: "a", name: "Restored" }]); });
      expect(hook.result.current.editing).toEqual({ id: "a", name: "Restored" });
    });

    it("is a no-op when editingIndex is null", () => {
      const { hook } = makeHook();
      act(() => { hook.result.current.syncEditing([{ id: "a", name: "X" }]); });
      expect(hook.result.current.editing).toEqual(emptyItem());
    });

    it("re-locates the edited record by id after an undo reorders the list", () => {
      const items: Item[] = [{ id: "a", name: "A" }, { id: "b", name: "B" }, { id: "c", name: "C" }];
      const { hook } = makeHook({ items });
      act(() => { hook.result.current.handleSelect(2); }); // editing "c" at index 2

      // A restored snapshot where the same records are reordered: "c" is now first.
      // Index 2 would point at a *different* record, so tracking must be by id.
      const reordered: Item[] = [
        { id: "c", name: "C-reverted" },
        { id: "a", name: "A" },
        { id: "b", name: "B" },
      ];
      act(() => { hook.result.current.syncEditing(reordered); });

      expect(hook.result.current.editing).toEqual({ id: "c", name: "C-reverted" });
      expect(hook.result.current.editingIndex).toBe(0);
    });

    it("closes the editor when editingIndex is out of bounds in newItems", () => {
      const items: Item[] = [{ id: "a", name: "A" }, { id: "b", name: "B" }];
      const { hook, onClose } = makeHook({ items });
      act(() => { hook.result.current.handleSelect(1); });
      // An undo that removed the record at index 1. Without closing, a later save
      // would re-add the record the user just reverted.
      act(() => { hook.result.current.syncEditing([{ id: "a", name: "A" }]); });
      expect(onClose).toHaveBeenCalledTimes(1);
    });
  });
});
