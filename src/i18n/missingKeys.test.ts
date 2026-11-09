import i18n, { i18nReady } from "./index";
import { resolveBackendError } from "../utils/backendI18n";
import { looksLikeTranslationKey, takeMissingKeys } from "./missingKeys";
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

  /**
   * Given a key no locale has, asked with the key itself as its default value
   * When the app asks i18next for it
   * Then it is reported: the player would read the raw key.
   */
  it("reports a key whose default value is the key itself", () => {
    takeMissingKeys();

    i18n.t("be.msg.noSuchMessage.title", { defaultValue: "be.msg.noSuchMessage.title" });

    expect(takeMissingKeys()).toEqual(["be.msg.noSuchMessage.title"]);
  });

  /**
   * Given a key no locale has, asked with another key as its default value (a caller that "falls
   *       back" to a backend key)
   * When the app asks i18next for it
   * Then it is reported: the player would read a raw key either way.
   */
  it("reports a key whose default value is itself a key", () => {
    takeMissingKeys();

    i18n.t("be.msg.noSuchMessage.title", { defaultValue: "be.msg.noSuchMessage.fallback" });

    expect(takeMissingKeys()).toEqual(["be.msg.noSuchMessage.title"]);
  });

  /**
   * Given a backend error whose param is a key no locale has (the digest-leak shape)
   * When the error is resolved
   * Then that key is reported, though the resolver asks `exists` before translating a param.
   */
  it("reports a key-shaped param that has no text", () => {
    takeMissingKeys();

    resolveBackendError("be.error.squadFloor.wouldLeaveShort?group=common.positionGroups.Nope");

    expect(takeMissingKeys()).toEqual(["common.positionGroups.Nope"]);
  });

  /**
   * Given a backend error that is a key no locale has
   * When the error is resolved
   * Then the key is reported, though the resolver asks `exists` first.
   */
  it("reports a backend error that is a key without text", () => {
    takeMissingKeys();

    resolveBackendError("be.error.noSuchError");

    expect(takeMissingKeys()).toEqual(["be.error.noSuchError"]);
  });

  /**
   * Given plain values that contain a dot (a money figure, a name with an initial, a number), and
   *       one that is under a namespace the locale has
   * When they are checked for the shape of a key
   * Then only the dotted, space-free one under a real namespace is key-shaped.
   */
  it.each([
    ["€1.8M", false],
    ["Mr. Smith", false],
    ["1.25", false],
    ["St.Mary", false],
    ["unknownNamespace.thing", false],
    ["common.positionGroups.Nope", true],
    ["be.msg.noSuchMessage.subject", true],
  ])("tells whether %s is shaped like a key: %s", (value, shaped) => {
    expect(looksLikeTranslationKey(value)).toBe(shaped);
  });
});
