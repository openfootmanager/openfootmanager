import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { PlayerForm } from "./PlayerForm";
import { emptyPlayer, emptyAttributes, emptyTeam, WIRE_MAX } from "./helpers";
import type { PlayerCareerEntryDef, PlayerDef, TeamDef } from "./types";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    // Echo the key back so a test can name the string it expects without
    // depending on the English wording, which translators may change.
    t: (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key,
    i18n: { language: "en" },
  }),
}));

function renderForm(player: Partial<PlayerDef> = {}, teams: TeamDef[] = []) {
  const updateField = vi.fn();
  render(
    <PlayerForm
      editing={{ ...emptyPlayer(), ...player }}
      editingIndex={0}
      isBusy={false}
      teams={teams}
      onBack={() => {}}
      onSave={() => {}}
      updateField={updateField}
      onAssetError={() => {}}
    />,
  );
  return { updateField };
}

function potentialInput() {
  return screen.getByRole("spinbutton", { name: "worldEditor.playerPotential" });
}

describe("PlayerForm potential", () => {
  it("shows an authored ceiling", () => {
    renderForm({ overall: 55, potential: 92 });

    expect(potentialInput()).toHaveValue(92);
  });

  it("stays available when ability is given as attributes", () => {
    // A ceiling is orthogonal to how current ability was expressed, so gating it
    // on the overall/attributes toggle would hide it from exactly the authors
    // exercising the most precise control.
    renderForm({ overall: null, potential: 88, attributes: emptyAttributes() });

    expect(potentialInput()).toHaveValue(88);
  });

  it("hands a blank field back to the engine's roll", () => {
    const { updateField } = renderForm({ overall: 55, potential: 92 });

    fireEvent.change(potentialInput(), { target: { value: "" } });

    expect(updateField).toHaveBeenCalledWith("potential", null);
  });

  it("keeps a typed ceiling as written rather than raising it to ability", () => {
    // Below-ability is a real authoring mistake, but the package validator is
    // what reports it. Repairing it here would hide the error the author needs.
    const { updateField } = renderForm({ overall: 70 });

    fireEvent.change(potentialInput(), { target: { value: "60" } });

    expect(updateField).toHaveBeenCalledWith("potential", 60);
  });

  it("clamps to the legal range", () => {
    const { updateField } = renderForm({ overall: 70 });

    fireEvent.change(potentialInput(), { target: { value: "140" } });

    expect(updateField).toHaveBeenCalledWith("potential", 99);
  });

  it("clamps scientific notation rather than truncating it", () => {
    // `type="number"` accepts `1e2` as a valid value, but parseInt stops at the
    // `e` and yields 1 — so a user asking for 100 silently got the opposite end
    // of the range.
    const { updateField } = renderForm({ overall: 70 });

    fireEvent.change(potentialInput(), { target: { value: "1e2" } });

    expect(updateField).toHaveBeenCalledWith("potential", 99);
  });

  it("ignores a value that is not a number at all", () => {
    const { updateField } = renderForm({ overall: 70, potential: 80 });

    fireEvent.change(potentialInput(), { target: { value: "abc" } });

    // Null, not 1: an unparseable entry is an absent ceiling, not the worst
    // possible one.
    expect(updateField).toHaveBeenCalledWith("potential", null);
  });
});

const REAL_MADRID: TeamDef = { ...emptyTeam(), id: "real-madrid", name: "Real Madrid" };

function numberField(name: string) {
  return screen.getByRole("spinbutton", { name });
}

describe("PlayerForm age", () => {
  it("shows an authored age, which the format carried but the form could not set", () => {
    renderForm({ age: 25 });

    expect(numberField("worldEditor.playerAge")).toHaveValue(25);
  });

  it("writes a typed age and hands a blank back to the engine", () => {
    const { updateField } = renderForm({ age: 25 });

    fireEvent.change(numberField("worldEditor.playerAge"), { target: { value: "31" } });
    expect(updateField).toHaveBeenLastCalledWith("age", 31);

    fireEvent.change(numberField("worldEditor.playerAge"), { target: { value: "" } });
    expect(updateField).toHaveBeenLastCalledWith("age", null);
  });

  it("holds an age to what its field stores, which is wider than a byte", () => {
    // Age is a u32 in the package, so 300 is a value that loads; capping it at 255
    // would change what the author typed without telling them.
    const { updateField } = renderForm({ age: 25 });

    fireEvent.change(numberField("worldEditor.playerAge"), { target: { value: "300" } });
    expect(updateField).toHaveBeenLastCalledWith("age", 300);

    fireEvent.change(numberField("worldEditor.playerAge"), { target: { value: "99999999999" } });
    expect(updateField).toHaveBeenLastCalledWith("age", WIRE_MAX.u32);
  });

  it("does not touch the date of birth when an age is typed", () => {
    // Which of the two wins is generation's rule. Clearing one here when the other
    // is typed would be a second copy of it, so the form just writes what it is given.
    const { updateField } = renderForm({ dateOfBirth: "1990-05-01" });

    fireEvent.change(numberField("worldEditor.playerAge"), { target: { value: "31" } });

    expect(updateField).toHaveBeenCalledTimes(1);
    expect(updateField).toHaveBeenCalledWith("age", 31);
  });
});

