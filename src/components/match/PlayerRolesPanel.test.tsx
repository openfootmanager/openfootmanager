import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { PlayerRolesPanel } from "./PlayerRolesPanel";
import type { EnginePlayerData, RoleChangeOutcome } from "./types";

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => {} },
  useTranslation: () => ({
    // Echoes the key, with the player's name where the string takes one, so a test can tell the
    // rows apart without depending on the English wording.
    t: (key: string, opts?: unknown) =>
      typeof opts === "object" && opts !== null && "player" in opts
        ? `${key}:${String((opts as { player: string }).player)}`
        : key,
  }),
}));

const player = (overrides: Partial<EnginePlayerData>): EnginePlayerData =>
  ({
    id: "p",
    name: "Player",
    position: "Midfielder",
    role: "Standard",
    ...overrides,
  }) as EnginePlayerData;

const squad = [
  player({ id: "gk", name: "Keeper", position: "Goalkeeper" }),
  player({ id: "st", name: "Striker", position: "Forward" }),
];

const renderPanel = (onChangeRole = vi.fn().mockResolvedValue({ applied: true })) => {
  render(<PlayerRolesPanel players={squad} onChangeRole={onChangeRole} />);
  fireEvent.click(screen.getByText("match.playerRoles"));
  return onChangeRole;
};

const refused: RoleChangeOutcome = {
  applied: false,
  reason: "be.error.roleNotValidForPosition",
};

const pickPoacher = () => {
  fireEvent.click(screen.getByRole("combobox", { name: "match.roleFor:Striker" }));
  fireEvent.click(screen.getByRole("option", { name: "tactics.playerRoles.Poacher" }));
};

