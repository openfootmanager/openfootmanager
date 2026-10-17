import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { PlayersPage, PlayersPageQuery } from "../services/playersService";
import { usePlayersPage } from "./usePlayersPage";

vi.mock("../services/playersService", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../services/playersService")>()),
  fetchPlayersPage: vi.fn(),
}));

const { fetchPlayersPage } = await import("../services/playersService");
const mockedFetch = vi.mocked(fetchPlayersPage);

const baseQuery: PlayersPageQuery = {
  search: null,
  position: null,
  positions: [],
  age_min: null,
  age_max: null,
  ovr_min: null,
  ovr_max: null,
  value_min: null,
  value_max: null,
  in_budget: false,
  scope: "all",
  team_id: null,
  status: "all",
  sort_key: "ovr",
  sort_asc: false,
  page: 1,
  page_size: 30,
};

const page = (label: string): PlayersPage =>
  ({ items: [{ id: label }], total: 1, page: 1, page_size: 30 }) as unknown as PlayersPage;

const translate = (key: string) => key;

beforeEach(() => {
  mockedFetch.mockReset();
});

describe("usePlayersPage", () => {
  /**
   * Given a starting query
   * When the hook mounts
   * Then it asks the backend for that page and exposes it.
   */
  it("fetches the page for the query it is given", async () => {
    mockedFetch.mockResolvedValue(page("a"));

    const { result } = renderHook(() => usePlayersPage(baseQuery, translate));

    await waitFor(() => expect(result.current.slice?.items[0].id).toBe("a"));
    expect(mockedFetch).toHaveBeenCalledWith(baseQuery);
  });

  /**
   * Given the player is on page 3
   * When any filter changes
   * Then the page goes back to 1, and a change that sets the page itself keeps it.
   */
  it("returns to page 1 when a filter changes, unless the change is the page", async () => {
    mockedFetch.mockResolvedValue(page("a"));
    const { result } = renderHook(() => usePlayersPage({ ...baseQuery, page: 3 }, translate));
    await waitFor(() => expect(result.current.slice).not.toBeNull());

    act(() => result.current.patchQuery({ search: "ron" }));
    expect(result.current.query.page).toBe(1);

    act(() => result.current.patchQuery({ page: 4 }));
    expect(result.current.query.page).toBe(4);
    expect(result.current.query.search).toBe("ron");
  });

  /**
   * Given a page already shown
   * When the data behind it changes and refetch is called
   * Then the same query is asked for again.
   */
  it("asks again for the same query on refetch", async () => {
    mockedFetch.mockResolvedValue(page("a"));
    const { result } = renderHook(() => usePlayersPage(baseQuery, translate));
    await waitFor(() => expect(mockedFetch).toHaveBeenCalledTimes(1));

    act(() => result.current.refetch());

    await waitFor(() => expect(mockedFetch).toHaveBeenCalledTimes(2));
    expect(mockedFetch).toHaveBeenLastCalledWith(baseQuery);
  });

  /**
   * Given the backend refuses
   * When the page is requested
   * Then the translated reason is exposed and clears on the next good answer.
   */
  it("exposes a translated error and clears it on success", async () => {
    mockedFetch.mockRejectedValueOnce("some failure").mockResolvedValueOnce(page("b"));
    const { result } = renderHook(() => usePlayersPage(baseQuery, translate));

    await waitFor(() => expect(result.current.error).toBe("some failure"));

    act(() => result.current.refetch());
    await waitFor(() => expect(result.current.error).toBeNull());
    expect(result.current.slice?.items[0].id).toBe("b");
  });

  /**
   * Given two requests in flight, the first answered last
   * When both have answered
   * Then the page shown is the newer query's, not the late older one.
   */
  it("ignores a late answer to an older query", async () => {
    let answerFirst!: (value: PlayersPage) => void;
    mockedFetch
      .mockImplementationOnce(() => new Promise<PlayersPage>((resolve) => (answerFirst = resolve)))
      .mockResolvedValueOnce(page("newer"));
    const { result } = renderHook(() => usePlayersPage(baseQuery, translate));

    act(() => result.current.patchQuery({ search: "x" }));
    await waitFor(() => expect(result.current.slice?.items[0].id).toBe("newer"));
    await act(async () => answerFirst(page("older")));

    expect(result.current.slice?.items[0].id).toBe("newer");
  });

  /**
   * Given the user is on page 3 and a game update leaves only 40 players (two pages)
   * When the backend answers page 3 with no rows
   * Then the hook moves the query to the last page, instead of leaving the user on an empty page
   *      whose pagination has gone.
   */
  it("moves back to the last page when the total shrinks below the current page", async () => {
    mockedFetch.mockImplementation(async (query) =>
      query.page > 2
        ? ({ items: [], total: 40, page: query.page, page_size: 30 } as unknown as PlayersPage)
        : ({
            items: [{ id: `page-${query.page}` }],
            total: 40,
            page: query.page,
            page_size: 30,
          } as unknown as PlayersPage),
    );
    const { result } = renderHook(() => usePlayersPage({ ...baseQuery, page: 3 }, translate));

    await waitFor(() => expect(result.current.query.page).toBe(2));
    await waitFor(() => expect(result.current.slice?.items[0].id).toBe("page-2"));
  });
});
