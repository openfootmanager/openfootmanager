import { useRef, useState, useEffect } from "react";

const MAX_HISTORY = 50;

export function useUndoRedo<S>(options: {
  getSnapshot: () => S;
  applySnapshot: (snapshot: S) => void;
  enabled: boolean;
  onDirty?: () => void;
}) {
  const { getSnapshot, applySnapshot, enabled, onDirty } = options;

  const undoStack = useRef<S[]>([]);
  const redoStack = useRef<S[]>([]);
  const [canUndo, setCanUndo] = useState(false);
  const [canRedo, setCanRedo] = useState(false);

  function pushHistory(snapshot: S) {
    undoStack.current = [...undoStack.current.slice(-MAX_HISTORY + 1), snapshot];
    redoStack.current = [];
    setCanUndo(true);
    setCanRedo(false);
    onDirty?.();
  }

  function clearHistory() {
    undoStack.current = [];
    redoStack.current = [];
    setCanUndo(false);
    setCanRedo(false);
  }

  function handleUndo() {
    if (undoStack.current.length === 0) return;
    const prev = undoStack.current[undoStack.current.length - 1];
    undoStack.current = undoStack.current.slice(0, -1);
    redoStack.current = [getSnapshot(), ...redoStack.current];
    applySnapshot(prev);
    setCanUndo(undoStack.current.length > 0);
    setCanRedo(true);
    onDirty?.();
  }

  function handleRedo() {
    if (redoStack.current.length === 0) return;
    const next = redoStack.current[0];
    redoStack.current = redoStack.current.slice(1);
    undoStack.current = [...undoStack.current, getSnapshot()];
    applySnapshot(next);
    setCanUndo(true);
    setCanRedo(redoStack.current.length > 0);
    onDirty?.();
  }

  // no dep array — re-binds every render so closures stay fresh
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (!enabled) return;
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;
      if ((e.ctrlKey || e.metaKey) && e.key === "z" && !e.shiftKey) {
        e.preventDefault();
        handleUndo();
      }
      if ((e.ctrlKey || e.metaKey) && (e.key === "y" || (e.key === "z" && e.shiftKey))) {
        e.preventDefault();
        handleRedo();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  return { canUndo, canRedo, pushHistory, clearHistory, handleUndo, handleRedo };
}
