import { describe, it, expect, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useState } from "react";
import { useEntityEditor } from "./useEntityEditor";

type Item = { id: string; name: string; logo?: string | null };

const emptyItem = (): Item => ({ id: "", name: "" });

function makeHook(overrides: Partial<Parameters<typeof useEntityEditor<Item>>[0]> = {}) {
  const setItems = vi.fn();
  const captureHistory = vi.fn();
  const saveItems = vi.fn().mockResolvedValue(undefined);
  const onOpen = vi.fn();
  const onClose = vi.fn();
  const setIsBusy = vi.fn();
  const onDirty = vi.fn();

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
    onDirty,
  };

  const hook = renderHook(
    (props: Parameters<typeof useEntityEditor<Item>>[0]) => useEntityEditor(props),
    { initialProps: { ...defaults, ...overrides } },
  );

  return {
    hook,
    setItems,
    captureHistory,
    saveItems,
    onOpen,
    onClose,
    setIsBusy,
    onDirty,
    defaults,
  };
}

describe("useEntityEditor", () => {
  describe("revision (remount signal)", () => {
    it("bumps on select/add/syncEditing but not on updateField", () => {
      const items: Item[] = [
        { id: "a", name: "Alpha" },
        { id: "b", name: "Beta" },
      ];
      const { hook } = makeHook({ items });
      const start = hook.result.current.revision;

      act(() => {
        hook.result.current.handleSelect(0);
      });
      const afterSelect = hook.result.current.revision;
      expect(afterSelect).toBeGreaterThan(start);

      // Editing a field must NOT bump revision (keeps the form mounted/focused).
      act(() => {
        hook.result.current.updateField("name", "Edited");
      });
      expect(hook.result.current.revision).toBe(afterSelect);

      act(() => {
        hook.result.current.handleAdd();
      });
      const afterAdd = hook.result.current.revision;
      expect(afterAdd).toBeGreaterThan(afterSelect);

      // undo/redo sync of the open record bumps it so the form remounts.
      act(() => {
        hook.result.current.handleSelect(1);
      });
      const beforeSync = hook.result.current.revision;
      act(() => {
        hook.result.current.syncEditing([
          { id: "a", name: "Alpha" },
          { id: "b", name: "Reverted" },
        ]);
      });
      expect(hook.result.current.revision).toBeGreaterThan(beforeSync);
      expect(hook.result.current.editing).toEqual({ id: "b", name: "Reverted" });
    });
  });

  describe("handleSelect", () => {
    it("sets editing to a copy of the item at index", () => {
      const items: Item[] = [
        { id: "a", name: "Alpha" },
        { id: "b", name: "Beta" },
      ];
      const { hook } = makeHook({ items });
      act(() => {
        hook.result.current.handleSelect(1);
      });
      expect(hook.result.current.editing).toEqual({ id: "b", name: "Beta" });
    });

    it("sets editingIndex to the given index", () => {
      const items: Item[] = [{ id: "x", name: "X" }];
      const { hook } = makeHook({ items });
      act(() => {
        hook.result.current.handleSelect(0);
      });
      expect(hook.result.current.editingIndex).toBe(0);
    });

    it("calls onOpen", () => {
      const items: Item[] = [{ id: "x", name: "X" }];
      const { hook, onOpen } = makeHook({ items });
      act(() => {
        hook.result.current.handleSelect(0);
      });
      expect(onOpen).toHaveBeenCalledTimes(1);
    });

    it("makes a shallow copy so mutations do not affect the original array", () => {
      const original: Item = { id: "x", name: "Original" };
      const { hook } = makeHook({ items: [original] });
      act(() => {
        hook.result.current.handleSelect(0);
      });
      expect(hook.result.current.editing).not.toBe(original);
    });
  });

  describe("handleAdd", () => {
    it("sets editing to an empty item", () => {
      const { hook } = makeHook();
      act(() => {
        hook.result.current.handleAdd();
      });
      expect(hook.result.current.editing).toEqual(emptyItem());
    });

    it("sets editingIndex to null", () => {
      const items: Item[] = [{ id: "x", name: "X" }];
      const { hook } = makeHook({ items });
      act(() => {
        hook.result.current.handleSelect(0);
      });
      act(() => {
        hook.result.current.handleAdd();
      });
      expect(hook.result.current.editingIndex).toBeNull();
    });

    it("calls onOpen", () => {
      const { hook, onOpen } = makeHook();
      act(() => {
        hook.result.current.handleAdd();
      });
      expect(onOpen).toHaveBeenCalledTimes(1);
    });
  });

  describe("updateField", () => {
    it("updates the named field in editing", () => {
      const { hook } = makeHook();
      act(() => {
        hook.result.current.updateField("name", "New Name");
      });
      expect(hook.result.current.editing.name).toBe("New Name");
    });

    it("does not replace other fields", () => {
      const { hook } = makeHook({ items: [{ id: "kept", name: "X" }] });
      act(() => {
        hook.result.current.handleSelect(0);
      });
      act(() => {
        hook.result.current.updateField("name", "Changed");
      });
      expect(hook.result.current.editing.id).toBe("kept");
    });
  });

  describe("handleDelete", () => {
    it("calls setItems with the item removed", () => {
      const items: Item[] = [
        { id: "a", name: "A" },
        { id: "b", name: "B" },
      ];
      const { hook, setItems } = makeHook({ items });
      act(() => {
        hook.result.current.handleDelete(0);
      });
      expect(setItems).toHaveBeenCalledWith([{ id: "b", name: "B" }]);
    });

    it("calls captureHistory before removing", () => {
      const items: Item[] = [{ id: "a", name: "A" }];
      const { hook, captureHistory } = makeHook({ items });
      act(() => {
        hook.result.current.handleDelete(0);
      });
      expect(captureHistory).toHaveBeenCalledTimes(1);
    });

    it("calls onClose when the deleted index matches editingIndex", () => {
      const items: Item[] = [{ id: "a", name: "A" }];
      const { hook, onClose } = makeHook({ items });
      act(() => {
        hook.result.current.handleSelect(0);
      });
      act(() => {
        hook.result.current.handleDelete(0);
      });
      expect(onClose).toHaveBeenCalledTimes(1);
    });

    it("does NOT call onClose when a different item is deleted", () => {
      const items: Item[] = [
        { id: "a", name: "A" },
        { id: "b", name: "B" },
      ];
      const { hook, onClose } = makeHook({ items });
      act(() => {
        hook.result.current.handleSelect(0);
      });
      act(() => {
        hook.result.current.handleDelete(1);
      });
      expect(onClose).not.toHaveBeenCalled();
    });

    it("decrements editingIndex when an item before it is deleted", () => {
      const items: Item[] = [
        { id: "a", name: "A" },
        { id: "b", name: "B" },
        { id: "c", name: "C" },
      ];
      const { hook, setItems, defaults } = makeHook({ items });
      act(() => {
        hook.result.current.handleSelect(2);
      }); // editing index 2 (C)
      act(() => {
        hook.result.current.handleDelete(1);
      }); // delete index 1 (B)
      // The page owns the array: feed the list the hook set back in, as the page re-renders it.
      hook.rerender({ ...defaults, items: setItems.mock.lastCall?.[0] as Item[] });
      expect(hook.result.current.editingIndex).toBe(1); // C is now at index 1
    });

    it("does NOT decrement editingIndex when an item after it is deleted", () => {
      const items: Item[] = [
        { id: "a", name: "A" },
        { id: "b", name: "B" },
        { id: "c", name: "C" },
      ];
      const { hook } = makeHook({ items });
      act(() => {
        hook.result.current.handleSelect(0);
      }); // editing index 0 (A)
      act(() => {
        hook.result.current.handleDelete(2);
      }); // delete index 2 (C)
      expect(hook.result.current.editingIndex).toBe(0); // A is still at index 0
    });

    it("calls saveItems when autoSave is true", async () => {
      const items: Item[] = [{ id: "a", name: "A" }];
      const { hook, saveItems } = makeHook({ items, autoSave: true });
      await act(async () => {
        hook.result.current.handleDelete(0);
      });
      expect(saveItems).toHaveBeenCalledTimes(1);
    });

    it("does NOT call saveItems when autoSave is false", async () => {
      const items: Item[] = [{ id: "a", name: "A" }];
      const { hook, saveItems } = makeHook({ items, autoSave: false });
      await act(async () => {
        hook.result.current.handleDelete(0);
      });
      expect(saveItems).not.toHaveBeenCalled();
    });
  });

  // Asset picks (team logo, player photo, competition badge) copy the image into
  // the package before setting the path, so the preview updates immediately. If
  // the path only lived in the editing buffer, switching entities dropped it and
  // the logo looked like it had vanished — see commitField.
  describe("commitField", () => {
    it("writes the value into the record, not just the editing buffer", () => {
      const items: Item[] = [
        { id: "a", name: "A" },
        { id: "b", name: "B" },
      ];
      const { hook, setItems } = makeHook({ items });
      act(() => {
        hook.result.current.handleSelect(0);
      });
      act(() => {
        hook.result.current.commitField("logo", "assets/images/a.png");
      });

      expect(hook.result.current.editing.logo).toBe("assets/images/a.png");
      expect(setItems).toHaveBeenCalledWith([
        { id: "a", name: "A", logo: "assets/images/a.png" },
        { id: "b", name: "B" },
      ]);
    });

    it("survives switching to another entity and back (the reported bug)", () => {
      const items: Item[] = [
        { id: "a", name: "A" },
        { id: "b", name: "B" },
      ];
      const { hook, setItems, defaults } = makeHook({ items });
      act(() => {
        hook.result.current.handleSelect(0);
      });
      act(() => {
        hook.result.current.commitField("logo", "assets/images/a.png");
      });

      // The page owns the array, so feed the committed list back in as the
      // parent would before the user clicks away.
      const updated = setItems.mock.calls[0][0] as Item[];
      hook.rerender({ ...defaults, items: updated });

      act(() => {
        hook.result.current.handleSelect(1);
      });
      expect(hook.result.current.editing.logo).toBeUndefined();

      act(() => {
        hook.result.current.handleSelect(0);
      });
      expect(hook.result.current.editing.logo).toBe("assets/images/a.png");
    });

    it("buffers without touching the list for a brand-new record", () => {
      const { hook, setItems, captureHistory, saveItems } = makeHook({ items: [], autoSave: true });
      act(() => {
        hook.result.current.handleAdd();
      });
      act(() => {
        hook.result.current.commitField("logo", "assets/images/new.png");
      });

      expect(hook.result.current.editing.logo).toBe("assets/images/new.png");
      expect(setItems).not.toHaveBeenCalled();
      expect(captureHistory).not.toHaveBeenCalled();
      expect(saveItems).not.toHaveBeenCalled();
    });

    it("persists once and captures a single history entry when autoSave is on", async () => {
      const items: Item[] = [{ id: "a", name: "A" }];
      const { hook, saveItems, captureHistory } = makeHook({ items, autoSave: true });
      act(() => {
        hook.result.current.handleSelect(0);
      });
      await act(async () => {
        hook.result.current.commitField("logo", "assets/images/a.png");
      });

      expect(captureHistory).toHaveBeenCalledTimes(1);
      expect(saveItems).toHaveBeenCalledTimes(1);
      expect(saveItems).toHaveBeenCalledWith([{ id: "a", name: "A", logo: "assets/images/a.png" }]);
    });

    it("flags the project dirty instead of persisting when autoSave is off", () => {
      const items: Item[] = [{ id: "a", name: "A" }];
      const { hook, saveItems, onDirty } = makeHook({ items, autoSave: false });
      act(() => {
        hook.result.current.handleSelect(0);
      });
      act(() => {
        hook.result.current.commitField("logo", "assets/images/a.png");
      });

      expect(saveItems).not.toHaveBeenCalled();
      expect(onDirty).toHaveBeenCalledTimes(1);
    });

    it("persists a cleared value the same way", () => {
      const items: Item[] = [{ id: "a", name: "A", logo: "assets/images/a.png" }];
      const { hook, setItems } = makeHook({ items });
      act(() => {
        hook.result.current.handleSelect(0);
      });
      act(() => {
        hook.result.current.commitField("logo", null);
      });

      expect(setItems).toHaveBeenCalledWith([{ id: "a", name: "A", logo: null }]);
    });

    it("writes to the record it was picked for, not the array slot it had", () => {
      // A pick awaits a native file dialog before committing, and the list can
      // be edited during that gap. `staleCommit` is deliberately captured
      // before the change: calling hook.result.current afterwards would take a
      // fresh closure and pass against the very bug this pins.
      const items: Item[] = [
        { id: "a", name: "A" },
        { id: "b", name: "B" },
      ];
      const { hook, setItems, defaults } = makeHook({ items });
      act(() => {
        hook.result.current.handleSelect(1);
      });
      const staleCommit = hook.result.current.commitField;

      // Meanwhile the user deletes the first entity, so "b" is now at index 0.
      const reordered: Item[] = [{ id: "b", name: "B" }];
      hook.rerender({ ...defaults, items: reordered });

      act(() => {
        staleCommit("logo", "assets/images/b.png");
      });

      expect(setItems).toHaveBeenCalledWith([{ id: "b", name: "B", logo: "assets/images/b.png" }]);
    });

    it("drops the commit when the record it was picked for is gone", () => {
      const items: Item[] = [
        { id: "a", name: "A" },
        { id: "b", name: "B" },
      ];
      const { hook, setItems, defaults, captureHistory } = makeHook({ items });
      act(() => {
        hook.result.current.handleSelect(1);
      });
      const staleCommit = hook.result.current.commitField;

      // An undo removed "b" while the file dialog was open. Re-adding it from a
      // stale commit would resurrect a record the user just discarded.
      hook.rerender({ ...defaults, items: [{ id: "a", name: "A" }] });
      setItems.mockClear();
      captureHistory.mockClear();

      act(() => {
        staleCommit("logo", "assets/images/b.png");
      });

      expect(setItems).not.toHaveBeenCalled();
      expect(captureHistory).not.toHaveBeenCalled();
    });

    it("keeps the buffer in sync when the id is renamed during the pick", () => {
      // Renaming the club while the copy is in flight is not switching away from
      // it. Matching the buffer on `id === editingId` failed here: the record got
      // the path, the buffer did not, and the next Save wrote the buffer back
      // over the record — losing the logo the user had just chosen.
      const items: Item[] = [{ id: "a", name: "A" }];
      const { hook } = makeHook({ items });
      act(() => {
        hook.result.current.handleSelect(0);
      });
      const commit = hook.result.current.commitField;

      act(() => {
        hook.result.current.updateField("id", "a-renamed");
      });
      act(() => {
        commit("logo", "assets/images/a.png");
      });

      expect(hook.result.current.editing).toEqual({
        id: "a-renamed",
        name: "A",
        logo: "assets/images/a.png",
      });
    });

    it("drops a pick from a new record the user has since abandoned", () => {
      // Add, start a pick, then go Back and Add again before the copy resolves.
      // The new-record branch has no id to re-locate by, so without a session
      // check the first record's asset lands on the second record's form.
      const { hook } = makeHook({ items: [] });
      act(() => {
        hook.result.current.handleAdd();
      });
      const staleCommit = hook.result.current.commitField;

      act(() => {
        hook.result.current.handleAdd();
      });
      act(() => {
        hook.result.current.updateField("name", "Second");
      });

      act(() => {
        staleCommit("logo", "assets/images/first.png");
      });

      expect(hook.result.current.editing).toEqual({ id: "", name: "Second" });
    });
  });

  describe("handleSave", () => {
    it("appends item to array when editingIndex is null (new item)", async () => {
      const items: Item[] = [{ id: "a", name: "A" }];
      const { hook, setItems } = makeHook({ items });
      act(() => {
        hook.result.current.handleAdd();
      });
      act(() => {
        hook.result.current.updateField("id", "b");
      });
      act(() => {
        hook.result.current.updateField("name", "B");
      });
      await act(async () => {
        await hook.result.current.handleSave();
      });
      expect(setItems).toHaveBeenCalledWith([
        { id: "a", name: "A" },
        { id: "b", name: "B" },
      ]);
    });

    it("replaces the item at editingIndex when editing an existing item", async () => {
      const items: Item[] = [
        { id: "a", name: "Old" },
        { id: "b", name: "B" },
      ];
      const { hook, setItems } = makeHook({ items });
      act(() => {
        hook.result.current.handleSelect(0);
      });
      act(() => {
        hook.result.current.updateField("name", "New");
      });
      await act(async () => {
        await hook.result.current.handleSave();
      });
      expect(setItems).toHaveBeenCalledWith([
        { id: "a", name: "New" },
        { id: "b", name: "B" },
      ]);
    });

    it("sets editingIndex to the newly appended index after adding", async () => {
      const items: Item[] = [{ id: "a", name: "A" }];
      const { hook, setItems, defaults } = makeHook({ items });
      act(() => {
        hook.result.current.handleAdd();
      });
      act(() => {
        hook.result.current.updateField("id", "b");
      });
      await act(async () => {
        await hook.result.current.handleSave();
      });
      // The page owns the array: feed the list the hook set back in, as the page re-renders it.
      hook.rerender({ ...defaults, items: setItems.mock.lastCall?.[0] as Item[] });
      expect(hook.result.current.editingIndex).toBe(1);
    });

    it("keeps editingIndex the same after editing", async () => {
      const items: Item[] = [
        { id: "a", name: "A" },
        { id: "b", name: "B" },
      ];
      const { hook, setItems, defaults } = makeHook({ items });
      act(() => {
        hook.result.current.handleSelect(1);
      });
      await act(async () => {
        await hook.result.current.handleSave();
      });
      // The page owns the array: feed the list the hook set back in, as the page re-renders it.
      hook.rerender({ ...defaults, items: setItems.mock.lastCall?.[0] as Item[] });
      expect(hook.result.current.editingIndex).toBe(1);
    });

    it("calls captureHistory before saving", async () => {
      const { hook, captureHistory } = makeHook();
      await act(async () => {
        await hook.result.current.handleSave();
      });
      expect(captureHistory).toHaveBeenCalledTimes(1);
    });

    it("calls saveItems with updated array when autoSave is true", async () => {
      const items: Item[] = [{ id: "a", name: "A" }];
      const { hook, saveItems } = makeHook({ items, autoSave: true });
      act(() => {
        hook.result.current.handleAdd();
      });
      act(() => {
        hook.result.current.updateField("id", "b");
      });
      await act(async () => {
        await hook.result.current.handleSave();
      });
      expect(saveItems).toHaveBeenCalledWith([
        { id: "a", name: "A" },
        { id: "b", name: "" }, // name not updated but id is
      ]);
    });

    it("does NOT call saveItems when autoSave is false", async () => {
      const { hook, saveItems } = makeHook({ autoSave: false });
      await act(async () => {
        await hook.result.current.handleSave();
      });
      expect(saveItems).not.toHaveBeenCalled();
    });

    it("sets isBusy true then false during autoSave", async () => {
      const { hook, setIsBusy } = makeHook({ autoSave: true });
      await act(async () => {
        await hook.result.current.handleSave();
      });
      expect(setIsBusy).toHaveBeenCalledWith(true);
      expect(setIsBusy).toHaveBeenLastCalledWith(false);
    });
  });

  describe("syncEditing", () => {
    it("refreshes editing from newItems at the current editingIndex", () => {
      const items: Item[] = [{ id: "a", name: "Old" }];
      const { hook } = makeHook({ items });
      act(() => {
        hook.result.current.handleSelect(0);
      });
      act(() => {
        hook.result.current.syncEditing([{ id: "a", name: "Restored" }]);
      });
      expect(hook.result.current.editing).toEqual({ id: "a", name: "Restored" });
    });

    it("is a no-op when editingIndex is null", () => {
      const { hook } = makeHook();
      act(() => {
        hook.result.current.syncEditing([{ id: "a", name: "X" }]);
      });
      expect(hook.result.current.editing).toEqual(emptyItem());
    });

    it("re-locates the edited record by id after an undo reorders the list", () => {
      const items: Item[] = [
        { id: "a", name: "A" },
        { id: "b", name: "B" },
        { id: "c", name: "C" },
      ];
      const { hook, defaults } = makeHook({ items });
      act(() => {
        hook.result.current.handleSelect(2);
      }); // editing "c" at index 2

      // A restored snapshot where the same records are reordered: "c" is now first.
      // Index 2 would point at a *different* record, so tracking must be by id.
      const reordered: Item[] = [
        { id: "c", name: "C-reverted" },
        { id: "a", name: "A" },
        { id: "b", name: "B" },
      ];
      act(() => {
        hook.result.current.syncEditing(reordered);
      });
      // The page applies the restored snapshot as the list, then the hook reports against it.
      hook.rerender({ ...defaults, items: reordered });

      expect(hook.result.current.editing).toEqual({ id: "c", name: "C-reverted" });
      expect(hook.result.current.editingIndex).toBe(0);
    });

    it("closes the editor when editingIndex is out of bounds in newItems", () => {
      const items: Item[] = [
        { id: "a", name: "A" },
        { id: "b", name: "B" },
      ];
      const { hook, onClose } = makeHook({ items });
      act(() => {
        hook.result.current.handleSelect(1);
      });
      // An undo that removed the record at index 1. Without closing, a later save
      // would re-add the record the user just reverted.
      act(() => {
        hook.result.current.syncEditing([{ id: "a", name: "A" }]);
      });
      expect(onClose).toHaveBeenCalledTimes(1);
    });
  });

  describe("handleDuplicate", () => {
    const items: Item[] = [
      { id: "alpha", name: "Alpha" },
      { id: "beta", name: "Beta" },
    ];

    it("inserts the copy directly after its source", () => {
      const { hook, setItems } = makeHook({ items });
      act(() => {
        hook.result.current.handleDuplicate(0);
      });
      expect(setItems).toHaveBeenCalledWith([
        { id: "alpha", name: "Alpha" },
        { id: "alpha-2", name: "Alpha" },
        { id: "beta", name: "Beta" },
      ]);
    });

    it("opens the copy so it can be renamed straight away", () => {
      const { hook, onOpen, setItems, defaults } = makeHook({ items });
      act(() => {
        hook.result.current.handleDuplicate(0);
      });
      // The page owns the array: feed the list the hook set back in, as the page re-renders it.
      hook.rerender({ ...defaults, items: setItems.mock.lastCall?.[0] as Item[] });
      expect(hook.result.current.editing).toEqual({ id: "alpha-2", name: "Alpha" });
      expect(hook.result.current.editingIndex).toBe(1);
      expect(onOpen).toHaveBeenCalledTimes(1);
    });

    it("captures history so the copy can be undone", () => {
      const { hook, captureHistory } = makeHook({ items });
      act(() => {
        hook.result.current.handleDuplicate(1);
      });
      expect(captureHistory).toHaveBeenCalledTimes(1);
    });

    it("skips an id already in use instead of colliding", () => {
      const taken: Item[] = [
        { id: "alpha", name: "Alpha" },
        { id: "alpha-2", name: "Copy" },
      ];
      const { hook, setItems } = makeHook({ items: taken });
      act(() => {
        hook.result.current.handleDuplicate(0);
      });
      expect(setItems.mock.calls[0][0][1].id).toBe("alpha-3");
    });

    it("counts up from the stem rather than chaining suffixes", () => {
      const { hook, setItems } = makeHook({ items: [{ id: "alpha-2", name: "Alpha" }] });
      act(() => {
        hook.result.current.handleDuplicate(0);
      });
      expect(setItems.mock.calls[0][0][1].id).toBe("alpha-3");
    });

    it("mints a distinct id when the source id is blank", () => {
      // A blank id normally means "derive from the name on save", but a copy
      // needs an id of its own: two blank-id siblings are indistinguishable to
      // syncEditing's findIndex, so an undo would re-bind the form to the
      // original. The copy gets a real id and the source stays blank.
      const { hook, setItems, defaults } = makeHook({ items: [{ id: "", name: "Alpha" }] });
      act(() => {
        hook.result.current.handleDuplicate(0);
      });
      const [source, copy] = setItems.mock.calls[0][0];
      expect(source.id).toBe("");
      expect(copy.id).toBe("copy-2");
      // The page owns the array: feed the list the hook set back in, as the page re-renders it.
      hook.rerender({ ...defaults, items: setItems.mock.calls[0][0] as Item[] });
      expect(hook.result.current.editingIndex).toBe(1);
    });

    it("deep-copies so editing the copy can't mutate the source", () => {
      type Nested = { id: string; colors: { primary: string } };
      const source: Nested[] = [{ id: "a", colors: { primary: "#fff" } }];
      const setItems = vi.fn();
      const hook = renderHook(() =>
        useEntityEditor<Nested>({
          items: source,
          setItems,
          empty: () => ({ id: "", colors: { primary: "" } }),
          captureHistory: vi.fn(),
          saveItems: vi.fn().mockResolvedValue(undefined),
          autoSave: false,
          onOpen: vi.fn(),
          onClose: vi.fn(),
          setIsBusy: vi.fn(),
        }),
      );
      act(() => {
        hook.result.current.handleDuplicate(0);
      });
      const copy = setItems.mock.calls[0][0][1] as Nested;
      copy.colors.primary = "#000";
      expect(source[0].colors.primary).toBe("#fff");
    });

    it("saves immediately when auto-save is on", () => {
      const { hook, saveItems } = makeHook({ items, autoSave: true });
      act(() => {
        hook.result.current.handleDuplicate(0);
      });
      expect(saveItems).toHaveBeenCalledTimes(1);
    });

    it("ignores an out-of-range index", () => {
      const { hook, setItems, captureHistory } = makeHook({ items });
      act(() => {
        hook.result.current.handleDuplicate(99);
      });
      expect(setItems).not.toHaveBeenCalled();
      expect(captureHistory).not.toHaveBeenCalled();
    });
  });
});

