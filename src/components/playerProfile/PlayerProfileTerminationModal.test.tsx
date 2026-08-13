import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import PlayerProfileTerminationModal from "./PlayerProfileTerminationModal";
import type { ContractTerminationPreviewData } from "../../services/contractService";

function translate(
  key: string,
  params?: Record<string, string | number>,
): string {
  if (key === "playerProfile.terminateContractTitle")
    return "Terminate Contract";
  if (key === "playerProfile.terminateContractBody")
    return `Release ${params?.name} immediately.`;
  if (key === "playerProfile.terminationSeverance") return "Severance";
  if (key === "playerProfile.projectedHealthyPlayers")
    return "Healthy players after release";
  if (key === "playerProfile.terminationUnsafe")
    return "This release would leave the squad unable to field a matchday XI.";
  if (key === "playerProfile.confirmTerminateContract")
    return "Terminate Contract";
  if (key === "common.cancel") return "Cancel";
  if (key === "common.loading") return "Loading...";
  return key;
}

function buildPreview(
  overrides: Partial<ContractTerminationPreviewData["squad_safety"]> = {},
): ContractTerminationPreviewData {
  return {
    player_id: "player-1",
    player_name: "John Smith",
    severance_cost: 250_000,
    squad_safety: {
      team_id: "team-1",
      projected_roster_size: 24,
      healthy_players: 18,
      healthy_goalkeepers: 2,
      effective_xi_size: 11,
      can_field_matchday_squad: true,
      missing_reasons: [],
      ...overrides,
    },
  };
}

function renderModal(
  props: Partial<
    React.ComponentProps<typeof PlayerProfileTerminationModal>
  > = {},
) {
  const onCancel = vi.fn();
  const onConfirm = vi.fn();

  render(
    <PlayerProfileTerminationModal
      show
      playerName="John Smith"
      t={translate}
      preview={buildPreview()}
      errorMessage={null}
      submitting={false}
      onCancel={onCancel}
      onConfirm={onConfirm}
      {...props}
    />,
  );

  return { onCancel, onConfirm };
}

describe("PlayerProfileTerminationModal", () => {
  it("renders nothing when hidden", () => {
    renderModal({ show: false });

    expect(screen.queryByText("Terminate Contract")).toBeNull();
  });

  it("shows the severance cost and squad impact once the preview arrives", () => {
    renderModal();

    expect(
      screen.getByText("Release John Smith immediately."),
    ).toBeInTheDocument();
    expect(screen.getByText("Severance")).toBeInTheDocument();
    expect(screen.getByText("18/11")).toBeInTheDocument();
  });

  it("waits on a loading message while the preview is in flight", () => {
    renderModal({ preview: null });

    expect(screen.getByText("Loading...")).toBeInTheDocument();
    expect(screen.queryByText("Severance")).toBeNull();
  });

  // The whole point of the preview: a release that would leave the club unable
  // to field a matchday XI must not be confirmable.
  it("blocks confirmation when the squad could no longer field a matchday XI", () => {
    const { onConfirm } = renderModal({
      preview: buildPreview({ can_field_matchday_squad: false }),
    });

    expect(
      screen.getByText(
        "This release would leave the squad unable to field a matchday XI.",
      ),
    ).toBeInTheDocument();

    const confirm = screen.getByRole("button", { name: "Terminate Contract" });
    expect(confirm).toBeDisabled();

    fireEvent.click(confirm);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("blocks confirmation while the preview is still loading", () => {
    renderModal({ preview: null });

    expect(
      screen.getByRole("button", { name: "Terminate Contract" }),
    ).toBeDisabled();
  });

  it("confirms and cancels through its callbacks", () => {
    const { onCancel, onConfirm } = renderModal();

    fireEvent.click(screen.getByRole("button", { name: "Terminate Contract" }));
    expect(onConfirm).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("disables both actions while a termination is submitting", () => {
    renderModal({ submitting: true });

    expect(screen.getByRole("button", { name: "Cancel" })).toBeDisabled();
    expect(
      screen.getByRole("button", { name: "Terminate Contract" }),
    ).toBeDisabled();
  });

  it("surfaces a backend failure message", () => {
    renderModal({ errorMessage: "Could not release this player." });

    expect(
      screen.getByText("Could not release this player."),
    ).toBeInTheDocument();
  });
});
