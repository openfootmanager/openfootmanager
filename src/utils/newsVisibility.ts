/**
 * News visibility, mirroring the backend `article_is_visible` rule.
 *
 * Some articles are dated for a future event (e.g. the World Cup kickoff, dated
 * at kickoff). They must stay hidden until their day so they can't sit atop the
 * feed — or inflate the unread badge — every day until they arrive. Dates come
 * in two shapes (a bare `YYYY-MM-DD` and an RFC3339 timestamp), so compare on
 * the `YYYY-MM-DD` day prefix.
 */

/**
 * The `YYYY-MM-DD` day an article (or clock) date falls on.
 *
 * Tolerates a missing date rather than throwing. An item with no date can't be
 * shown to be in the future, and hiding it — or crashing the unread count —
 * would be a worse answer than showing it.
 */
export function articleDay(date: string | undefined | null): string {
  return date ? date.slice(0, 10) : "";
}

/** Whether an article dated `date` is visible at clock date `today`. */
export function isNewsArticleVisible(
  date: string | undefined | null,
  today: string | undefined | null,
): boolean {
  if (!today || !date) return true;
  return articleDay(date) <= articleDay(today);
}

/**
 * Whether an inbox message dated `date` is visible at clock date `today`,
 * mirroring the backend `message_is_visible` rule.
 *
 * The same rule as news, applied to the inbox, which had no such rule at all
 * until issue #520. Nothing dates a message ahead of the clock today — a
 * backend test now keeps it that way — so this is the display half of that
 * guard, and it keeps a future-dated message out of the unread badge as well as
 * out of the list.
 */
export function isMessageVisible(
  date: string | undefined | null,
  today: string | undefined | null,
): boolean {
  return isNewsArticleVisible(date, today);
}
