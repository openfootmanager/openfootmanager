import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import i18n, { i18nReady } from "../../i18n";

import type { MessageData } from "../../store/gameStore";
import type { ArchivedMessagesPage } from "../../services/inboxService";
import ArchivedMessages from "./ArchivedMessages";

const fetchArchivedMessages = vi.hoisted(() => vi.fn());

vi.mock("../../services/inboxService", () => ({ fetchArchivedMessages }));

vi.mock("react-i18next", async (importOriginal) => ({
  ...(await importOriginal<typeof import("react-i18next")>()),
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: "en" },
  }),
}));

function message(id: string, subject: string, date: string): MessageData {
  return {
    id,
    subject,
    body: `Body of ${subject}`,
    sender: "Board",
    sender_role: "Board",
    date,
    read: true,
    category: "System",
    priority: "Normal",
    actions: [],
    context: { team_id: null, player_id: null, fixture_id: null, match_result: null },
  };
}

function page(
  items: MessageData[],
  next: { date: string; id: string } | null,
): ArchivedMessagesPage {
  return { items, next_before: next };
}

describe("ArchivedMessages", () => {
  beforeAll(async () => {
    await i18nReady;
    await i18n.changeLanguage("en");
    i18n.addResourceBundle(
      "en",
      "translation",
      {
        "test.archived.subject": "Offer from {{team}}",
        "test.archived.body": "{{team}} wants to sign you.",
      },
      true,
      true,
    );
  });

  beforeEach(() => {
    fetchArchivedMessages.mockReset();
  });

  /**
   * Given the inbox is open
   * When nothing has been asked for
   * Then older mail is not fetched, and the control to load it is offered.
   */
  it("offers to load older messages without fetching any on its own", () => {
    render(<ArchivedMessages />);

    expect(screen.getByRole("button", { name: "inbox.archive.load" })).toBeEnabled();
    expect(fetchArchivedMessages).not.toHaveBeenCalled();
  });

  /**
   * Given archived mail on two pages
   * When the player loads, then loads more
   * Then the second request continues from the first page's cursor, both pages are listed in
   *      order, and with the last page the control gives way to the end note.
   */
  it("pages through the archive from each page's own cursor", async () => {
    const cursor = { date: "2030-02-01", id: "b" };
    fetchArchivedMessages
      .mockResolvedValueOnce(page([message("b", "Second newest", "2030-02-01")], cursor))
      .mockResolvedValueOnce(page([message("a", "Oldest", "2030-01-01")], null));
    render(<ArchivedMessages />);

    fireEvent.click(screen.getByRole("button", { name: "inbox.archive.load" }));
    expect(await screen.findByText("Second newest")).toBeInTheDocument();
    expect(fetchArchivedMessages).toHaveBeenNthCalledWith(1, null);

    fireEvent.click(screen.getByRole("button", { name: "inbox.archive.loadMore" }));
    expect(await screen.findByText("Oldest")).toBeInTheDocument();
    expect(fetchArchivedMessages).toHaveBeenNthCalledWith(2, cursor);

    const subjects = screen.getAllByRole("listitem").map((item) => item.textContent ?? "");
    expect(subjects[0]).toContain("Second newest");
    expect(subjects[1]).toContain("Oldest");
    expect(screen.queryByRole("button", { name: /inbox\.archive\.load/ })).toBeNull();
    expect(screen.getByText("inbox.archive.end")).toBeInTheDocument();
  });

  /**
   * Given an archive with nothing in it
   * When the player loads
   * Then they are told there is none, and there is nothing more to load.
   */
  it("says so when there is no older mail", async () => {
    fetchArchivedMessages.mockResolvedValueOnce(page([], null));
    render(<ArchivedMessages />);

    fireEvent.click(screen.getByRole("button", { name: "inbox.archive.load" }));

    expect(await screen.findByText("inbox.archive.empty")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /inbox\.archive\.load/ })).toBeNull();
  });

  /**
   * Given a load that fails
   * When the player has clicked
   * Then the failure is announced and the same control stays, so they can try again.
   */
  it("announces a failed load and lets the player retry", async () => {
    fetchArchivedMessages
      .mockRejectedValueOnce(new Error("boom"))
      .mockResolvedValueOnce(page([message("a", "Back again", "2030-01-01")], null));
    render(<ArchivedMessages />);

    fireEvent.click(screen.getByRole("button", { name: "inbox.archive.load" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("inbox.archive.error");

    fireEvent.click(screen.getByRole("button", { name: "inbox.archive.load" }));
    expect(await screen.findByText("Back again")).toBeInTheDocument();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  /**
   * Given a page that is still loading
   * When the player clicks again
   * Then no second request is made, and the control says it is busy.
   */
  it("does not ask twice while a page is loading", async () => {
    let resolve: (value: ArchivedMessagesPage) => void = () => {};
    fetchArchivedMessages.mockReturnValueOnce(
      new Promise<ArchivedMessagesPage>((r) => {
        resolve = r;
      }),
    );
    render(<ArchivedMessages />);

    fireEvent.click(screen.getByRole("button", { name: "inbox.archive.load" }));
    const busy = await screen.findByRole("button", { name: "inbox.archive.loading" });
    expect(busy).toBeDisabled();
    expect(busy).toHaveAttribute("aria-busy", "true");
    fireEvent.click(busy);
    expect(fetchArchivedMessages).toHaveBeenCalledTimes(1);

    resolve(page([], null));
    await waitFor(() => expect(screen.getByText("inbox.archive.empty")).toBeInTheDocument());
  });

  /**
   * Given a loaded message
   * When its row is opened
   * Then its body can be read there: archived mail is read-only and has no actions.
   */
  it("shows a message body in its row and offers no actions", async () => {
    fetchArchivedMessages.mockResolvedValueOnce(
      page([message("a", "Old offer", "2030-01-01")], null),
    );
    render(<ArchivedMessages />);

    fireEvent.click(screen.getByRole("button", { name: "inbox.archive.load" }));
    await screen.findByText("Old offer");

    expect(screen.getByText("Body of Old offer")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /delete|inbox\.deleteMessage/i })).toBeNull();
  });

  /**
   * Given archived mail the way the backend writes most of it: blank subject and body, with
   *       translation keys and parameters beside them
   * When it is loaded
   * Then the row shows the translated text, not a blank or a raw key.
   */
  it("shows key-based mail in the player's language", async () => {
    fetchArchivedMessages.mockResolvedValueOnce(
      page(
        [
          {
            ...message("k", "", "2030-01-01"),
            body: "",
            subject_key: "test.archived.subject",
            body_key: "test.archived.body",
            i18n_params: { team: "Alpha FC" },
          },
        ],
        null,
      ),
    );
    render(<ArchivedMessages />);

    fireEvent.click(screen.getByRole("button", { name: "inbox.archive.load" }));

    expect(await screen.findByText("Offer from Alpha FC")).toBeInTheDocument();
    expect(screen.getByText("Alpha FC wants to sign you.")).toBeInTheDocument();
  });
});