describe("PlayerRolesPanel", () => {
  /**
   * Given two players on the pitch
   * When the roles section is opened
   * Then each has a role picker named after them.
   */
  it("gives each player a picker named after them", () => {
    renderPanel();

    expect(screen.getByRole("combobox", { name: "match.roleFor:Keeper" })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "match.roleFor:Striker" })).toBeInTheDocument();
  });

  /**
   * Given a forward and a goalkeeper
   * When the forward's picker is opened
   * Then it offers forward roles and no goalkeeper role.
   */
  it("offers only the roles that suit the player's position", () => {
    renderPanel();

    fireEvent.click(screen.getByRole("combobox", { name: "match.roleFor:Striker" }));
    const options = within(screen.getByRole("listbox")).getAllByRole("option");
    const names = options.map((option) => option.textContent);

    expect(names).toContain("tactics.playerRoles.Poacher");
    expect(names).not.toContain("tactics.playerRoles.SweeperKeeper");
  });

  /**
   * Given the striker's picker is open
   * When the Poacher role is chosen
   * Then the change is asked for that player and role.
   */
  it("asks for the chosen role for that player", () => {
    const onChangeRole = renderPanel();

    fireEvent.click(screen.getByRole("combobox", { name: "match.roleFor:Striker" }));
    fireEvent.click(screen.getByRole("option", { name: "tactics.playerRoles.Poacher" }));

    expect(onChangeRole).toHaveBeenCalledWith("st", "Poacher");
  });

  /**
   * Given the engine refuses a role
   * When it is chosen
   * Then the refusal is announced with the player's name and the picker still shows their role.
   */
  it("announces a refusal and keeps the player's role", async () => {
    const refused: RoleChangeOutcome = {
      applied: false,
      reason: "be.error.roleNotValidForPosition",
    };
    renderPanel(vi.fn().mockResolvedValue(refused));

    fireEvent.click(screen.getByRole("combobox", { name: "match.roleFor:Striker" }));
    fireEvent.click(screen.getByRole("option", { name: "tactics.playerRoles.Poacher" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("match.roleChangeRefused:Striker");
    expect(screen.getByRole("combobox", { name: "match.roleFor:Striker" })).toHaveTextContent(
      "tactics.playerRoles.Standard",
    );
  });

  /**
   * Given a refusal is showing
   * When the next change is accepted
   * Then the refusal goes away.
   */
  it("clears the refusal once a later change is accepted", async () => {
    const onChangeRole = vi
      .fn()
      .mockResolvedValueOnce({ applied: false, reason: "be.error.roleNotValidForPosition" })
      .mockResolvedValueOnce({ applied: true });
    renderPanel(onChangeRole);
    fireEvent.click(screen.getByRole("combobox", { name: "match.roleFor:Striker" }));
    fireEvent.click(screen.getByRole("option", { name: "tactics.playerRoles.Poacher" }));
    await screen.findByRole("alert");

    fireEvent.click(screen.getByRole("combobox", { name: "match.roleFor:Striker" }));
    fireEvent.click(screen.getByRole("option", { name: "tactics.playerRoles.TargetMan" }));

    await vi.waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
  });

  /**
   * Given the engine refused a role
   * When the same role is refused again
   * Then a new alert is announced, not the old one left unchanged.
   */
  it("announces a repeated refusal again", async () => {
    renderPanel(vi.fn().mockResolvedValue(refused));
    pickPoacher();
    const first = await screen.findByRole("alert");

    pickPoacher();

    await vi.waitFor(() => {
      const second = screen.getByRole("alert");
      expect(second).not.toBe(first);
      expect(first.isConnected).toBe(false);
    });
  });

  /**
   * Given a refusal for the striker
   * When it is showing
   * Then the striker's picker is described by it and the keeper's is not, and focus stays on the
   *      striker's picker.
   */
  it("ties the refusal to the picker that caused it and leaves focus there", async () => {
    renderPanel(vi.fn().mockResolvedValue(refused));
    const striker = screen.getByRole("combobox", { name: "match.roleFor:Striker" });
    striker.focus();

    pickPoacher();
    await screen.findByRole("alert");

    expect(striker).toHaveAccessibleDescription(/match\.roleChangeRefused:Striker/);
    expect(
      screen.getByRole("combobox", { name: "match.roleFor:Keeper" }),
    ).not.toHaveAccessibleDescription();
    expect(striker).toHaveFocus();
  });

  /**
   * Given a refusal is showing
   * When the roles section is closed
   * Then the stale refusal is gone.
   */
  it("drops a refusal when the section is closed", async () => {
    renderPanel(vi.fn().mockResolvedValue(refused));
    pickPoacher();
    await screen.findByRole("alert");

    fireEvent.click(screen.getByText("match.playerRoles"));

    await vi.waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
  });

  /**
   * Given two role changes made quickly, the first refused but answered last
   * When the answers arrive newest first
   * Then the older refusal does not appear: the newer, accepted change decides what is shown.
   */
  it("ignores a stale refusal that arrives after a newer change", async () => {
    let answerFirst!: (outcome: RoleChangeOutcome) => void;
    const onChangeRole = vi
      .fn()
      .mockImplementationOnce(
        () => new Promise<RoleChangeOutcome>((resolve) => (answerFirst = resolve)),
      )
      .mockResolvedValueOnce({ applied: true });
    renderPanel(onChangeRole);

    pickPoacher();
    fireEvent.click(screen.getByRole("combobox", { name: "match.roleFor:Striker" }));
    fireEvent.click(screen.getByRole("option", { name: "tactics.playerRoles.TargetMan" }));
    await vi.waitFor(() => expect(onChangeRole).toHaveBeenCalledTimes(2));
    answerFirst(refused);
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  /**
   * Given a role change still pending when the section is closed
   * When the engine refuses it after the close, and the section is opened again
   * Then no refusal shows: closing put the change out of date.
   */
  it("shows no refusal for a change answered after the section was closed", async () => {
    let answer!: (outcome: RoleChangeOutcome) => void;
    renderPanel(
      vi
        .fn()
        .mockImplementationOnce(
          () => new Promise<RoleChangeOutcome>((resolve) => (answer = resolve)),
        ),
    );
    pickPoacher();

    fireEvent.click(screen.getByText("match.playerRoles"));
    await new Promise((resolve) => setTimeout(resolve, 0));
    answer(refused);
    await new Promise((resolve) => setTimeout(resolve, 0));
    fireEvent.click(screen.getByText("match.playerRoles"));
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });
});
