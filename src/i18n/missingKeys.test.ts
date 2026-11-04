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
   * Given a key no locale has, asked with a fallback text
   * When the app asks i18next for it
   * Then it is still reported: the fallback hides the bug on screen, it does not fix it.
   */
  it("reports a missing key even when a default value hides it", () => {
    takeMissingKeys();

    expect(i18n.t("be.msg.noSuchMessage.body", { defaultValue: "Fallback" })).toBe("Fallback");

    expect(takeMissingKeys()).toEqual(["be.msg.noSuchMessage.body"]);
  });
});
