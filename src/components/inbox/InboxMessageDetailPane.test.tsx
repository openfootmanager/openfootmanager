import { render, screen, waitFor, within } from "@testing-library/react";
import { createInstance } from "i18next";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import en from "../../i18n/locales/en.json";
import type { MessageData } from "../../store/gameStore";
import type { PotentialOpinion } from "../../store/generated/PotentialOpinion";
import { createPlayer } from "../../test-utils/factories";
import InboxMessageDetailPane from "./InboxMessageDetailPane";

const translations = createInstance();
await translations.init({
  lng: "en",
  resources: { en: { translation: en } },
  interpolation: { escapeValue: false },
});
vi.mock("react-i18next", async (importOriginal) => ({
  ...(await importOriginal<typeof import("react-i18next")>()),
  useTranslation: () => ({ t: translations.t.bind(translations), i18n: translations }),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

/** A youth report from Ann Scout naming two prospects. */
const youthReport: MessageData = {
  id: "youth-scout-1",
  subject: "Youth prospects found",
  body: "",
  sender: "Ann Scout",
  sender_role: "Scout",
  date: "2026-09-01",
  read: true,
  category: "ScoutReport",
  priority: "Normal",
  actions: [],
  context: {
    team_id: "team-1",
    player_id: null,
    fixture_id: null,
    youth_target_position: null,
    match_result: null,
    youth_prospects: [
      createPlayer({ id: "judged", full_name: "Leo Builder", team_id: null }),
      createPlayer({ id: "unjudged", full_name: "Rui Prospect", team_id: null }),
    ],
  },
};

const annsOpinion: PotentialOpinion = {
  low: 76,
  high: 84,
  band: "A",
  certainty: "fairly_sure",
  source: { kind: "scout", scout_name: "Ann Scout", scouted_on: "2026-09-01", current: true },
  wonderkid: false,
};

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

describe("the youth report's opinions of its prospects", () => {
  /**
   * Given a youth report whose scout judged one of its two prospects 76-84, as the backend holds
   * When the report is open
   * Then the judged prospect's card shows her opinion, asked of the backend for this report, and
   *      the other prospect's shows none
   */
  it("shows the opinion the backend gives of each prospect", async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) =>
      command === "get_youth_prospect_opinions" ? { judged: annsOpinion } : null,
    );

    render(
      <InboxMessageDetailPane
        effectFeedback={null}
        currentTeamId="team-1"
        currentTeamName="Test FC"
        language="en"
        knowledgeRevision="r1"
        selectedMessage={youthReport}
        onAction={vi.fn()}
        onCloseSelectedMessage={vi.fn()}
        onRequestDelete={vi.fn()}
        onScoutPlayerClick={vi.fn()}
      />,
    );

    const opinion = await screen.findByRole("region", { name: "Scout's opinion" });
    expect(within(opinion).getByText("Potential 76–84")).toBeInTheDocument();
    expect(screen.getAllByRole("region", { name: "Scout's opinion" })).toHaveLength(1);
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("get_youth_prospect_opinions", {
        messageId: "youth-scout-1",
      }),
    );
  });
});
