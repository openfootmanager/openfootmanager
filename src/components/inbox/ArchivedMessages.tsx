import type { JSX } from "react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { formatDateShort } from "../../lib/helpers";
import { type ArchiveCursor, fetchArchivedMessages } from "../../services/inboxService";
import type { MessageData } from "../../store/gameStore";
import { resolveMessage } from "../../utils/backendI18n";
import { Button } from "../ui";
import { renderMessageBodyLine } from "./inboxHelpers";

type LoadState = "idle" | "loading" | "failed";

/**
 * Mail older than the inbox keeps, fetched a page at a time. It is read-only: the backend keeps
 * only read mail with nothing left to answer here, so there is nothing to act on, mark or delete.
 */
export default function ArchivedMessages(): JSX.Element {
  const { t, i18n } = useTranslation();
  const [messages, setMessages] = useState<MessageData[]>([]);
  const [next, setNext] = useState<ArchiveCursor | null>(null);
  const [loadedOnce, setLoadedOnce] = useState(false);
  const [state, setState] = useState<LoadState>("idle");

  async function loadPage(): Promise<void> {
    setState("loading");
    try {
      const page = await fetchArchivedMessages(next);
      setMessages((loaded) => [...loaded, ...page.items.map(resolveMessage)]);
      setNext(page.next_before);
      setLoadedOnce(true);
      setState("idle");
    } catch {
      setState("failed");
    }
  }

  const exhausted = loadedOnce && next === null;
  const loading = state === "loading";
  const buttonLabel = loading
    ? t("inbox.archive.loading")
    : t(loadedOnce ? "inbox.archive.loadMore" : "inbox.archive.load");

  return (
    <section aria-labelledby="inbox-archive-heading" className="mt-4">
      <h3
        id="inbox-archive-heading"
        className="mb-2 font-heading text-sm font-bold uppercase tracking-wider text-gray-700 dark:text-gray-300"
      >
        {t("inbox.archive.heading")}
      </h3>

      {messages.length > 0 && (
        <ul className="mb-3 divide-y divide-gray-100 rounded-xl border border-gray-200 bg-white dark:divide-navy-600 dark:border-navy-600 dark:bg-navy-800">
          {messages.map((message) => (
            <li key={message.id}>
              <details className="group px-4 py-3">
                <summary className="cursor-pointer text-sm text-gray-900 focus:outline-none focus-visible:ring-2 focus-visible:ring-primary-500 dark:text-gray-100">
                  <span className="mr-2 text-xs text-gray-500 dark:text-gray-400">
                    {formatDateShort(message.date, i18n.language)}
                  </span>
                  {message.subject}
                </summary>
                <div className="mt-2 text-gray-700 dark:text-gray-300">
                  {message.body.split("\n").map(renderMessageBodyLine)}
                </div>
              </details>
            </li>
          ))}
        </ul>
      )}

      {state === "failed" && (
        <p role="alert" className="mb-2 text-sm text-red-600 dark:text-red-400">
          {t("inbox.archive.error")}
        </p>
      )}

      {exhausted ? (
        <p role="status" className="text-sm text-gray-500 dark:text-gray-400">
          {t(messages.length === 0 ? "inbox.archive.empty" : "inbox.archive.end")}
        </p>
      ) : (
        <Button
          variant="outline"
          size="sm"
          disabled={loading}
          aria-busy={loading}
          onClick={() => {
            void loadPage();
          }}
        >
          {buttonLabel}
        </Button>
      )}
    </section>
  );
}