// Each of these is a number the backend validator holds to a game range. The form
// holds them only to what their type can store, so a typo cannot make the file
// fail to load, and leaves the range to the validator, which says so in words.
describe.each([
  ["contractLength", "worldEditor.playerContractLength", WIRE_MAX.u32],
  ["wage", "common.wage", WIRE_MAX.u32],
  ["value", "common.value", WIRE_MAX.safe],
  ["condition", "common.condition", WIRE_MAX.u8],
  ["morale", "common.morale", WIRE_MAX.u8],
  ["weakFoot", "common.weakFoot", WIRE_MAX.u8],
] as const)("PlayerForm %s", (field, label, max) => {
  it("shows an authored value", () => {
    renderForm({ [field]: 3 } as Partial<PlayerDef>);

    expect(numberField(label)).toHaveValue(3);
  });

  it("writes a typed whole number", () => {
    const { updateField } = renderForm();

    fireEvent.change(numberField(label), { target: { value: "42" } });

    expect(updateField).toHaveBeenCalledWith(field, 42);
  });

  it("hands a blank or unparseable entry back to the engine rather than writing zero", () => {
    const { updateField } = renderForm({ [field]: 3 } as Partial<PlayerDef>);

    fireEvent.change(numberField(label), { target: { value: "" } });
    expect(updateField).toHaveBeenLastCalledWith(field, null);

    fireEvent.change(numberField(label), { target: { value: "abc" } });
    expect(updateField).toHaveBeenLastCalledWith(field, null);
  });

  it("keeps a real zero", () => {
    const { updateField } = renderForm();

    fireEvent.change(numberField(label), { target: { value: "0" } });

    expect(updateField).toHaveBeenCalledWith(field, 0);
  });

  it("leaves a number past the game's range for the validator to report", () => {
    // 150 is not a legal condition or weak foot, but clamping it here would quietly
    // repair the mistake the validator exists to show.
    const { updateField } = renderForm();

    fireEvent.change(numberField(label), { target: { value: "150" } });

    expect(updateField).toHaveBeenCalledWith(field, 150);
  });

  it("never writes what its stored type cannot load", () => {
    const { updateField } = renderForm();

    fireEvent.change(numberField(label), { target: { value: "-5" } });
    expect(updateField).toHaveBeenLastCalledWith(field, 0);

    fireEvent.change(numberField(label), { target: { value: "1e30" } });
    expect(updateField).toHaveBeenLastCalledWith(field, max);
  });
});

describe("PlayerForm contract dates", () => {
  it.each([
    ["contractStart", "worldEditor.playerContractStart"],
    ["contractEnd", "worldEditor.playerContractEnd"],
  ] as const)("shows an authored %s", (field, label) => {
    renderForm({ [field]: "2031-03-15" } as Partial<PlayerDef>);

    const group = within(screen.getByRole("group", { name: label }));
    expect(group.getByRole("textbox", { name: "date.dayLabel" })).toHaveValue("15");
    expect(group.getByRole("textbox", { name: "date.yearLabel" })).toHaveValue("2031");
  });

  it.each([
    ["contractStart", "worldEditor.playerContractStart"],
    ["contractEnd", "worldEditor.playerContractEnd"],
  ] as const)("writes a changed date to %s and to nothing else", (field, label) => {
    const { updateField } = renderForm({ [field]: "2031-03-15" } as Partial<PlayerDef>);

    const group = within(screen.getByRole("group", { name: label }));
    fireEvent.change(group.getByRole("textbox", { name: "date.yearLabel" }), {
      target: { value: "2032" },
    });

    expect(updateField).toHaveBeenCalledTimes(1);
    expect(updateField).toHaveBeenCalledWith(field, "2032-03-15");
  });

  it("does not work out an end date from a start and a length", () => {
    // What a length resolves to is the backend's rule, and it depends on the year a
    // career opens in, which the editor does not know. Showing a guess here would be
    // a second, wrong, copy of it.
    renderForm({ contractStart: "2024-01-15", contractLength: 2 });

    const end = within(screen.getByRole("group", { name: "worldEditor.playerContractEnd" }));
    expect(end.getByRole("textbox", { name: "date.dayLabel" })).toHaveValue("");
    expect(end.getByRole("textbox", { name: "date.yearLabel" })).toHaveValue("");
  });
});

