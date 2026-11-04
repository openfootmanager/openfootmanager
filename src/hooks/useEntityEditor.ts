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
  const {
    items,
    setItems,
    empty,
    captureHistory,
    saveItems,
    autoSave,
    onOpen,
    onClose,
    setIsBusy,
    onDirty,
  } = options;
  const cloneItem =
    options.cloneItem ?? ((item: T, id: string) => ({ ...structuredClone(item), id }));

  const [editing, setEditing] = useState<T>(empty);
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  // The id of the record being edited, captured when it is selected/saved and
  // left untouched by field edits. syncEditing uses it to re-locate the record
  // after an undo/redo that may have reordered the list, so the form can't end
  // up bound to a different record that happens to share the old array index.
  const [editingId, setEditingId] = useState<string | null>(null);
  // The row this editor last opened or wrote, by object identity. A row nobody else touched keeps
  // its identity through the other editor's inserts and deletes (the first-team and youth player
  // editors share one list), so this finds the open record exactly whatever its id: a duplicate id,
  // or a blank one, is told apart from its twin (#1197).
  const openRow = useRef<T | null>(null);
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
  const latest = useRef({
    items,
    setItems,
    captureHistory,
    saveItems,
    autoSave,
    onDirty,
    revision,
  });
  latest.current = { items, setItems, captureHistory, saveItems, autoSave, onDirty, revision };

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
  // The open row as of this render: a `commitField` from this render targets it.
  const rowAtRender = openRow.current;

  function commitField<K extends keyof T>(key: K, value: T[K]) {
    // Whether the form still holds the record this pick was started from.
    // `revision` bumps on select/add/duplicate/undo-sync — every way the open
    // record can change — and deliberately not on an ordinary field edit, so
    // renaming the entity mid-pick does not count as switching away from it.
    const sameSession = revision === latest.current.revision;
    if (editingIndex === null) {
      // A new record has no id to re-locate by, so the session is all there is.
      // Without this, abandoning an unsaved record mid-pick and starting another
      // grafts the first record's asset onto the second one's form.
      if (!sameSession) return;
      setEditing((prev) => ({ ...prev, [key]: value }));
      return;
    }
    const { items, setItems, captureHistory, saveItems, autoSave, onDirty } = latest.current;
    // The record open when the pick began, by identity: the row of the render that started it,
    // or, if this session has since rewritten that row (a save, an undo sync), the row it became.
    // A position or an id alone could name a different record by now: the other editor over the
    // same list shifts rows, and two rows may share an id. Only a pick that outlived its session
    // falls back to the id, preferring the row it had.
    let index = rowAtRender ? items.indexOf(rowAtRender) : -1;
    if (index === -1 && sameSession && openRow.current) index = items.indexOf(openRow.current);
    if (index === -1 && !sameSession && editingId) {
      index = rowWithId(items, editingIndex, editingId);
    }
    // The record is gone (deleted, or discarded by an undo). Writing it back would resurrect
    // something the user just removed, or land on another record.
    if (index === -1) return;
    // Match on the session, not on the buffer's id. The id is editable text: a
    // user who renames the club while the copy is in flight would otherwise fail
    // an `id === editingId` check, leave the buffer without the asset path, and
    // lose it on the next Save — the record having been written correctly.
    if (sameSession) setEditing((prev) => ({ ...prev, [key]: value }));
    captureHistory();
    const written = { ...items[index], [key]: value };
    const updated = items.map((item, i) => (i === index ? written : item));
    if (sameSession) openRow.current = written;
    setItems(updated);
    onDirty?.();
    if (autoSave)
      void saveItems(updated).catch(() => {
        /* persist already showed the error */
      });
  }

  /**
   * `id`'s row in `list`: `hint` while that row still carries the id, so a duplicate id that has
   * not moved stays told apart from its twin; otherwise the first row with it, or -1.
   */
  function rowWithId(list: T[], hint: number | null, id: string): number {
    if (hint !== null && list[hint]?.id === id) return hint;
    return list.findIndex((item) => item.id === id);
  }

  /**
   * Where the record being edited sits in `list` now, or `null` for a new record or one that is
   * no longer there. Not trusted to the position it was opened at: a second editor over the same
   * list inserts and removes rows before it, and writing back by a stale position overwrote a
   * different record (#1197).
   *
   * Found by object identity only (`openRow`). Every write to the list keeps it current: this
   * editor's own save, delete, duplicate and field commit set it, and an undo or redo re-anchors it
   * in `syncEditing`; the other editor never replaces this editor's row. So a row that is not in
   * the list is a record that was deleted, and `null` saves it as a new one. Falling back to its id
   * would write it over a twin that shares the id, or over whoever took a blank id's place.
   */
  function locate(list: T[]): number | null {
    if (editingIndex === null) return null;
    const index = openRow.current ? list.indexOf(openRow.current) : -1;
    return index === -1 ? null : index;
  }

  function handleSelect(index: number) {
    openRow.current = items[index];
    setEditing({ ...items[index] });
    setEditingIndex(index);
    setEditingId(items[index].id);
    setRevision((r) => r + 1);
    onOpen();
  }

  function handleAdd() {
    openRow.current = null;
    setEditing(empty());
    setEditingIndex(null);
    setEditingId(null);
    setRevision((r) => r + 1);
    onOpen();
  }

  function handleDelete(index: number) {
    const open = locate(items);
    captureHistory();
    const updated = items.filter((_, i) => i !== index);
    setItems(updated);
    if (autoSave)
      void saveItems(updated).catch(() => {
        /* persist already showed the error */
      });
    if (open === index) {
      onClose();
    } else if (open !== null && index < open) {
      setEditingIndex(open - 1);
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
    openRow.current = clone;
    setItems(updated);
    if (autoSave)
      void saveItems(updated).catch(() => {
        /* persist already showed the error */
      });
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
    let idx = openRow.current ? newItems.indexOf(openRow.current) : -1;
    if (idx === -1 && editingId) idx = rowWithId(newItems, editingIndex, editingId);
    if (idx === -1) {
      // The restored snapshot no longer contains the record (e.g. an undo that
      // removed it). Close so a save can't re-add it.
      onClose();
      return;
    }
    openRow.current = newItems[idx];
    setEditing({ ...newItems[idx] });
    setEditingIndex(idx);
    setRevision((r) => r + 1);
  }

  async function handleSave() {
    captureHistory();
    // A record another editor removed while it was open is saved as a new one: what the form
    // shows is what the user asked to keep, and writing it over a different record loses that one.
    const open = locate(items);
    const updated =
      open === null ? [...items, editing] : items.map((item, i) => (i === open ? editing : item));
    const newIndex = open ?? updated.length - 1;
    openRow.current = editing;
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

  return {
    editing,
    // The open record's current row, so a list highlights the record the form shows.
    editingIndex: locate(items),
    revision,
    setEditing,
    updateField,
    commitField,
    handleSelect,
    handleAdd,
    handleDelete,
    handleDuplicate,
    handleSave,
    syncEditing,
  };
}
