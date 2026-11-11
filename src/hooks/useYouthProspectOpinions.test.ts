import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { getYouthProspectOpinions } from "../services/scoutingService";
import type { PotentialOpinion } from "../store/generated/PotentialOpinion";
import { useYouthProspectOpinions } from "./useYouthProspectOpinions";

vi.mock("../services/scoutingService", () => ({ getYouthProspectOpinions: vi.fn() }));
const mockedOpinions = vi.mocked(getYouthProspectOpinions);

const opinion = (low: number): PotentialOpinion => ({
  low,
  high: low + 8,
  band: "A",
  certainty: "fairly_sure",
  source: { kind: "scout", scout_name: "Ann Scout", scouted_on: "2026-09-01", current: true },
  wonderkid: false,
});

beforeEach(() => {
  mockedOpinions.mockReset();
});

describe("useYouthProspectOpinions", () => {
  /**
   * Given a youth report whose scout judged one prospect 76-84
   * When the hook asks for it
   * Then it reads no opinions until the backend answers, then the backend's, keyed by prospect
   */
  it("reads no opinions until the backend answers, then its answer", async () => {
    mockedOpinions.mockResolvedValue({ "prospect-1": opinion(76) });

    const { result } = renderHook(() => useYouthProspectOpinions("youth-scout-1", "r1"));

    expect(result.current).toEqual({});
    await waitFor(() => expect(result.current).toEqual({ "prospect-1": opinion(76) }));
    expect(mockedOpinions).toHaveBeenCalledWith("youth-scout-1");
  });

  /**
   * Given an open message that is no youth report
   * When the hook is used
   * Then it asks the backend nothing and reads no opinions
   */
  it("asks nothing when the open message is no youth report", () => {
    const { result } = renderHook(() => useYouthProspectOpinions(null, "r1"));

    expect(result.current).toEqual({});
    expect(mockedOpinions).not.toHaveBeenCalled();
  });

  /**
   * Given one report's opinions on screen
   * When another report opens, whose answer has not arrived
   * Then the first report's opinions are not shown for it
   */
  it("never shows the previous report's opinions", async () => {
    mockedOpinions.mockResolvedValueOnce({ "prospect-1": opinion(76) });
    const { result, rerender } = renderHook(
      ({ messageId }) => useYouthProspectOpinions(messageId, "r1"),
      { initialProps: { messageId: "youth-scout-1" } },
    );
    await waitFor(() => expect(result.current).toEqual({ "prospect-1": opinion(76) }));
    mockedOpinions.mockReturnValueOnce(new Promise(() => {}));

    rerender({ messageId: "youth-scout-2" });

    expect(result.current).toEqual({});
  });

  /**
   * Given a report's opinions on screen
   * When the game moves on (a new revision) and the backend's next answer fails
   * Then no opinions are read, not the last answer
   */
  it("reads no opinions, not the last answer, until the game's next answer arrives", async () => {
    mockedOpinions.mockResolvedValueOnce({ "prospect-1": opinion(76) });
    const errors = vi.spyOn(console, "error").mockImplementation(() => {});
    const { result, rerender } = renderHook(
      ({ revision }) => useYouthProspectOpinions("youth-scout-1", revision),
      { initialProps: { revision: "r1" } },
    );
    await waitFor(() => expect(result.current).toEqual({ "prospect-1": opinion(76) }));
    mockedOpinions.mockRejectedValueOnce("be.error.noActiveGameSession");

    rerender({ revision: "r2" });

    await waitFor(() => expect(errors).toHaveBeenCalled());
    expect(result.current).toEqual({});
    errors.mockRestore();
  });
});
