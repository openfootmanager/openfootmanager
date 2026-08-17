import { describe, expect, it } from "vitest";

import cs from "../../../i18n/locales/cs.json";
import de from "../../../i18n/locales/de.json";
import en from "../../../i18n/locales/en.json";
import es from "../../../i18n/locales/es.json";
import fr from "../../../i18n/locales/fr.json";
import itIT from "../../../i18n/locales/it.json";
import ptBR from "../../../i18n/locales/pt-BR.json";
import pt from "../../../i18n/locales/pt.json";
import ru from "../../../i18n/locales/ru.json";
import tr from "../../../i18n/locales/tr.json";
import zhCN from "../../../i18n/locales/zh-CN.json";

const LOCALE_BUNDLES = { cs, de, en, es, fr, it: itIT, pt, "pt-BR": ptBR, ru, tr, "zh-CN": zhCN };
import {
  PLAY_STYLES,
  buildParticipantSpec,
  normalizePlayStyle,
  emptyCompetition,
  emptyConfederation,
  emptyCountry,
  emptyMeta,
  emptyNamesDefinition,
  emptyPlayer,
  emptyTeam,
  entityRowKey,
  makeRange,
  mergeFallbackLeague,
  parsePoolText,
  parseRangeBound,
  toSlug,
} from "./helpers";
import type { SelectorSpec } from "./types";

describe("parseRangeBound", () => {
  it("returns null for empty string", () => {
    expect(parseRangeBound("")).toBeNull();
  });

  it("returns null for a non-numeric string", () => {
    expect(parseRangeBound("abc")).toBeNull();
  });

  it("returns null for the string 'NaN'", () => {
    expect(parseRangeBound("NaN")).toBeNull();
  });

  it("parses a valid integer string", () => {
    expect(parseRangeBound("42")).toBe(42);
  });

  it("parses a negative integer string", () => {
    expect(parseRangeBound("-10")).toBe(-10);
  });

  it("parses zero", () => {
    expect(parseRangeBound("0")).toBe(0);
  });

  it("parses a string with a leading zero (parseInt strips the zero)", () => {
    expect(parseRangeBound("007")).toBe(7);
  });
});

describe("makeRange", () => {
  it("returns null when the first bound is null", () => {
    expect(makeRange(null, 50)).toBeNull();
  });

  it("returns null when the second bound is null", () => {
    expect(makeRange(10, null)).toBeNull();
  });

  it("returns null when both bounds are null", () => {
    expect(makeRange(null, null)).toBeNull();
  });

  it("returns a tuple when both bounds are provided", () => {
    expect(makeRange(10, 50)).toEqual([10, 50]);
  });

  it("does not enforce min <= max (no validation is the contract)", () => {
    expect(makeRange(50, 10)).toEqual([50, 10]);
  });

  it("allows equal bounds", () => {
    expect(makeRange(30, 30)).toEqual([30, 30]);
  });

  it("allows zero as a valid bound", () => {
    expect(makeRange(0, 100)).toEqual([0, 100]);
  });
});

describe("emptyTeam", () => {
  it("defaults reputation and finance ranges to null", () => {
    const team = emptyTeam();
    expect(team.reputationRange).toBeNull();
    expect(team.financeRange).toBeNull();
  });

  it("defaults play style to Balanced", () => {
    expect(emptyTeam().playStyle).toBe("Balanced");
  });

  it("defaults logo to null", () => {
    expect(emptyTeam().logo).toBeNull();
  });

  it("returns a new object on each call (not a shared reference)", () => {
    expect(emptyTeam()).not.toBe(emptyTeam());
  });

  it("returns an object with new colors reference on each call", () => {
    expect(emptyTeam().colors).not.toBe(emptyTeam().colors);
  });
});

describe("emptyMeta", () => {
  it("defaults package type to database", () => {
    expect(emptyMeta().packageType).toBe("database");
  });

  it("defaults version to 1.0.0", () => {
    expect(emptyMeta().version).toBe("1.0.0");
  });

  it("defaults baseYear to null", () => {
    expect(emptyMeta().baseYear).toBeNull();
  });

  it("defaults formatVersion to 1", () => {
    expect(emptyMeta().formatVersion).toBe(1);
  });

  it("defaults defaultActiveRegions to an empty array", () => {
    expect(emptyMeta().defaultActiveRegions).toEqual([]);
  });

  it("returns a new object on each call (not a shared reference)", () => {
    expect(emptyMeta()).not.toBe(emptyMeta());
  });
});

