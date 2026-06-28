import { useState } from "react";
import type { NamePool, NamesDefinition } from "../components/menu/PackageEditor/types";

interface UseNamesPoolEditorOptions {
  names: NamesDefinition;
  setNames: (n: NamesDefinition) => void;
  autoSave: boolean;
  captureHistory: () => void;
  saveNames: (names: NamesDefinition) => Promise<void>;
  onOpen: () => void;
  onClose: () => void;
  setIsBusy: (busy: boolean) => void;
}

export function useNamesPoolEditor({
  names,
  setNames,
  autoSave,
  captureHistory,
  saveNames,
  onOpen,
  onClose,
  setIsBusy,
}: UseNamesPoolEditorOptions) {
  const [editingPoolKey, setEditingPoolKey] = useState("");
  const [editingPool, setEditingPool] = useState<NamePool>({ first_names: [], last_names: [] });
  const [isNewPool, setIsNewPool] = useState(false);
  // Bumped whenever the editing buffer is replaced (select / add / undo-redo
  // sync); used as a React `key` so the pool form remounts with fresh state.
  const [revision, setRevision] = useState(0);

  function handleSelectPool(key: string) {
    setEditingPoolKey(key);
    setEditingPool({ ...names.pools[key] });
    setIsNewPool(false);
    setRevision((r) => r + 1);
    onOpen();
  }

  function handleAddPool() {
    setEditingPoolKey("");
    setEditingPool({ first_names: [], last_names: [] });
    setIsNewPool(true);
    setRevision((r) => r + 1);
    onOpen();
  }

  function handleDeletePool(key: string) {
    captureHistory();
    const updated: NamesDefinition = {
      ...names,
      pools: Object.fromEntries(Object.entries(names.pools).filter(([k]) => k !== key)),
    };
    setNames(updated);
    if (autoSave) void saveNames(updated).catch(() => { /* persist already showed the error */ });
    if (editingPoolKey === key) onClose();
  }

  /// Refresh the open pool buffer from a restored snapshot (undo/redo) so a save
  /// can't reapply values the user just reverted.
  function syncEditing(newNames: NamesDefinition) {
    if (!editingPoolKey) return;
    if (newNames.pools[editingPoolKey]) {
      setEditingPool({ ...newNames.pools[editingPoolKey] });
      setRevision((r) => r + 1);
    } else {
      // The restored snapshot no longer contains the key being edited (e.g. an
      // undo that removed it). Close the editor so a save can't recreate it.
      onClose();
    }
  }

  async function handleSavePool(key: string, pool: NamePool) {
    // Reject a rename/new key that collides with a *different* existing pool;
    // Object.fromEntries would otherwise silently drop one and lose its names.
    const collidesWithOther = isNewPool
      ? key in names.pools
      : key !== editingPoolKey && key in names.pools;
    if (collidesWithOther) {
      return;
    }
    captureHistory();
    const updatedPools = isNewPool
      ? { ...names.pools, [key]: pool }
      : Object.fromEntries(
          Object.entries(names.pools).map(([k, v]) =>
            k === editingPoolKey ? [key, pool] : [k, v],
          ),
        );
    const updated: NamesDefinition = { ...names, pools: updatedPools };
    setNames(updated);
    setEditingPoolKey(key);
    setIsNewPool(false);
    if (autoSave) {
      setIsBusy(true);
      try {
        await saveNames(updated);
      } catch {
        // non-fatal
      } finally {
        setIsBusy(false);
      }
    }
  }

  return {
    editingPoolKey,
    editingPool,
    isNewPool,
    revision,
    handleSelectPool,
    handleAddPool,
    handleDeletePool,
    handleSavePool,
    syncEditing,
  };
}
