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

  function updateField<K extends keyof T>(key: K, value: T[K]) {
    setEditing((prev) => ({ ...prev, [key]: value }));
  }

  function handleSelect(index: number) {
    setEditing({ ...items[index] });
    setEditingIndex(index);
    onOpen();
  }

  function handleAdd() {
    setEditing(empty());
    setEditingIndex(null);
    onOpen();
  }

  function handleDelete(index: number) {
    captureHistory();
    const updated = items.filter((_, i) => i !== index);
    setItems(updated);
    if (autoSave) void saveItems(updated);
    if (editingIndex === index) {
      onClose();
    } else if (editingIndex !== null && index < editingIndex) {
      setEditingIndex(editingIndex - 1);
    }
  }

  function syncEditing(newItems: T[]) {
    if (editingIndex !== null && editingIndex < newItems.length) {
      setEditing({ ...newItems[editingIndex] });
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

  return { editing, editingIndex, setEditing, updateField, handleSelect, handleAdd, handleDelete, handleSave, syncEditing };
}