describe("parsePoolText", () => {
  it("returns empty array for empty string", () => {
    expect(parsePoolText("")).toEqual([]);
  });

  it("splits on newlines", () => {
    expect(parsePoolText("Alice\nBob")).toEqual(["Alice", "Bob"]);
  });

  it("trims whitespace from each entry", () => {
    expect(parsePoolText("  Alice  \n  Bob  ")).toEqual(["Alice", "Bob"]);
  });

  it("filters out blank lines", () => {
    expect(parsePoolText("Alice\n\nBob\n\n")).toEqual(["Alice", "Bob"]);
  });

  it("filters lines that are only whitespace", () => {
    expect(parsePoolText("Alice\n   \nBob")).toEqual(["Alice", "Bob"]);
  });

  it("returns single-element array for a single name", () => {
    expect(parsePoolText("Carlos")).toEqual(["Carlos"]);
  });

  it("returns empty array for whitespace-only input", () => {
    expect(parsePoolText("   \n   ")).toEqual([]);
  });
});

describe("buildParticipantSpec", () => {
  const noopSelector: SelectorSpec = { kind: "topByReputation", excludeCompetitions: [] };

  it("explicit mode returns the parsed team list under explicit key", () => {
    const result = buildParticipantSpec("explicit", "team-a\nteam-b", noopSelector);
    expect(result.explicit).toEqual(["team-a", "team-b"]);
  });

  it("explicit mode does NOT include a selector key", () => {
    const result = buildParticipantSpec("explicit", "team-a", noopSelector);
    expect(result.selector).toBeUndefined();
  });

  it("explicit mode with empty text yields empty array, not undefined", () => {
    const result = buildParticipantSpec("explicit", "", noopSelector);
    expect(result.explicit).toEqual([]);
  });

  it("explicit mode strips blank lines from the team list", () => {
    const result = buildParticipantSpec("explicit", "team-a\n\nteam-b\n", noopSelector);
    expect(result.explicit).toEqual(["team-a", "team-b"]);
  });

  it("selector mode returns the selector under selector key", () => {
    const sel: SelectorSpec = { kind: "allInCountry", country: "ENG", excludeCompetitions: [] };
    const result = buildParticipantSpec("selector", "ignored-text", sel);
    expect(result.selector).toEqual(sel);
  });

  it("selector mode does NOT include an explicit key", () => {
    const result = buildParticipantSpec("selector", "team-a", noopSelector);
    expect(result.explicit).toBeUndefined();
  });

  it("selector mode ignores the explicit text argument", () => {
    const sel: SelectorSpec = { kind: "allInRegion", region: "europe", excludeCompetitions: [] };
    const r1 = buildParticipantSpec("selector", "team-x", sel);
    const r2 = buildParticipantSpec("selector", "", sel);
    expect(r1).toEqual(r2);
  });
});

describe("emptyConfederation", () => {
  it("returns an object with empty id and name", () => {
    expect(emptyConfederation()).toEqual({ id: "", name: "" });
  });

  it("returns a new object on each call", () => {
    expect(emptyConfederation()).not.toBe(emptyConfederation());
  });
});

describe("emptyCountry", () => {
  it("returns an object with empty id, name, and confederation", () => {
    expect(emptyCountry()).toEqual({ id: "", name: "", confederation: "" });
  });

  it("returns a new object on each call", () => {
    expect(emptyCountry()).not.toBe(emptyCountry());
  });
});

describe("emptyPlayer", () => {
  it("defaults position to Goalkeeper", () => {
    expect(emptyPlayer().position).toBe("Goalkeeper");
  });

  it("defaults all optional fields to null", () => {
    const p = emptyPlayer();
    expect(p.dateOfBirth).toBeNull();
    expect(p.overall).toBeNull();
    expect(p.attributes).toBeNull();
    // Null, not absent: a new player hands their ceiling back to the engine's
    // age-based roll, which is what every package did before the field existed.
    expect(p.potential).toBeNull();
  });

  it("uses the backend-aligned `footedness` key (not `foot`)", () => {
    const p = emptyPlayer();
    // The Rust PlayerDef field is `footedness`; a `foot` key would be dropped
    // on save and the authored foot lost. Pin the contract here.
    expect("footedness" in p).toBe(true);
    expect((p as unknown as Record<string, unknown>).foot).toBeUndefined();
    expect(p.footedness).toBeNull();
  });

  it("returns a new object on each call", () => {
    expect(emptyPlayer()).not.toBe(emptyPlayer());
  });
});

describe("emptyNamesDefinition", () => {
  it("defaults version to 1", () => {
    expect(emptyNamesDefinition().version).toBe(1);
  });

  it("defaults pools to an empty object", () => {
    expect(emptyNamesDefinition().pools).toEqual({});
  });

  it("returns a new object on each call", () => {
    expect(emptyNamesDefinition()).not.toBe(emptyNamesDefinition());
  });
});

