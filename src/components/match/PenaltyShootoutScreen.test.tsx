import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { KickRow } from "./PenaltyShootoutScreen";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (k: string) => k }),
}));

describe("KickRow", function (): void {
  it("shows future cells for untaken kicks", function (): void {
    render(<KickRow label="Home" taken={0} scored={0} maxRounds={5} />);
    const cells = screen.getAllByText("?");
    expect(cells).toHaveLength(5);
  });

  it("renders goals as ⚽ and misses as ✗", function (): void {
    // 3 taken: first two scored, third missed
    render(<KickRow label="Home" taken={3} scored={2} maxRounds={5} />);
    expect(screen.getAllByText("⚽")).toHaveLength(2);
    expect(screen.getAllByText("✗")).toHaveLength(1);
    expect(screen.getAllByText("?")).toHaveLength(2);
  });

  it("does not mark a miss as a goal (isGoal = i < scored only)", function (): void {
    // taken=2, scored=1 — i=0 is a goal, i=1 is a miss
    render(<KickRow label="Away" taken={2} scored={1} maxRounds={5} />);
    expect(screen.getAllByText("⚽")).toHaveLength(1);
    expect(screen.getAllByText("✗")).toHaveLength(1);
    expect(screen.getAllByText("?")).toHaveLength(3);
  });

  it("expands cells beyond maxRounds when taken exceeds it", function (): void {
    render(<KickRow label="Home" taken={7} scored={4} maxRounds={5} />);
    expect(screen.getAllByText("⚽")).toHaveLength(4);
    expect(screen.getAllByText("✗")).toHaveLength(3);
    expect(screen.queryAllByText("?")).toHaveLength(0);
  });
});