// The World Editor runs two editors over one list: the first-team and youth player editors both
// edit `players`, each by its index into the whole array. A row one inserts or removes moves the
// other's record, so an editor must find its record by id, not by the position it last saw.
describe("two editors over one list", () => {
  type Editor = ReturnType<typeof useEntityEditor<Item>>;
  type Shared = { items: Item[]; first: Editor; youth: Editor };

  function renderTwoEditors(initial: Item[]) {
    return renderHook((): Shared => {
      const [items, setItems] = useState(initial);
      const options = {
        items,
        setItems,
        empty: emptyItem,
        captureHistory: vi.fn(),
        saveItems: vi.fn().mockResolvedValue(undefined),
        autoSave: false,
        onOpen: vi.fn(),
        onClose: vi.fn(),
        setIsBusy: vi.fn(),
      };
      return { items, first: useEntityEditor(options), youth: useEntityEditor(options) };
    });
  }

  const youthAndOne = (): Item[] => [
    { id: "y0", name: "Youth Zero" },
    { id: "y1", name: "Youth One" },
    { id: "p", name: "First Teamer" },
  ];

  // Given a first-team player open in one editor, and a youth before him removed in the other
  // When the first editor saves its edit
  // Then the edit lands on that player, and the player who moved into his old place is kept
  it("saves an edit onto its own record after the other editor removed a row before it", () => {
    const { result } = renderTwoEditors(youthAndOne());
    act(() => result.current.first.handleSelect(2));
    act(() => result.current.first.updateField("name", "First Teamer Edited"));
    act(() => result.current.youth.handleDelete(0));
    act(() => {
      void result.current.first.handleSave();
    });

    expect(result.current.items).toEqual([
      { id: "y1", name: "Youth One" },
      { id: "p", name: "First Teamer Edited" },
    ]);
  });

  // Given a first-team player open in one editor, and a youth before him duplicated in the other
  // When the first editor saves its edit
  // Then the edit lands on that player, and neither the youth player nor his copy is overwritten
  it("saves an edit onto its own record after the other editor inserted a row before it", () => {
    const { result } = renderTwoEditors(youthAndOne());
    act(() => result.current.first.handleSelect(2));
    act(() => result.current.first.updateField("name", "First Teamer Edited"));
    act(() => result.current.youth.handleDuplicate(0));
    act(() => {
      void result.current.first.handleSave();
    });

    const ids = result.current.items.map((item) => item.id);
    expect(ids.filter((id) => id === "p")).toHaveLength(1);
    expect(result.current.items.find((item) => item.id === "p")?.name).toBe("First Teamer Edited");
    expect(result.current.items.slice(0, 3).map((item) => item.name)).toEqual([
      "Youth Zero",
      "Youth Zero",
      "Youth One",
    ]);
  });

  // Given two players sharing one id, the second of them open in an editor
  // When that editor saves its edit
  // Then the edit lands on the second, and the first keeps his own data: a duplicate id must not
  //      send the save to its twin
  it("saves onto the open one of two records sharing an id", () => {
    const { result } = renderTwoEditors([
      { id: "x", name: "First X" },
      { id: "y", name: "Why" },
      { id: "x", name: "Second X" },
    ]);
    act(() => result.current.first.handleSelect(2));
    act(() => result.current.first.updateField("name", "Second X Edited"));
    act(() => {
      void result.current.first.handleSave();
    });

    expect(result.current.items.map((item) => item.name)).toEqual([
      "First X",
      "Why",
      "Second X Edited",
    ]);
  });

  // Given an unnamed first-team player (blank id) open in one editor, and a youth before him
  //      removed in the other
  // When the first editor saves its edit
  // Then the edit lands on the unnamed player, not on whoever took his old row
  it("saves an unnamed record onto itself after the other editor removed a row before it", () => {
    const { result } = renderTwoEditors([
      { id: "y0", name: "Youth Zero" },
      { id: "y1", name: "Youth One" },
      { id: "", name: "Unnamed" },
    ]);
    act(() => result.current.first.handleSelect(2));
    act(() => result.current.first.updateField("name", "Unnamed Edited"));
    act(() => result.current.youth.handleDelete(0));
    act(() => {
      void result.current.first.handleSave();
    });

    expect(result.current.items).toEqual([
      { id: "y1", name: "Youth One" },
      { id: "", name: "Unnamed Edited" },
    ]);
  });

  // Given a first-team player open in one editor, whose row the other editor deletes
  // When the list asks which row is open, and then the first editor saves
  // Then no row is open in between (the form stays, the highlight goes), and the save adds the
  //      record back as a new one rather than writing it over another
  it("saves a record the other editor removed as a new one", () => {
    const { result } = renderTwoEditors(youthAndOne());
    act(() => result.current.first.handleSelect(2));
    act(() => result.current.first.updateField("name", "First Teamer Edited"));
    act(() => result.current.youth.handleDelete(2));

    expect(result.current.first.editingIndex).toBeNull();

    act(() => {
      void result.current.first.handleSave();
    });

    expect(result.current.items).toEqual([
      { id: "y0", name: "Youth Zero" },
      { id: "y1", name: "Youth One" },
      { id: "p", name: "First Teamer Edited" },
    ]);
  });

  // Given two rows sharing an id, the second open, and its twin left after the other editor deletes
  //      the open one
  // When the first editor saves its edit
  // Then the edit is added as a new record: the twin that shares its id keeps its own data
  it("appends the edit when the other editor deletes the open row and a same-id row remains", () => {
    const { result } = renderTwoEditors([
      { id: "x", name: "First X" },
      { id: "x", name: "Second X" },
    ]);
    act(() => result.current.first.handleSelect(1));
    act(() => result.current.first.updateField("name", "Second X Edited"));
    act(() => result.current.youth.handleDelete(1));
    act(() => {
      void result.current.first.handleSave();
    });

    expect(result.current.items).toEqual([
      { id: "x", name: "First X" },
      { id: "x", name: "Second X Edited" },
    ]);
  });

  // Given the second of two rows sharing an id open, an asset pick started, and a youth before
  //      both removed by the other editor while the pick's dialog was open
  // When the pick commits
  // Then the asset lands on the record the pick began from, not on its twin
  it("commits a picked asset onto the record the pick began from after rows shifted", () => {
    const { result } = renderTwoEditors([
      { id: "y0", name: "Youth Zero" },
      { id: "x", name: "First X" },
      { id: "x", name: "Second X" },
    ]);
    act(() => result.current.first.handleSelect(2));
    const commit = result.current.first.commitField;
    act(() => result.current.youth.handleDelete(0));
    act(() => commit("logo", "assets/images/team/x.png"));

    expect(result.current.items).toEqual([
      { id: "x", name: "First X" },
      { id: "x", name: "Second X", logo: "assets/images/team/x.png" },
    ]);
  });

  // Given a record open, an asset pick started, and the record saved while the pick's dialog was
  //      open
  // When the pick commits
  // Then the asset lands on the saved record: the save replaced the row the pick began from
  it("keeps a picked asset when the record was saved before the pick finished", () => {
    const { result } = renderTwoEditors(youthAndOne());
    act(() => result.current.first.handleSelect(2));
    const commit = result.current.first.commitField;
    act(() => result.current.first.updateField("name", "First Teamer Edited"));
    act(() => {
      void result.current.first.handleSave();
    });
    act(() => commit("logo", "assets/images/player/p.png"));

    expect(result.current.items[2]).toEqual({
      id: "p",
      name: "First Teamer Edited",
      logo: "assets/images/player/p.png",
    });
  });

  // Given a first-team player open in one editor, and a youth before him removed in the other
  // When the list asks which row is open
  // Then it is the open player's new row, so the list highlights the record the form shows
  it("reports its record's current row after the other editor removed a row before it", () => {
    const { result } = renderTwoEditors(youthAndOne());
    act(() => result.current.first.handleSelect(2));
    act(() => result.current.youth.handleDelete(0));

    expect(result.current.first.editingIndex).toBe(1);
  });

  // Given a first-team player open in one editor, and a youth before him removed in the other
  // When the first editor deletes the row now holding a different player
  // Then the open player's form stays open: it is not the row that was deleted
  it("keeps its form open when the other row at its old position is deleted", () => {
    const onClose = vi.fn();
    const { result } = renderHook(() => {
      const [items, setItems] = useState(youthAndOne());
      const options = {
        items,
        setItems,
        empty: emptyItem,
        captureHistory: vi.fn(),
        saveItems: vi.fn().mockResolvedValue(undefined),
        autoSave: false,
        onOpen: vi.fn(),
        onClose,
        setIsBusy: vi.fn(),
      };
      return { items, first: useEntityEditor(options), youth: useEntityEditor(options) };
    });
    act(() => result.current.first.handleSelect(1));
    act(() => result.current.youth.handleDelete(0));
    act(() => result.current.first.handleDelete(1));

    expect(onClose).not.toHaveBeenCalled();
    expect(result.current.items.map((item) => item.id)).toEqual(["y1"]);
  });
});
