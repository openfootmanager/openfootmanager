import { useState } from "react";

export function useEntityEditor<T extends { id: string }>(options: {
  items: T[];
  setItems: (items: T[]) => void;
  empty: () => T;
  captureHistory: () => void;
  saveItems: (items: T[]) => Promise<void>;
  autoSave: boolean;
  onOpen: () => void;
  onClose: () => void;
  setIsBusy: (busy: boolean) => void;
}) {
  const { items, setItems, empty, captureHistory, saveItems, autoSave, onOpen, onClose, setIsBusy } =
    options;

  const [editing, setEditing] = useState<T>(empty);
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  // The id of the record being edited, captured when it is selected/saved and
  // left untouched by field edits. syncEditing uses it to re-locate the record
  // after an undo/redo that may have reordered the list, so the form can't end
  // up bound to a different record that happens to share the old array index.
  const [editingId, setEditingId] = useState<string | null>(null);
  // Bumped only when the whole editing buffer is replaced (select / add / undo-
  // redo sync), never on a field edit. Consumers use it as a React `key` so the
  // form remounts and re-derives local state on a buffer swap, but keeps its
  // state (and focus) while the user types.
  const [revision, setRevision] = useState(0);

  function updateField<K extends keyof T>(key: K, value: T[K]) {
    setEditing((prev) => ({ ...prev, [key]: value }));
  }

  function handleSelect(index: number) {
    setEditing({ ...items[index] });
    setEditingIndex(index);
    setEditingId(items[index].id);
    setRevision((r) => r + 1);
    onOpen();
  }

  function handleAdd() {
    setEditing(empty());
    setEditingIndex(null);
    setEditingId(null);
    setRevision((r) => r + 1);
    onOpen();
  }

  function handleDelete(index: number) {
    captureHistory();
    const updated = items.filter((_, i) => i !== index);
    setItems(updated);
    if (autoSave) void saveItems(updated).catch(() => { /* persist already showed the error */ });
    if (editingIndex === index) {
      onClose();
    } else if (editingIndex !== null && index < editingIndex) {
      setEditingIndex(editingIndex - 1);
    }
  }

  function syncEditing(newItems: T[]) {
    // editingIndex === null means a brand-new, unsaved record (no id to track);
    // there is nothing in the restored snapshot to reconcile against.
    if (editingIndex === null) return;
    // Re-locate the edited record by identity, not by array position: an undo/
    // redo can reorder the list, so newItems[editingIndex] may be a different
    // record. Find it by id and resync both the buffer and its (possibly moved)
    // index.
    const idx = newItems.findIndex((item) => item.id === editingId);
    if (idx === -1) {
      // The restored snapshot no longer contains the record (e.g. an undo that
      // removed it). Close so a save can't re-add it.
      onClose();
      return;
    }
    setEditing({ ...newItems[idx] });
    setEditingIndex(idx);
    setRevision((r) => r + 1);
  }

  async function handleSave() {
    captureHistory();
    const updated =
      editingIndex === null
        ? [...items, editing]
        : items.map((item, i) => (i === editingIndex ? editing : item));
    const newIndex = editingIndex ?? updated.length - 1;
    setItems(updated);
    setEditingIndex(newIndex);
    // Track the saved record's id so a later undo/redo can re-locate it — covers
    // a newly appended record and an in-place id rename.
    setEditingId(editing.id);
    if (autoSave) {
      setIsBusy(true);
      try {
        await saveItems(updated);
      } catch {
        // non-fatal — persist already showed the error
      } finally {
        setIsBusy(false);
      }
    }
  }

  return { editing, editingIndex, revision, setEditing, updateField, handleSelect, handleAdd, handleDelete, handleSave, syncEditing };
}
