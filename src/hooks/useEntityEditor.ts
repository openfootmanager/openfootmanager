import { useRef, useState } from "react";

/**
 * Pick an unused id for a copy of `baseId`, counting up from `-2`.
 *
 * An existing numeric suffix is stripped first so duplicating a duplicate gives
 * `team-3` rather than chaining into `team-2-2`. An empty id is left empty —
 * those are auto-generated from the entity name on save.
 */
export function uniqueEntityId(baseId: string, taken: Set<string>): string {
  if (!baseId) return "";
  const stem = baseId.replace(/-\d+$/, "") || baseId;
  for (let n = 2; ; n += 1) {
    const candidate = `${stem}-${n}`;
    if (!taken.has(candidate)) return candidate;
  }
}

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
  /**
   * Build the copy made by `handleDuplicate`, given the source entity and an
   * id that is already unique. Defaults to a deep copy carrying the new id;
   * override to also disambiguate a display name.
   */
  cloneItem?: (item: T, id: string) => T;
  onDirty?: () => void;
}) {
  const { items, setItems, empty, captureHistory, saveItems, autoSave, onOpen, onClose, setIsBusy, onDirty } =
    options;
  const cloneItem = options.cloneItem ?? ((item: T, id: string) => ({ ...structuredClone(item), id }));

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

  // commitField can run long after the render that created it — an asset pick
  // awaits a native file dialog first — so it must not write through the list
  // it saw back then. Anything the user did during the dialog would be undone.
  const latest = useRef({ items, setItems, captureHistory, saveItems, autoSave, onDirty });
  latest.current = { items, setItems, captureHistory, saveItems, autoSave, onDirty };

  /**
   * Like `updateField`, but also writes the value straight into the record so it
   * survives switching to another entity without pressing Save.
   *
   * Used for fields whose edit already has a side effect outside the form — an
   * asset pick copies the image into the package before this runs, so the
   * preview updating while the path stays unpersisted reads as a lost edit.
   * A brand-new record (`editingIndex === null`) has no row to write into yet,
   * so it keeps the buffer-until-Save behaviour.
   *
   * The target is the record that was open when the commit was set in motion,
   * re-located by id in the *current* list — the same identity-over-position
   * rule `syncEditing` follows, and for the same reason: between the pick and
   * the commit the list may have been reordered, or the record removed.
   */
  function commitField<K extends keyof T>(key: K, value: T[K]) {
    if (editingIndex === null) {
      setEditing((prev) => ({ ...prev, [key]: value }));
      return;
    }
    const { items, setItems, captureHistory, saveItems, autoSave, onDirty } = latest.current;
    // A blank id cannot be told apart from another blank one, so an unnamed
    // record falls back to the position it had when the commit started — no
    // worse than before, and it keeps a not-yet-named entity working.
    const index = editingId
      ? items.findIndex((item) => item.id === editingId)
      : editingIndex;
    // The record is gone (an undo discarded it). Writing it back would
    // resurrect something the user just removed.
    if (index === -1 || index >= items.length) return;
    setEditing((prev) => (editingId && prev.id !== editingId ? prev : { ...prev, [key]: value }));
    captureHistory();
    const updated = items.map((item, i) => (i === index ? { ...item, [key]: value } : item));
    setItems(updated);
    onDirty?.();
    if (autoSave) void saveItems(updated).catch(() => { /* persist already showed the error */ });
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

  /**
   * Insert a copy of `items[index]` directly after it and open the copy for
   * editing. Landing next to the original keeps a long list navigable, and
   * opening it means the user can immediately give the copy a real name — the
   * generated id is unique but rarely what they want to keep.
   */
  function handleDuplicate(index: number) {
    const source = items[index];
    if (!source) return;
    captureHistory();
    const taken = new Set(items.map((item) => item.id));
    // A copy needs an id distinct from its source so syncEditing can re-locate
    // it by identity after an undo/redo. When the source id is blank (a not-yet-
    // named entity) fall back to a generic stem rather than minting a second
    // blank id that findIndex can't tell apart from the original.
    const clone = cloneItem(source, uniqueEntityId(source.id || "copy", taken));
    const updated = [...items.slice(0, index + 1), clone, ...items.slice(index + 1)];
    setItems(updated);
    if (autoSave) void saveItems(updated).catch(() => { /* persist already showed the error */ });
    setEditing({ ...clone });
    setEditingIndex(index + 1);
    setEditingId(clone.id);
    setRevision((r) => r + 1);
    onOpen();
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

  return { editing, editingIndex, revision, setEditing, updateField, commitField, handleSelect, handleAdd, handleDelete, handleDuplicate, handleSave, syncEditing };
}
