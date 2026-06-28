/**
 * Serializes async writes so they run one at a time in call order. Without this,
 * concurrent saves (e.g. rapid autosaves across entity types) issue overlapping
 * full-file writes, and an older write can land after a newer one — silently
 * reverting data on disk.
 *
 * Each enqueued task runs only after the previous one settles (success or
 * failure), so a single failed write doesn't stall or break the queue. Callers
 * still receive their own task's promise to await or catch.
 */
export function createWriteQueue() {
  let tail: Promise<unknown> = Promise.resolve();

  return function enqueue<T>(task: () => Promise<T>): Promise<T> {
    const result = tail.then(task, task);
    // Keep the chain alive regardless of this task's outcome.
    tail = result.then(
      () => undefined,
      () => undefined,
    );
    return result;
  };
}
