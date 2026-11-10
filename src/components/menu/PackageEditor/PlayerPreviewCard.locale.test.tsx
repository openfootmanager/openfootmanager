import { render, screen } from "@testing-library/react";
import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";

import i18n, { i18nReady } from "../../../i18n";
import { emptyPlayer } from "./helpers";
import { PlayerPreviewCard } from "./PlayerPreviewCard";

vi.mock("../../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});

// The real i18n instance, not a mocked `t`: the point is what a German-speaking modder reads.
beforeAll(async () => {
  await i18nReady;
  await i18n.changeLanguage("de");
});
afterAll(async () => {
  await i18n.changeLanguage("en");
});

describe("PlayerPreviewCard in German", () => {
  // Given a player with a stated overall rating and no attribute block, so the rating is an estimate
  // When the preview card renders in German
  // Then the rating label is German ("GES", as everywhere else in the game), not the English "OVR",
  //      and the estimate marker is the translated one, not the English "est."
  it("labels the overall rating and its estimate in German", () => {
    render(
      <PlayerPreviewCard
        editing={{ ...emptyPlayer(), id: "p1", name: "Ada", overall: 70, attributes: null }}
        photoDataUrl={null}
      />,
    );

    expect(screen.getByText("GES")).toBeInTheDocument();
    expect(screen.queryByText("OVR")).not.toBeInTheDocument();
    expect(screen.getByText("gesch.")).toBeInTheDocument();
    expect(screen.queryByText("est.")).not.toBeInTheDocument();
  });
});
