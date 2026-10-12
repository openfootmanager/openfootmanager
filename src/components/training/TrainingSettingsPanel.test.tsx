import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { TrainingFocusAttributesData } from "../../store/types";
import TrainingSettingsPanel from "./TrainingSettingsPanel";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      const scheduleText: Record<string, string> = {
        "training.schedules.Intense.label": "Intense",
        "training.schedules.Intense.desc": "Six training days, one rest day",
        "training.schedules.Balanced.label": "Balanced",
        "training.schedules.Balanced.desc": "Four training days, three rest days",
        "training.schedules.Balanced.detail": "Training on Monday, Tuesday, Thursday, and Friday.",
        "training.schedules.Light.label": "Light",
        "training.schedules.Light.desc": "Two training days, five rest days",
      };

      if (key === "training.weeklySchedule") return "Weekly Schedule";
      if (key === "training.trainingFocus") return "Training Focus";
      if (key === "training.intensity") return "Intensity";
      if (key === "training.trainingAppliedNote") return "Applied note";
      if (key === "training.recoveryNote") return "Recovery note";
      if (key === "training.goalkeepersLabel") return "Goalkeepers";
      if (key === "training.currentlyTraining") {
        return `Training ${params?.attrs} at ${params?.intensity}`;
      }
      if (key === "training.todayIs") return `${params?.day} is ${params?.type}`;
      if (key === "training.aTrainingDay") return "a training day";
      if (key === "training.aRestDay") return "a rest day";
      if (key in scheduleText) return scheduleText[key];
      if (key.startsWith("training.focuses.")) return key.replace("training.focuses.", "");
      if (key.startsWith("training.intensities.")) return key.replace("training.intensities.", "");
      if (key.startsWith("training.days.")) return key.replace("training.days.", "");
      if (key.startsWith("common.attributes.")) return key.replace("common.attributes.", "");
      return key;
    },
  }),
}));

function focus(
  name: string,
  outfield: string[],
  goalkeeper: string[] = outfield,
): TrainingFocusAttributesData {
  return { focus: name, outfield, goalkeeper };
}

function renderPanel(
  currentFocus: string,
  focusAttributes: Record<string, TrainingFocusAttributesData>,
) {
  return render(
    <TrainingSettingsPanel
      currentFocus={currentFocus}
      currentIntensity="Medium"
      currentSchedule="Balanced"
      isSaving={false}
      todayWeekday={1}
      isTodayTraining={true}
      onSetTraining={vi.fn()}
      onSetSchedule={vi.fn()}
      scheduleIds={["Balanced"]}
      scheduleIcons={{ Balanced: "B" }}
      scheduleColors={{ Balanced: "text-blue" }}
      dayKeys={["mon", "tue", "wed", "thu", "fri", "sat", "sun"]}
      trainingFocusIds={["Technical"]}
      trainingFocusIcons={{ Technical: "T" }}
      focusAttributes={focusAttributes}
      intensityIds={["Medium"]}
      intensityColors={{ Medium: "text-yellow" }}
    />,
  );
}

describe("TrainingSettingsPanel", () => {
  // Given the backend lists keeper skills for Technical, when the panel renders, then it shows them under Goalkeepers beside the outfield list.
  it("shows the backend's goalkeeper attributes for the focus", () => {
    renderPanel("Technical", {
      Technical: focus("Technical", ["passing", "shooting"], ["passing", "handling", "reflexes"]),
    });

    expect(screen.getByText("Goalkeepers:")).toBeInTheDocument();
    expect(screen.getAllByText("handling").length).toBeGreaterThan(0);
    expect(screen.getAllByText("shooting").length).toBeGreaterThan(0);
    expect(screen.getByText(/Goalkeepers: passing, handling, reflexes\./)).toBeInTheDocument();
  });

  // Given a focus whose goalkeeper list equals the outfield one, when the panel renders, then no goalkeeper row appears.
  it("hides the goalkeeper row when goalkeepers train the same attributes", () => {
    renderPanel("Technical", { Technical: focus("Technical", ["passing", "shooting"]) });

    expect(screen.queryByText("Goalkeepers:")).not.toBeInTheDocument();
  });

  it("renders the current training schedule, focus, and applied note", () => {
    render(
      <TrainingSettingsPanel
        currentFocus="Physical"
        currentIntensity="Medium"
        currentSchedule="Balanced"
        isSaving={false}
        todayWeekday={1}
        isTodayTraining={true}
        onSetTraining={vi.fn()}
        onSetSchedule={vi.fn()}
        scheduleIds={["Intense", "Balanced", "Light"]}
        scheduleIcons={{ Intense: "I", Balanced: "B", Light: "L" }}
        scheduleColors={{ Intense: "text-red", Balanced: "text-blue", Light: "text-sky" }}
        dayKeys={["mon", "tue", "wed", "thu", "fri", "sat", "sun"]}
        trainingFocusIds={["Physical", "Technical", "Recovery"]}
        trainingFocusIcons={{ Physical: "P", Technical: "T", Recovery: "R" }}
        focusAttributes={{
          Physical: focus("Physical", ["pace", "stamina"]),
          Technical: focus("Technical", ["passing"]),
          Recovery: focus("Recovery", []),
        }}
        intensityIds={["Low", "Medium", "High"]}
        intensityColors={{ Low: "text-blue", Medium: "text-yellow", High: "text-red" }}
      />,
    );

    expect(screen.getByText("Weekly Schedule")).toBeInTheDocument();
    expect(screen.getByText("Training Focus")).toBeInTheDocument();
    expect(screen.getByText("Four training days, three rest days")).toBeInTheDocument();
    expect(screen.getByText(/Training pace, stamina at Medium.label/)).toBeInTheDocument();
    expect(screen.getByText(/tue is a training day/)).toBeInTheDocument();
  });

  it("wires schedule, focus, and intensity actions through callbacks", () => {
    const onSetTraining = vi.fn();
    const onSetSchedule = vi.fn();

    render(
      <TrainingSettingsPanel
        currentFocus="Physical"
        currentIntensity="Medium"
        currentSchedule="Balanced"
        isSaving={false}
        todayWeekday={1}
        isTodayTraining={true}
        onSetTraining={onSetTraining}
        onSetSchedule={onSetSchedule}
        scheduleIds={["Intense", "Balanced", "Light"]}
        scheduleIcons={{ Intense: "I", Balanced: "B", Light: "L" }}
        scheduleColors={{ Intense: "text-red", Balanced: "text-blue", Light: "text-sky" }}
        dayKeys={["mon", "tue", "wed", "thu", "fri", "sat", "sun"]}
        trainingFocusIds={["Physical", "Technical", "Recovery"]}
        trainingFocusIcons={{ Physical: "P", Technical: "T", Recovery: "R" }}
        focusAttributes={{
          Physical: focus("Physical", ["pace"]),
          Technical: focus("Technical", ["passing"]),
          Recovery: focus("Recovery", []),
        }}
        intensityIds={["Low", "Medium", "High"]}
        intensityColors={{ Low: "text-blue", Medium: "text-yellow", High: "text-red" }}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /Intense/i }));
    fireEvent.click(screen.getByRole("button", { name: /Technical.label/i }));
    fireEvent.click(screen.getByRole("button", { name: /High.label/i }));

    expect(onSetSchedule).toHaveBeenCalledWith("Intense");
    expect(onSetTraining).toHaveBeenCalledWith("Technical", "Medium");
    expect(onSetTraining).toHaveBeenCalledWith("Physical", "High");
  });
});
