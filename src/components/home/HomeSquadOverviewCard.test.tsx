import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Scale } from "lucide-react";

import HomeSquadOverviewCard from "./HomeSquadOverviewCard";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "dashboard.training") return "Training";
      if (key === "home.squadOverview") return "Squad Overview";
      if (key === "home.avgCondition") return "Avg Condition";
      if (key === "home.avgOvr") return "Avg OVR";
      if (key === "home.exhaustedPlayers") return `${params?.count} exhausted players`;
      if (key === "home.scheduleLabel") return "Schedule";
      if (key === "common.trainingFocuses.Physical") return "Physical";
      return key;
    },
  }),
}));

describe("HomeSquadOverviewCard", () => {
  it("renders squad overview metrics and schedule summary", () => {
    render(
      <HomeSquadOverviewCard
        avgCondition={78}
        avgOvr={64}
        exhaustedCount={2}
        scheduleIcon={<Scale className="w-3.5 h-3.5" />}
        scheduleColorClass="text-primary-500"
        scheduleLabel="Balanced"
        focus="Physical"
      />,
    );

    expect(screen.getByText("Squad Overview")).toBeInTheDocument();
    expect(screen.getByText("78%")).toBeInTheDocument();
    expect(screen.getByText("64")).toBeInTheDocument();
    expect(screen.getByText("2 exhausted players")).toBeInTheDocument();
    expect(screen.getByText("Balanced")).toBeInTheDocument();
    expect(screen.getByText("Physical")).toBeInTheDocument();
  });

  /**
   * Given no average condition (the backend has not answered, or no healthy first-team player)
   * When the card renders
   * Then it shows a dash, not 0%.
   */
  it("shows a dash instead of 0% when there is no average condition", () => {
    render(
      <HomeSquadOverviewCard
        avgCondition={null}
        avgOvr={64}
        exhaustedCount={0}
        scheduleIcon={<Scale className="w-3.5 h-3.5" />}
        scheduleColorClass="text-primary-500"
        scheduleLabel="Balanced"
        focus="Physical"
      />,
    );

    expect(screen.getByText("—")).toBeInTheDocument();
    expect(screen.queryByText("0%")).not.toBeInTheDocument();
  });
});
