import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import HomeSeasonStatusCard from "./HomeSeasonStatusCard";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "season.preseasonStatus") return "Preseason Status";
      if (key === "season.phases.Preseason") return "Preseason";
      if (key === "season.transferWindowStatus.Open") return "Window Open";
      if (key === "season.preseasonFocus") return "Prepare the squad for the opener.";
      if (key === "season.opener") return "Opener";
      if (key === "season.startsOn") return `Starts on ${params?.date}`;
      if (key === "season.startsInDays") return `Starts in ${params?.count} days`;
      if (key === "transfers.centre") return "Transfer Centre";
      if (key === "season.windowClosesOn") return `Closes on ${params?.date}`;
      return key;
    },
  }),
}));

describe("HomeSeasonStatusCard", () => {
  it("renders preseason and transfer-window summary content", () => {
    render(
      <HomeSeasonStatusCard
        phase="Preseason"
        seasonStartLabel="Jan 12"
        daysUntilSeasonStart={5}
        transferWindowStatus="Open"
        transferWindowVariant="success"
        transferWindowSummary="Window closes in 3 days"
        transferWindowOpensOn={null}
        transferWindowClosesOn="2025-01-15"
        lang="en"
      />,
    );

    expect(screen.getByText("Preseason Status")).toBeInTheDocument();
    expect(screen.getByText("Preseason")).toBeInTheDocument();
    expect(screen.getByText("Window Open")).toBeInTheDocument();
    expect(screen.getByText("Starts on Jan 12")).toBeInTheDocument();
    expect(screen.getByText("Starts in 5 days")).toBeInTheDocument();
    expect(screen.getByText("Window closes in 3 days")).toBeInTheDocument();
  });
});
