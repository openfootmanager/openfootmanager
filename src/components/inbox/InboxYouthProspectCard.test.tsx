import { render, screen, within } from "@testing-library/react";
import { createInstance } from "i18next";
import { describe, expect, it, vi } from "vitest";

import en from "../../i18n/locales/en.json";
import type { PotentialOpinion } from "../../store/generated/PotentialOpinion";
import { createPlayer } from "../../test-utils/factories";
import InboxYouthProspectCard from "./InboxYouthProspectCard";

const translations = createInstance();
await translations.init({
  lng: "en",
  resources: { en: { translation: en } },
  interpolation: { escapeValue: false },
});
const t = translations.t.bind(translations);
// The trait badges translate through react-i18next themselves: route them to the same English.
vi.mock("react-i18next", async (importOriginal) => ({
  ...(await importOriginal<typeof import("react-i18next")>()),
  useTranslation: () => ({ t, i18n: translations }),
}));

// Rated 63, truly potential 91, on 2K a week and worth 350K: the true numbers no report shows,
// and the terms the club would sign him on.
const prospect = createPlayer({
  id: "prospect-1",
  full_name: "Leo Builder",
  team_id: null,
  ovr: 63,
  potential: 91,
  wage: 2_000,
  market_value: 350_000,
  traits: [],
});

const scoutsOpinion = (wonderkid: boolean): PotentialOpinion => ({
  low: 76,
  high: 84,
  band: "A",
  certainty: "fairly_sure",
  source: { kind: "scout", scout_name: "Ann Scout", scouted_on: "2026-09-01", current: true },
  wonderkid,
});

const renderCard = (opinion: PotentialOpinion | null, traits: string[] = []) =>
  render(
    <InboxYouthProspectCard
      prospect={{ ...prospect, traits }}
      opinion={opinion}
      action={undefined}
      messageId="youth-scout-1"
      currentTeamId="team-1"
      language="en"
      t={t}
      onAction={vi.fn()}
      onScoutPlayerClick={vi.fn()}
    />,
  );

describe("InboxYouthProspectCard", () => {
  /**
   * Given a prospect whose scout, Ann Scout, judged his potential 76-84
   * When his card shows
   * Then it gives her opinion: the range, how sure she is, and her name
   */
  it("shows the scout's opinion of the prospect", () => {
    renderCard(scoutsOpinion(false));

    const opinion = within(screen.getByRole("region", { name: "Scout's opinion" }));
    expect(opinion.getByText("Potential 76–84")).toBeInTheDocument();
    expect(opinion.getByText("Fairly sure")).toBeInTheDocument();
    expect(opinion.getByText(/Scouted by Ann Scout/)).toBeInTheDocument();
  });

  /**
   * Given a prospect rated 63 with a true potential of 91
   * When his card shows
   * Then neither number shows, nor an OVR label: the club has only the scout's opinion
   */
  it("never shows the prospect's rating or true potential", () => {
    renderCard(scoutsOpinion(false));

    expect(screen.queryByText(/\b63\b/)).not.toBeInTheDocument();
    expect(screen.queryByText(/\b91\b/)).not.toBeInTheDocument();
    expect(screen.queryByText(/OVR/)).not.toBeInTheDocument();
  });

  /**
   * Given a prospect on 2K a week, worth 350K
   * When his card shows
   * Then the club's offer terms show: his wage and market value
   */
  it("keeps the terms the club would sign him on", () => {
    renderCard(scoutsOpinion(false));

    expect(screen.getByText(/Wage\/wk/)).toBeInTheDocument();
    expect(screen.getByText(/Market Value/)).toBeInTheDocument();
  });

  /**
   * Given a prospect in a report written before reports carried an opinion
   * When his card shows
   * Then no opinion shows, and the card still names him
   */
  it("shows no opinion for a prospect the report gave none of", () => {
    renderCard(null);

    expect(screen.getByText("Leo Builder")).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Scout's opinion" })).not.toBeInTheDocument();
    expect(screen.queryByText(/^Potential /)).not.toBeInTheDocument();
  });

  /**
   * Given a prospect whose own traits hold no Wonderkid tag, and a scout's opinion that calls him
   *       one
   * When his card shows
   * Then the Wonderkid tag shows: it is the club's verdict
   */
  it("tags the prospect a wonderkid when the scout's opinion calls him one", () => {
    renderCard(scoutsOpinion(true));

    expect(screen.getByText("Wonderkid")).toBeInTheDocument();
  });

  /**
   * Given a prospect who carries the Wonderkid tag, and a scout's opinion that does not call him
   *       one
   * When his card shows
   * Then the tag does not show: his own tells his true potential
   */
  it("hides the prospect's own Wonderkid tag when the scout's opinion does not call him one", () => {
    renderCard(scoutsOpinion(false), ["Wonderkid"]);

    expect(screen.queryByText("Wonderkid")).not.toBeInTheDocument();
  });
});
