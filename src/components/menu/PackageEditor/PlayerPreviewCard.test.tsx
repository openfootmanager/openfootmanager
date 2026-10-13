import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { PlayerPreviewCard } from "./PlayerPreviewCard";
import type { PlayerDef } from "./types";

// Mock react-i18next so translation keys render verbatim; this lets us assert
// that the footedness value goes through t() rather than printing the raw enum.
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (k: string, params?: { age?: number }) =>
      params?.age === undefined ? k : `${k}:${params.age}`,
  }),
}));

function makePlayer(overrides: Partial<PlayerDef>): PlayerDef {
  return {
    id: "p1",
    firstName: "Ada",
    lastName: "Lovelace",
    name: "Ada Lovelace",
    club: "",
    nationality: "GB",
    position: "Midfielder",
    footedness: "Right",
    dateOfBirth: null,
    photo: null,
    overall: 70,
    attributes: null,
    ...overrides,
  } as PlayerDef;
}

describe("PlayerPreviewCard", () => {
  it("renders the footedness through the translation layer, not the raw enum", () => {
    render(<PlayerPreviewCard editing={makePlayer({ footedness: "Left" })} photoDataUrl={null} />);
    expect(screen.getByText("common.footedness.Left")).toBeInTheDocument();
    // The bare enum value must not leak into the rendered output.
    expect(screen.queryByText("Left")).not.toBeInTheDocument();
  });

  it("omits the footedness row for right-footed players (the default)", () => {
    render(<PlayerPreviewCard editing={makePlayer({ footedness: "Right" })} photoDataUrl={null} />);
    // The whole row is gone — assert on the row label, not just the value, so a
    // raw "Right" or other fallback value can't slip through.
    expect(screen.queryByText(/worldEditor\.playerFoot/)).not.toBeInTheDocument();
    expect(screen.queryByText("common.footedness.Right")).not.toBeInTheDocument();
  });

  describe("age today", () => {
    beforeEach(() => {
      vi.useFakeTimers();
      vi.setSystemTime(new Date("2026-10-12T12:00:00Z"));
    });

    afterEach(() => {
      vi.useRealTimers();
    });

    /**
     * Given a player whose birthday is today
     * When the preview renders
     * Then the age already counts the birthday (26), not the day-count floor (25)
     */
    it("counts a birthday that falls today", () => {
      render(
        <PlayerPreviewCard
          editing={makePlayer({ dateOfBirth: "2000-10-12" })}
          photoDataUrl={null}
        />,
      );

      expect(screen.getByText(/worldEditor\.playerAgeToday:26/)).toBeInTheDocument();
    });

    /**
     * Given a veteran older than 80
     * When the preview renders
     * Then the age is still shown
     */
    it("does not hide ages over 80", () => {
      render(
        <PlayerPreviewCard
          editing={makePlayer({ dateOfBirth: "1930-01-01" })}
          photoDataUrl={null}
        />,
      );

      expect(screen.getByText(/worldEditor\.playerAgeToday:96/)).toBeInTheDocument();
    });

    /**
     * Given a date of birth in the future
     * When the preview renders
     * Then no age is shown, not a negative one
     */
    it("shows no age for a future date of birth", () => {
      render(
        <PlayerPreviewCard
          editing={makePlayer({ dateOfBirth: "2030-01-01" })}
          photoDataUrl={null}
        />,
      );

      expect(screen.queryByText(/playerAgeToday/)).not.toBeInTheDocument();
    });

    /**
     * Given a date of birth that is not a date
     * When the preview renders
     * Then no age is shown
     */
    it("shows no age for an unreadable date of birth", () => {
      render(
        <PlayerPreviewCard
          editing={makePlayer({ dateOfBirth: "not-a-date" })}
          photoDataUrl={null}
        />,
      );

      expect(screen.queryByText(/playerAgeToday/)).not.toBeInTheDocument();
    });
  });
});
