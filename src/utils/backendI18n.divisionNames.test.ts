import { afterAll, beforeAll, describe, expect, it } from "vitest";

import i18n, { i18nReady } from "../i18n";
import type { MessageData } from "../store/gameStore";
import { resolveMessage } from "./backendI18n";

const makeMessage = (overrides: Partial<MessageData> = {}): MessageData => ({
  id: "division_change",
  subject: "raw subject",
  body: "raw body",
  sender: "raw sender",
  sender_role: "raw role",
  date: "2032-01-01",
  read: false,
  category: "general",
  priority: "normal",
  actions: [],
  context: { team_id: null, player_id: null, fixture_id: null, match_result: null },
  ...overrides,
});

let previousLanguage = "en";

beforeAll(async () => {
  await i18nReady;
  previousLanguage = i18n.language;
  await i18n.changeLanguage("de");
});

afterAll(async () => {
  await i18n.changeLanguage(previousLanguage);
});

describe("division-change message names", () => {
  /**
   * Given a promotion into a third-tier split-season league
   * When the notice is resolved
   * Then the backend tier and phase metadata produce the localized name
   */
  it("localizes the tier and opening phase in a promotion notice", () => {
    const promotion = resolveMessage(
      makeMessage({
        subject_key: "be.msg.promotion.subject",
        body_key: "be.msg.promotion.body",
        i18n_params: {
          division: "Germany Division 3",
          divisionKey: "tournaments.competitions.tierDivision",
          divisionCountry: "DE",
          divisionSeason: "2032",
          divisionTier: "3",
          divisionPhase: "opening",
          season: "2032",
        },
      }),
    );

    expect(promotion.subject).toContain("Deutschland 3. Liga");
    expect(promotion.subject).toContain("Eröffnungsturnier");
    expect(promotion.body).toContain("Eröffnungsturnier");
  });

  /**
   * Given a legacy notice with no tier metadata
   * When its generic tier key cannot be rendered safely
   * Then the original stored division name remains the fallback
   */
  it("keeps the stored name when a generic tier key has no tier", () => {
    const promotion = resolveMessage(
      makeMessage({
        subject_key: "be.msg.promotion.subject",
        body_key: "be.msg.promotion.body",
        i18n_params: {
          division: "Germany Division 3",
          divisionKey: "tournaments.competitions.tierDivision",
          divisionCountry: "DE",
          divisionPhase: "opening",
          season: "2032",
        },
      }),
    );

    expect(promotion.subject).toContain("Germany Division 3");
    expect(promotion.subject).not.toContain("Eröffnungsturnier");
  });
});