describe("PlayerForm alternate positions", () => {
  const chips = () =>
    within(screen.getByRole("group", { name: "worldEditor.playerAlternatePositions" }));

  it("marks the chosen positions as pressed and the rest as not", () => {
    renderForm({ alternatePositions: ["LeftWinger"] });

    expect(chips().getByRole("button", { name: "common.positions.LeftWinger" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(chips().getByRole("button", { name: "common.positions.Striker" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });

  it("adds a position when an unpressed one is pressed", () => {
    const { updateField } = renderForm({ alternatePositions: ["LeftWinger"] });

    fireEvent.click(chips().getByRole("button", { name: "common.positions.Striker" }));

    expect(updateField).toHaveBeenCalledWith("alternatePositions", ["LeftWinger", "Striker"]);
  });

  it("removes a position when a pressed one is pressed", () => {
    const { updateField } = renderForm({ alternatePositions: ["LeftWinger", "Striker"] });

    fireEvent.click(chips().getByRole("button", { name: "common.positions.LeftWinger" }));

    expect(updateField).toHaveBeenCalledWith("alternatePositions", ["Striker"]);
  });

  it("does not modify the list it was handed", () => {
    // A store array mutated in place desyncs the UI silently. Frozen, a mutation throws.
    const given = Object.freeze(["LeftWinger"]) as unknown as PlayerDef["alternatePositions"];
    const { updateField } = renderForm({ alternatePositions: given });

    fireEvent.click(chips().getByRole("button", { name: "common.positions.Striker" }));

    expect(updateField).toHaveBeenCalledWith("alternatePositions", ["LeftWinger", "Striker"]);
    expect(given).toEqual(["LeftWinger"]);
  });
});

describe("PlayerForm career history", () => {
  const entry = (extra: Partial<PlayerCareerEntryDef> = {}): PlayerCareerEntryDef => ({
    season: 2019,
    teamName: "Juventus",
    appearances: 30,
    goals: 10,
    assists: 5,
    ...extra,
  });
  const rows = () => screen.getAllByRole("group", { name: "worldEditor.careerEntryRow" });
  const inRow = (index: number) => within(rows()[index]);

  it("adds a blank entry", () => {
    const { updateField } = renderForm();

    fireEvent.click(screen.getByRole("button", { name: "worldEditor.addCareerEntry" }));

    expect(updateField).toHaveBeenCalledWith("careerHistory", [
      { season: 0, teamName: "", appearances: 0, goals: 0, assists: 0 },
    ]);
  });

  it("shows each entry in its own named group, so its fields are not all just 'Club name'", () => {
    renderForm({ careerHistory: [entry(), entry({ season: 2020, teamName: "Real Madrid" })] });

    expect(rows()).toHaveLength(2);
    expect(inRow(0).getByRole("textbox", { name: "worldEditor.careerEntryClubName" })).toHaveValue(
      "Juventus",
    );
    expect(inRow(1).getByRole("textbox", { name: "worldEditor.careerEntryClubName" })).toHaveValue(
      "Real Madrid",
    );
    expect(inRow(0).getByRole("spinbutton", { name: "playerProfile.season" })).toHaveValue(2019);
    expect(inRow(0).getByRole("spinbutton", { name: "playerProfile.goals" })).toHaveValue(10);
  });

  /**
   * The point of a free-text club: a package that defines only Real Madrid must still
   * be able to record a spell at Juventus. The club picker elsewhere on this form
   * only offers free text when a package has *no* teams, so this is what it gets
   * wrong if copied, and what has to be said outright.
   */
  it("lets a club the package does not define be typed even when the package has teams", () => {
    const { updateField } = renderForm({ careerHistory: [entry({ teamName: "" })] }, [REAL_MADRID]);

    fireEvent.change(inRow(0).getByRole("textbox", { name: "worldEditor.careerEntryClubName" }), {
      target: { value: "Juventus" },
    });

    const [, written] = updateField.mock.calls[0] as [string, PlayerCareerEntryDef[]];
    expect(written[0].teamName).toBe("Juventus");
    expect(written[0].teamId ?? null).toBeNull();
  });

  it("picks a package team and names the entry after it when it has no name yet", () => {
    const { updateField } = renderForm({ careerHistory: [entry({ teamName: "" })] }, [REAL_MADRID]);

    const picker = inRow(0).getByRole("button", { name: "worldEditor.careerEntryPackageTeam" });
    fireEvent.mouseDown(picker);
    const search = screen.getByRole("combobox", { name: "worldEditor.careerEntryPackageTeam" });
    fireEvent.keyDown(search, { key: "ArrowDown" });
    fireEvent.keyDown(search, { key: "Enter" });

    const [, written] = updateField.mock.calls[0] as [string, PlayerCareerEntryDef[]];
    expect(written[0].teamId).toBe("real-madrid");
    expect(written[0].teamName).toBe("Real Madrid");
  });

  it("keeps a name that was already typed when a package team is picked", () => {
    const { updateField } = renderForm({ careerHistory: [entry({ teamName: "Los Blancos" })] }, [
      REAL_MADRID,
    ]);

    const picker = inRow(0).getByRole("button", { name: "worldEditor.careerEntryPackageTeam" });
    fireEvent.mouseDown(picker);
    const search = screen.getByRole("combobox", { name: "worldEditor.careerEntryPackageTeam" });
    fireEvent.keyDown(search, { key: "ArrowDown" });
    fireEvent.keyDown(search, { key: "Enter" });

    const [, written] = updateField.mock.calls[0] as [string, PlayerCareerEntryDef[]];
    expect(written[0].teamId).toBe("real-madrid");
    expect(written[0].teamName).toBe("Los Blancos");
  });

  it("clears the team id, and leaves the name, when 'not in this package' is chosen", () => {
    const { updateField } = renderForm(
      { careerHistory: [entry({ teamId: "real-madrid", teamName: "Real Madrid" })] },
      [REAL_MADRID],
    );

    const picker = inRow(0).getByRole("button", { name: "worldEditor.careerEntryPackageTeam" });
    fireEvent.mouseDown(picker);
    fireEvent.keyDown(
      screen.getByRole("combobox", { name: "worldEditor.careerEntryPackageTeam" }),
      {
        key: "Enter",
      },
    );

    const [, written] = updateField.mock.calls[0] as [string, PlayerCareerEntryDef[]];
    expect(written[0].teamId ?? null).toBeNull();
    expect(written[0].teamName).toBe("Real Madrid");
  });

  it("changes only the entry that was edited", () => {
    const first = entry();
    const second = entry({ season: 2020, teamName: "Real Madrid", goals: 1 });
    const { updateField } = renderForm({ careerHistory: [first, second] });

    fireEvent.change(inRow(1).getByRole("spinbutton", { name: "playerProfile.goals" }), {
      target: { value: "7" },
    });

    expect(updateField).toHaveBeenCalledWith("careerHistory", [first, { ...second, goals: 7 }]);
  });

  it("writes counts as whole numbers, zero when blank, never negative", () => {
    const { updateField } = renderForm({ careerHistory: [entry()] });
    const goals = () => inRow(0).getByRole("spinbutton", { name: "playerProfile.goals" });
    const lastGoals = () => {
      const { calls } = updateField.mock;
      const last = calls[calls.length - 1] as [string, PlayerCareerEntryDef[]];
      return last[1][0].goals;
    };

    fireEvent.change(goals(), { target: { value: "" } });
    expect(lastGoals()).toBe(0);
    fireEvent.change(goals(), { target: { value: "12.6" } });
    expect(lastGoals()).toBe(13);
    fireEvent.change(goals(), { target: { value: "-3" } });
    expect(lastGoals()).toBe(0);
  });

  it("removes just the chosen entry", () => {
    const first = entry();
    const second = entry({ season: 2020, teamName: "Real Madrid" });
    const { updateField } = renderForm({ careerHistory: [first, second] });

    fireEvent.click(inRow(0).getByRole("button", { name: "worldEditor.removeCareerEntry" }));

    expect(updateField).toHaveBeenCalledWith("careerHistory", [second]);
  });

  it("does not modify the list it was handed", () => {
    const given = Object.freeze([Object.freeze(entry())]) as unknown as PlayerCareerEntryDef[];
    const { updateField } = renderForm({ careerHistory: given });

    fireEvent.click(screen.getByRole("button", { name: "worldEditor.addCareerEntry" }));

    expect(updateField).toHaveBeenCalled();
    expect(given).toHaveLength(1);
  });
});
