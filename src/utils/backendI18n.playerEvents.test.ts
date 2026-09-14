import { describe, expect, it } from "vitest";

import {
  inferPlayerEventActionLabelKey,
  inferPlayerEventGroup,
  inferPlayerEventOptionBaseKey,
} from "./backendI18nPlayerEvents.ts";

describe("backendI18n.playerEvents", () => {
  it("infers the player-event group from legacy message prefixes", () => {
    expect(inferPlayerEventGroup("happy_player_p_fwd0")).toBe("happyPlayer");
    expect(inferPlayerEventGroup("bench_complaint_mid_1")).toBe("benchComplaint");
  });

  it("returns undefined for unrelated messages", () => {
    expect(inferPlayerEventGroup("transfer_bid_123")).toBeUndefined();
    expect(inferPlayerEventActionLabelKey("transfer_bid_123", "respond")).toBeUndefined();
  });

  it("infers the generic respond action key for legacy player events", () => {
    expect(inferPlayerEventActionLabelKey("morale_talk_cb_2", "respond")).toBe(
      "be.msg.playerEvent.respond",
    );
    expect(inferPlayerEventActionLabelKey("morale_talk_cb_2", "acknowledge")).toBeUndefined();
  });

  it("infers option base keys for known legacy player-event options", () => {
    expect(inferPlayerEventOptionBaseKey("happy_player_p_fwd0", "praise_back")).toBe(
      "be.msg.playerEvent.options.happyPlayer.praiseBack",
    );
    expect(inferPlayerEventOptionBaseKey("contract_concern_1", "no_renewal")).toBe(
      "be.msg.playerEvent.options.contractConcern.noRenewal",
    );
  });

  it("returns undefined for unknown option ids", () => {
    expect(inferPlayerEventOptionBaseKey("happy_player_p_fwd0", "unknown_option")).toBeUndefined();
  });
});
