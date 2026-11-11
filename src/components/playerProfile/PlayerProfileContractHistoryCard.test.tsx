import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { ContractHistoryRow } from "./PlayerProfile.contractHistory";
import PlayerProfileContractHistoryCard from "./PlayerProfileContractHistoryCard";

const t = (key: string, options?: Record<string, string | number>) =>
  options ? `${key} ${JSON.stringify(options)}` : key;

const row = (patch: Partial<ContractHistoryRow>): ContractHistoryRow => ({
  key: "r",
  clubName: "North FC",
  clubId: "north",
  fromClubName: null,
  start: "2031-07-01",
  end: "2033-06-30",
  weeklyWage: 10_000,
  source: "renewal",
  endedBy: null,
  replacedOn: null,
  inForce: true,
  ...patch,
});

const RENEWED = row({ key: "a" });
const EXPIRED = row({
  key: "b",
  start: "2029-07-01",
  end: "2031-06-30",
  weeklyWage: 7_000,
  source: "transfer",
  fromClubName: "South FC",
  endedBy: "expired",
  inForce: false,
});

function renderCard(
  rows: ContractHistoryRow[],
  patch: { showsWages?: boolean; onOpenTeam?: (id: string) => void } = {},
) {
  return render(
    <PlayerProfileContractHistoryCard
      language="en"
      rows={rows}
      showsWages={patch.showsWages ?? true}
      onOpenTeam={patch.onOpenTeam}
      t={t}
    />,
  );
}

describe("PlayerProfileContractHistoryCard", () => {
  /**
   * Given two contracts he has had, newest first
   * When the card is drawn for his own player
   * Then each row leads with its date range, then the club and how he joined it, then the wage,
   *      and the one that ended says how in the same note
   */
  it("leads each row with its date range, then the club and how he joined, then the wage", () => {
    renderCard([RENEWED, EXPIRED]);

    expect(screen.getByText("playerProfile.contractsHad")).toBeInTheDocument();
    const rows = screen.getAllByRole("row").slice(1);
    expect(rows).toHaveLength(2);
    const [current, closed] = rows;
    expect(current.firstElementChild).toHaveTextContent(
      'playerProfile.contractSheet.range {"start":"Jul 1, 2031","end":"Jun 30, 2033"}',
    );
    expect(
      within(current).getByText("playerProfile.contractSheet.how.renewal"),
    ).toBeInTheDocument();
    expect(within(current).getByText(/10,000/)).toBeInTheDocument();
    expect(closed.firstElementChild).toHaveTextContent(
      '"start":"Jul 1, 2029","end":"Jun 30, 2031"',
    );
    expect(
      within(closed).getByText('playerProfile.contractSheet.how.transfer {"club":"South FC"}'),
    ).toBeInTheDocument();
    expect(within(closed).getByText(/7,000/)).toBeInTheDocument();
    expect(within(closed).getByText(/playerProfile\.movementReleasedExpired/)).toBeInTheDocument();
    expect(within(current).queryByText(/movementReleasedExpired/)).toBeNull();
  });

  /**
   * Given the club of each contract, and a page that can open a club
   * When the card is drawn
   * Then the club is a link that opens it; with no way to open a club it is plain text
   */
  it("makes the club a link when a club can be opened", () => {
    const onOpenTeam = vi.fn();
    const { unmount } = renderCard([RENEWED], { onOpenTeam });

    fireEvent.click(screen.getByRole("button", { name: "North FC" }));
    expect(onOpenTeam).toHaveBeenCalledWith("north");
    unmount();

    renderCard([RENEWED]);
    expect(screen.queryByRole("button", { name: "North FC" })).toBeNull();
    expect(screen.getByText("North FC")).toBeInTheDocument();
  });

  /**
   * Given the contract he is on now and one that is over
   * When the card is drawn
   * Then the current row is tinted and the other is not
   */
  it("tints the contract he is on now", () => {
    renderCard([RENEWED, EXPIRED]);

    const [current, closed] = screen.getAllByRole("row").slice(1);
    expect(current.className).toContain("bg-positive-tint");
    expect(closed.className).not.toContain("bg-positive-tint");
  });

  /**
   * Given each way a contract can have begun
   * When the card is drawn
   * Then each says how in words: a transfer with no known club, a free-agent signing and a first
   *      contract included
   */
  it("says how each contract began", () => {
    renderCard([
      row({ key: "1", source: "transfer", fromClubName: null, inForce: false }),
      row({ key: "2", source: "free_agent", inForce: false }),
      row({ key: "3", source: "initial", inForce: false }),
    ]);

    expect(screen.getByText("playerProfile.contractSheet.how.transferNoClub")).toBeInTheDocument();
    expect(screen.getByText("playerProfile.contractSheet.how.free_agent")).toBeInTheDocument();
    expect(screen.getByText("playerProfile.contractSheet.how.initial")).toBeInTheDocument();
  });

  /**
   * Given a rival's contracts, whose wages the club may not see
   * When the card is drawn
   * Then the dates stay and no wage is shown
   */
  it("leaves the wage out when the club may not see it", () => {
    renderCard([RENEWED], { showsWages: false });

    expect(screen.queryByText(/10,000/)).toBeNull();
    expect(screen.getByText(/contractSheet\.range/)).toBeInTheDocument();
  });

  /**
   * Given a contract from a save made before starts were recorded
   * When the card is drawn
   * Then it says so in words and shows its end alone, never a start it does not have
   */
  it("shows an unknown start as an end alone, with the legacy label", () => {
    renderCard([row({ start: null, source: "legacy_migrated" })]);

    expect(screen.getByText("playerProfile.movementLegacyContract")).toBeInTheDocument();
    expect(
      screen.getByText('playerProfile.contractSheet.until {"end":"Jun 30, 2033"}'),
    ).toBeInTheDocument();
  });

  /** Given no contract on record, then the card says so. */
  it("says when there is no contract on record", () => {
    renderCard([]);

    expect(screen.getByText("playerProfile.noContractsHad")).toBeInTheDocument();
    expect(screen.queryByRole("table")).toBeNull();
  });
});
