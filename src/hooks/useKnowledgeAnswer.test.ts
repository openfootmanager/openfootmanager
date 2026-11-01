import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { answeredOr, useKnowledgeAnswer } from "./useKnowledgeAnswer";

describe("useKnowledgeAnswer", () => {
  /**
   * Given a question whose answer is "nothing known" (null)
   * When it is asked
   * Then it reads loading, then answered with null: a real absence, not a failure
   */
  it("reads loading, then the answer, a null answer included", async () => {
    const { result } = renderHook(() => useKnowledgeAnswer("p1|r1", async () => null));

    expect(result.current.status).toBe("loading");
    await waitFor(() => expect(result.current).toEqual({ status: "answered", value: null }));
  });

  /**
   * Given a question whose request fails
   * When it is asked
   * Then it reads failed, with no data, and a retry
   */
  it("reads a failed load as failed, never as an answer", async () => {
    const errors = vi.spyOn(console, "error").mockImplementation(() => {});
    const { result } = renderHook(() =>
      useKnowledgeAnswer("p1|r1", () => Promise.reject("be.error.noActiveGameSession")),
    );

    await waitFor(() => expect(result.current.status).toBe("failed"));
    expect(result.current).not.toHaveProperty("value");
    errors.mockRestore();
  });

  /**
   * Given a failed load
   * When the retry is asked for and the backend answers this time
   * Then it reads loading again, then the answer
   */
  it("asks again on retry and reads the new answer", async () => {
    const errors = vi.spyOn(console, "error").mockImplementation(() => {});
    const load = vi
      .fn<() => Promise<string>>()
      .mockRejectedValueOnce("down")
      .mockResolvedValueOnce("answer");
    const { result } = renderHook(() => useKnowledgeAnswer("p1|r1", load));
    await waitFor(() => expect(result.current.status).toBe("failed"));

    act(() => {
      if (result.current.status === "failed") result.current.retry();
    });

    expect(result.current.status).toBe("loading");
    await waitFor(() => expect(result.current).toEqual({ status: "answered", value: "answer" }));
    expect(load).toHaveBeenCalledTimes(2);
    errors.mockRestore();
  });

  /**
   * Given one question's answer on screen
   * When another question is asked, whose answer has not arrived yet
   * Then it reads loading: the previous question's answer never shows for the new one
   */
  it("never shows another question's answer", async () => {
    const { result, rerender } = renderHook(({ key, load }) => useKnowledgeAnswer(key, load), {
      initialProps: { key: "p1|r1", load: async () => "first" },
    });
    await waitFor(() => expect(result.current).toEqual({ status: "answered", value: "first" }));

    rerender({ key: "p2|r1", load: () => new Promise<string>(() => {}) });

    expect(result.current.status).toBe("loading");
  });

  /**
   * Given a failed, a loading and an answered state
   * When the value is read with a fallback
   * Then the fallback stands in for the first two, the answer for the third
   */
  it("reads the fallback unless answered", () => {
    expect(answeredOr({ status: "loading" }, "none")).toBe("none");
    expect(answeredOr({ status: "failed", retry: () => {} }, "none")).toBe("none");
    expect(answeredOr({ status: "answered", value: "x" }, "none")).toBe("x");
  });
});
