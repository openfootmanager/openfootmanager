import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Circle, Users } from "lucide-react";

import HomeOnboardingChecklistCard from "./HomeOnboardingChecklistCard";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => {
      if (key === "onboarding.title") return "Getting Started";
      if (key === "onboarding.description") return "Complete the first setup steps.";
      return key;
    },
  }),
}));

describe("HomeOnboardingChecklistCard", () => {
  it("renders onboarding progress and delegates navigation", () => {
    const onNavigate = vi.fn();

    render(
      <HomeOnboardingChecklistCard
        completedSteps={1}
        totalSteps={2}
        steps={[
          {
            id: "squad",
            done: false,
            label: "Review squad",
            description: "Check the current roster.",
            tab: "Squad",
            icon: <Users className="w-4 h-4" />,
          },
          {
            id: "staff",
            done: true,
            label: "Hire staff",
            description: "Fill key roles.",
            tab: "Staff",
            icon: <Circle className="w-4 h-4" />,
          },
        ]}
        onNavigate={onNavigate}
      />,
    );

    expect(screen.getByText("Getting Started")).toBeInTheDocument();
    expect(screen.getByText("Complete the first setup steps.")).toBeInTheDocument();
    expect(screen.getByText("1/2")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Review squad/i }));

    expect(onNavigate).toHaveBeenCalledWith("Squad");
  });
});