describe("emptyCompetition", () => {
  it("defaults type to League", () => {
    expect(emptyCompetition().type).toBe("League");
  });

  it("defaults scope to Domestic", () => {
    expect(emptyCompetition().scope).toBe("Domestic");
  });

  it("defaults format kind to LeagueTable", () => {
    expect(emptyCompetition().format.kind).toBe("LeagueTable");
  });

  it("defaults participants to explicit empty list", () => {
    expect(emptyCompetition().participants.explicit).toEqual([]);
    expect(emptyCompetition().participants.selector).toBeUndefined();
  });

  it("returns a new object on each call", () => {
    expect(emptyCompetition()).not.toBe(emptyCompetition());
  });
});

describe("mergeFallbackLeague", () => {
  it("creates a config from the first override set", () => {
    expect(mergeFallbackLeague(null, { name: "Premier Division" })).toEqual({
      name: "Premier Division",
    });
  });

  it("keeps previously set overrides when patching another field", () => {
    const withName = mergeFallbackLeague(null, { name: "Premier Division" });
    expect(mergeFallbackLeague(withName, { legs: 1 })).toEqual({
      name: "Premier Division",
      legs: 1,
    });
  });

  it("collapses to null once every override is cleared", () => {
    const config = mergeFallbackLeague({ name: "Premier Division" }, { legs: 2 });
    const nameCleared = mergeFallbackLeague(config, { name: null });
    expect(nameCleared).toEqual({ name: null, legs: 2 });
    expect(mergeFallbackLeague(nameCleared, { legs: null })).toBeNull();
  });

  it("treats an all-blank patch as no config rather than an empty object", () => {
    expect(mergeFallbackLeague(null, { name: "", legs: null, scope: null })).toBeNull();
  });

  it("keeps a scope-only config", () => {
    expect(mergeFallbackLeague(null, { scope: "Continental" })).toEqual({
      scope: "Continental",
    });
  });
});

describe("entityRowKey", () => {
  it("uses the id when there is one", () => {
    expect(entityRowKey("fc-test", 3)).toBe("fc-test");
  });

  it("keeps the key stable when the row moves", () => {
    // The point of keying by id: an insert above shifts every index, and the
    // row's delete-confirmation state must stay with the row that owns it.
    expect(entityRowKey("fc-test", 0)).toBe(entityRowKey("fc-test", 7));
  });

  it("gives entities without an id distinct keys", () => {
    // A package can arrive with several ids blank, and repairing one is exactly
    // what the editor is for; identical keys would let one row's state show on
    // another.
    expect(entityRowKey("", 0)).not.toBe(entityRowKey("", 1));
  });

  it("cannot collide with a generated id", () => {
    // toSlug strips everything outside [a-z0-9-], so no auto-generated id can
    // begin with the fallback's "#".
    expect(entityRowKey("", 0).startsWith("#")).toBe(true);
    expect(toSlug("#row-0")).not.toContain("#");
  });
});

describe("PLAY_STYLES", () => {
  it("offers exactly the styles the engine accepts", () => {
    // `play_style_from_str` in the generator matches these names and falls back
    // to Balanced for anything else, silently. The editor previously offered
    // "Pressing", which is not a variant, so picking it produced a Balanced
    // club with no warning — and Possession and HighPress were unreachable.
    expect([...PLAY_STYLES].sort()).toEqual(
      ["Attacking", "Balanced", "Counter", "Defensive", "HighPress", "Possession"],
    );
  });

  it("has a translation in every supported locale for every style it offers", () => {
    // LabeledSelect renders raw option values unless given labels, so every
    // style must resolve through playStyles.* — and in all 11 locales, not
    // just English, or a missing entry ships as a raw enum name.
    for (const [code, bundle] of Object.entries(LOCALE_BUNDLES)) {
      for (const style of PLAY_STYLES) {
        const label = bundle.common.playStyles[style as keyof typeof bundle.common.playStyles];
        expect(label, `${code} is missing common.playStyles.${style}`).toBeTruthy();
      }
    }
  });

  it("normalizes a legacy play style that is no longer offered", () => {
    // Packages authored before "Pressing" was removed still carry it. Binding
    // an unlisted value to the select leaves it blank and saves the invalid
    // value straight back, so it has to be coerced on the way in.
    expect(normalizePlayStyle("Pressing")).toBe("Balanced");
    expect(normalizePlayStyle("")).toBe("Balanced");
    expect(normalizePlayStyle("nonsense")).toBe("Balanced");
  });

  it("leaves a supported play style untouched", () => {
    for (const style of PLAY_STYLES) {
      expect(normalizePlayStyle(style)).toBe(style);
    }
  });
});
