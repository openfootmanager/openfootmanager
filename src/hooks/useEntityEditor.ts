import { useState } from "react";

export function useEntityEditor<T>(options: {
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
    setRevision((r) => r + 1);
    onOpen();
  }

  function handleAdd() {
    setEditing(empty());
    setEditingIndex(null);
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
    if (editingIndex !== null && editingIndex < newItems.length) {
      setEditing({ ...newItems[editingIndex] });
      setRevision((r) => r + 1);
    }
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
