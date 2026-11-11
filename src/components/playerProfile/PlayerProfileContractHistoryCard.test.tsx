import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { ContractHistoryRow } from "./PlayerProfile.contractHistory";
import PlayerProfileContractHistoryCard from "./PlayerProfileContractHistoryCard";

const t = (key: string, options?: Record<string, string | number>) =>
  options ? `${key} ${JSON.stringify(options)}` : key;

const row = (patch: Partial<ContractHistoryRow>): ContractHistoryRow => ({
  key: "r",
  clubName: "North FC",
  start: "2031-07-01",
  end: "2033-06-30",
  weeklyWage: 10_000,
  source: "renewal",
  endedBy: null,
  replacedOn: null,
  inForce: true,
  ...patch,
});

const RENEWED = row({ key: "a", start: "2031-07-01", end: "2033-06-30", weeklyWage: 10_000 });
const EXPIRED = row({
  key: "b",
  start: "2029-07-01",
  end: "2031-06-30",
  weeklyWage: 7_000,
  source: "transfer",
  endedBy: "expired",
  inForce: false,
});

describe("PlayerProfileContractHistoryCard", () => {
  /**
   * Given two contracts he has had, newest first
   * When the card is drawn for his own player
   * Then it is a table with a row per contract, each with its club, span and wage, and the one that
   *      ended says how
   */
  it("lists each contract with its club, span, wage and how it ended", () => {
    render(<PlayerProfileContractHistoryCard rows={[RENEWED, EXPIRED]} showsWages t={t} />);

    expect(screen.getByText("playerProfile.contractsHad")).toBeInTheDocument();
    const rows = screen.getAllByRole("row").slice(1);
    expect(rows).toHaveLength(2);
    expect(within(rows[0]).getByText(/10,000/)).toBeInTheDocument();
    expect(within(rows[1]).getByText(/7,000/)).toBeInTheDocument();
    expect(
      within(rows[1]).getByText(
        'playerProfile.movementContractRange {"start":"2029-07-01","end":"2031-06-30"}',
      ),
    ).toBeInTheDocument();
    expect(within(rows[1]).getByText("playerProfile.movementReleasedExpired")).toBeInTheDocument();
    expect(within(rows[0]).queryByText("playerProfile.movementReleasedExpired")).toBeNull();
  });

  /**
   * Given a rival's contracts, whose wages the club may not see
   * When the card is drawn
   * Then the dates stay and no wage, and no wage column, is shown
   */
  it("leaves the wage out when the club may not see it", () => {
    render(<PlayerProfileContractHistoryCard rows={[RENEWED]} showsWages={false} t={t} />);

    expect(screen.queryByText(/10,000/)).toBeNull();
    expect(
      screen.queryByRole("columnheader", { name: "playerProfile.contractSheet.weeklyWage" }),
    ).toBeNull();
    expect(
      screen.getByText(
        'playerProfile.movementContractRange {"start":"2031-07-01","end":"2033-06-30"}',
      ),
    ).toBeInTheDocument();
  });

  /**
   * Given a contract from a save made before starts were recorded
   * When the card is drawn
   * Then it says so in words and shows its end alone, never a start it does not have
   */
  it("shows an unknown start as an end alone, with the legacy label", () => {
    render(
      <PlayerProfileContractHistoryCard
        rows={[row({ start: null, source: "legacy_migrated" })]}
        showsWages
        t={t}
      />,
    );

    expect(screen.getByText("playerProfile.movementLegacyContract")).toBeInTheDocument();
    expect(
      screen.getByText('playerProfile.movementContractUntil {"end":"2033-06-30"}'),
    ).toBeInTheDocument();
  });

  /** Given no contract on record, then the card says so. */
  it("says when there is no contract on record", () => {
    render(<PlayerProfileContractHistoryCard rows={[]} showsWages t={t} />);

    expect(screen.getByText("playerProfile.noContractsHad")).toBeInTheDocument();
    expect(screen.queryByRole("table")).toBeNull();
  });
});
