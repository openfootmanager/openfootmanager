import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { CompetitionPreviewCard } from "./CompetitionPreviewCard";
import { emptyCompetition, emptyPlayer, emptyTeam } from "./helpers";
import { PlayerPreviewCard } from "./PlayerPreviewCard";
import { TeamPreviewCard } from "./TeamPreviewCard";

vi.mock("../../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});

// Keys render verbatim, so a placeholder that goes through t() shows its key and one written as
// JSX text shows English: the English text alone would pass under the English locale either way.
vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

describe("preview card placeholders", () => {
  // Given a team just added in the package editor, with no name yet
  // When its preview card renders
  // Then the placeholder is the translated one, not English text (#1185)
  it("translates the placeholder of a team with no name", () => {
    render(<TeamPreviewCard team={emptyTeam()} logoDataUrl={null} />);

    expect(screen.getByText("worldEditor.previewPlaceholder.team")).toBeInTheDocument();
    expect(screen.queryByText("New Team")).not.toBeInTheDocument();
  });

  // Given a player just added in the package editor, with no name yet
  // When its preview card renders
  // Then the placeholder is the translated one, not English text
  it("translates the placeholder of a player with no name", () => {
    render(<PlayerPreviewCard editing={emptyPlayer()} photoDataUrl={null} />);

    expect(screen.getByText("worldEditor.previewPlaceholder.player")).toBeInTheDocument();
    expect(screen.queryByText("New Player")).not.toBeInTheDocument();
  });

  // Given a competition just added in the package editor, with no name yet
  // When its preview card renders
  // Then the placeholder is the translated one, not English text
  it("translates the placeholder of a competition with no name", () => {
    render(<CompetitionPreviewCard competition={emptyCompetition()} logoDataUrl={null} />);

    expect(screen.getByText("worldEditor.previewPlaceholder.competition")).toBeInTheDocument();
    expect(screen.queryByText("New Competition")).not.toBeInTheDocument();
  });
});
