import { takeMissingKeys } from "../i18n/missingKeys";
import { beforeAll, describe, expect, it } from "vitest";

import { i18nReady } from "../i18n";
import { getErrorMessage, resolveTranslatedErrorMessage } from "./errorMessage";

beforeAll(async () => {
  await i18nReady;
});

describe("resolveTranslatedErrorMessage", () => {
  /**
   * Given a backend error whose param is itself a key (a position group)
   * When the banner's message is resolved
   * Then the param is translated as well: no raw `common.positionGroups.*` reaches the player.
   */
  it("translates a param that is itself a key", () => {
    const message = resolveTranslatedErrorMessage(
      "be.error.squadFloor.wouldLeaveShort?group=common.positionGroups.Goalkeeper",
    );

    expect(message).toContain("Position group affected: Goalkeepers.");
    expect(message).not.toContain("common.");
  });

  /**
   * Given a backend error carrying a whole-number amount
   * When the banner's message is resolved
   * Then the amount is shown as money with its currency, not as a bare number.
   */
  it("formats a money param with the currency", () => {
    const message = resolveTranslatedErrorMessage(
      new Error("be.error.contracts.boardWagePolicy?budget=75000"),
    );

    expect(message).toMatch(/[^\d\s,.]\s?75,000|75,000\s?[^\d\s,.]/);
  });

  /**
   * Given a message that is not a translation key (a plain sentence, an Error, an unknown key)
   * When the banner's message is resolved
   * Then it is shown as it was, so nothing a player could read is lost.
   */
  it("leaves a message that is not a known key as it was", () => {
    // An error of a shape nobody sends is shown as its text, never blank.
    expect(resolveTranslatedErrorMessage(null)).toBe("null");
    expect(resolveTranslatedErrorMessage(42)).toBe("42");

    expect(resolveTranslatedErrorMessage("Something went wrong")).toBe("Something went wrong");
    expect(resolveTranslatedErrorMessage(new Error("be.error.noSuchKey"))).toBe(
      "be.error.noSuchKey",
    );
    expect(resolveTranslatedErrorMessage({ message: "plain" })).toBe("plain");
    // `be.error.noSuchKey` is unknown on purpose.
    expect(takeMissingKeys()).toEqual(["be.error.noSuchKey"]);
  });
});

describe("getErrorMessage", () => {
  /** Given a string, an Error and an object with a message, then each gives its text. */
  it("reads the text of an error however it arrives", () => {
    expect(getErrorMessage("a")).toBe("a");
    expect(getErrorMessage(new Error("b"))).toBe("b");
    expect(getErrorMessage({ message: "c" })).toBe("c");
  });
});
