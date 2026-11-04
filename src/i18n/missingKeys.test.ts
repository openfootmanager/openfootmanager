import i18n, { i18nReady } from "./index";
import { takeMissingKeys } from "./missingKeys";
import { beforeAll, describe, expect, it } from "vitest";

beforeAll(async () => {
  await i18nReady;
});

describe("missing translation keys", () => {
  /**
   * Given a key no locale has
   * When the app asks i18next for it
   * Then the key is reported (so the test that rendered it fails), and a key that exists is not.
   */
  it("reports a key no locale has and stays quiet for one that exists", () => {
    takeMissingKeys();

    i18n.t("common.close");
    i18n.t("be.msg.noSuchMessage.subject");

    expect(takeMissingKeys()).toEqual(["be.msg.noSuchMessage.subject"]);
  });

  /**
   * Given a key no locale has, asked with a default value
   * When the app asks i18next for it
   * Then it is not reported: the caller chose what the player reads, so no raw key shows.
   */
  it("stays quiet for a key asked with a default value", () => {
    takeMissingKeys();

    expect(i18n.t("be.msg.noSuchMessage.body", { defaultValue: "Fallback" })).toBe("Fallback");

    expect(takeMissingKeys()).toEqual([]);
  });
});
