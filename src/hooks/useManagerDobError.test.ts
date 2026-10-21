import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useManagerDobError } from "./useManagerDobError";

vi.mock("../services/careerService", () => ({ validateManagerDob: vi.fn() }));

const { validateManagerDob } = await import("../services/careerService");
const mockedValidate = vi.mocked(validateManagerDob);

beforeEach(() => {
  mockedValidate.mockReset();
});

describe("useManagerDobError", () => {
  /**
   * Given a date of birth the backend finds too young for the chosen start
   * When the hook is asked
   * Then it asks with exactly the date, year and phase, and returns the backend's key.
   */
  it("asks the backend about the date, year and phase, and returns its key", async () => {
    mockedValidate.mockResolvedValue("be.error.createManager.minAge");

    const { result } = renderHook(() => useManagerDobError("1996-06-15", 2026, "seasonStart"));
    expect(result.current).toBeNull();

    await waitFor(() => expect(result.current).toBe("be.error.createManager.minAge"));
    expect(mockedValidate).toHaveBeenCalledWith("1996-06-15", 2026, "seasonStart");
  });

  /**
   * Given no date of birth yet, or a start year that is not valid
   * When the hook renders
   * Then the backend is not asked, and an earlier answer is not kept.
   */
  it("does not ask without a date and a valid start, and forgets the earlier answer", async () => {
    mockedValidate.mockResolvedValue("be.error.createManager.minAge");
    const { result, rerender } = renderHook(
      ({ dob, year }: { dob: string; year: number | null }) =>
        useManagerDobError(dob, year, "seasonStart"),
      { initialProps: { dob: "1996-06-15", year: 2026 as number | null } },
    );
    await waitFor(() => expect(result.current).toBe("be.error.createManager.minAge"));
    mockedValidate.mockClear();

    rerender({ dob: "", year: 2026 });
    expect(result.current).toBeNull();
    rerender({ dob: "1996-06-15", year: null });
    expect(result.current).toBeNull();

    expect(mockedValidate).not.toHaveBeenCalled();
  });

  /**
   * Given the start phase changes while the date of birth stays the same
   * When the hook renders again
   * Then the backend is asked again with the new phase, and its new answer replaces the old.
   */
  it("asks again when the start phase changes", async () => {
    mockedValidate.mockImplementation(async (_dob, _year, phase) =>
      phase === "seasonStart" ? "be.error.createManager.minAge" : null,
    );
    const { result, rerender } = renderHook(
      ({ phase }) => useManagerDobError("2008-08-01", 2038, phase),
      { initialProps: { phase: "seasonStart" } },
    );
    await waitFor(() => expect(result.current).toBe("be.error.createManager.minAge"));

    rerender({ phase: "midSeason" });

    await waitFor(() => expect(result.current).toBeNull());
    expect(mockedValidate).toHaveBeenLastCalledWith("2008-08-01", 2038, "midSeason");
  });

  /**
   * Given an under-age verdict and then a year the backend cannot open a career in (the call fails)
   * When the hook asks again
   * Then the old verdict is dropped: a failed read does not keep blocking the form with it.
   */
  it("drops the old verdict when a read fails", async () => {
    const quiet = vi.spyOn(console, "error").mockImplementation(() => {});
    mockedValidate
      .mockResolvedValueOnce("be.error.createManager.minAge")
      .mockRejectedValueOnce("be.error.createManager.invalidStartYear");
    const { result, rerender } = renderHook(
      ({ year }) => useManagerDobError("2010-06-15", year, "seasonStart"),
      { initialProps: { year: 2026 } },
    );
    await waitFor(() => expect(result.current).toBe("be.error.createManager.minAge"));

    rerender({ year: 300000 });

    await waitFor(() => expect(result.current).toBeNull());
    quiet.mockRestore();
  });

  /**
   * Given a first question still unanswered when the form changes, and a second that answers first
   * When the first, which belongs to the old form, answers last
   * Then its verdict is dropped: the form keeps the answer to what is on screen.
   */
  it("ignores a late answer to a question the form has moved past", async () => {
    let answerFirst: (value: string | null) => void = () => {};
    mockedValidate
      .mockReturnValueOnce(
        new Promise<string | null>((resolve) => {
          answerFirst = resolve;
        }),
      )
      .mockResolvedValueOnce(null);
    const { result, rerender } = renderHook(
      ({ phase }) => useManagerDobError("2008-08-01", 2038, phase),
      { initialProps: { phase: "seasonStart" } },
    );
    rerender({ phase: "midSeason" });
    await waitFor(() => expect(mockedValidate).toHaveBeenCalledTimes(2));

    await act(async () => {
      answerFirst("be.error.createManager.minAge");
    });

    expect(result.current).toBeNull();
  });
});
