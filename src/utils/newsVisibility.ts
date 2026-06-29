/**
 * News visibility, mirroring the backend `article_is_visible` rule.
 *
 * Some articles are dated for a future event (e.g. the World Cup kickoff, dated
 * at kickoff). They must stay hidden until their day so they can't sit atop the
 * feed — or inflate the unread badge — every day until they arrive. Dates come
 * in two shapes (a bare `YYYY-MM-DD` and an RFC3339 timestamp), so compare on
 * the `YYYY-MM-DD` day prefix.
 */

/** The `YYYY-MM-DD` day an article (or clock) date falls on. */
export function articleDay(date: string): string {
  return date.slice(0, 10);
}

/** Whether an article dated `date` is visible at clock date `today`. */
export function isNewsArticleVisible(
  date: string,
  today: string | undefined | null,
): boolean {
  if (!today) return true;
  return articleDay(date) <= articleDay(today);
}
