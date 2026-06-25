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

  function handleSelectPool(key: string) {
    setEditingPoolKey(key);
    setEditingPool({ ...names.pools[key] });
    setIsNewPool(false);
    onOpen();
  }

  function handleAddPool() {
    setEditingPoolKey("");
    setEditingPool({ first_names: [], last_names: [] });
    setIsNewPool(true);
    onOpen();
  }

  function handleDeletePool(key: string) {
    captureHistory();
    const updated: NamesDefinition = {
      ...names,
      pools: Object.fromEntries(Object.entries(names.pools).filter(([k]) => k !== key)),
    };
    setNames(updated);
    if (autoSave) void saveNames(updated);
    if (editingPoolKey === key) onClose();
  }

  async function handleSavePool(key: string, pool: NamePool) {
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
    handleSelectPool,
    handleAddPool,
    handleDeletePool,
    handleSavePool,
  };
}
