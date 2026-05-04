import { describe, expect, it } from "vitest";

import de from "../i18n/locales/de.json";
import en from "../i18n/locales/en.json";
import es from "../i18n/locales/es.json";
import fr from "../i18n/locales/fr.json";
import itLocale from "../i18n/locales/it.json";
import ptBR from "../i18n/locales/pt-BR.json";
import pt from "../i18n/locales/pt.json";
import zhCN from "../i18n/locales/zh-CN.json";

type LocaleTree = Record<string, unknown>;

const LOCALES: Record<string, LocaleTree> = {
  de,
  en,
  es,
  fr,
  it: itLocale,
  pt,
  "pt-BR": ptBR,
  "zh-CN": zhCN,
};

const REQUIRED_KEYS = [
  "be.sender.assistantManager",
  "be.role.assistantManager",
  "be.msg.delegatedRenewals.subject",
  "be.msg.delegatedRenewals.body",
  "be.msg.delegatedRenewals.case.successful",
  "be.msg.delegatedRenewals.case.stalled",
  "be.msg.delegatedRenewals.case.failed",
  "be.msg.delegatedRenewals.notes.beyondLimits",
  "be.msg.delegatedRenewals.notes.prefersManager",
  "be.msg.delegatedRenewals.notes.managerBlocked",
  "be.msg.delegatedRenewals.notes.relationshipBlocked",
  "be.msg.playerEvent.respond",
  "be.msg.playerEvent.options.happyPlayer.praiseBack.label",
  "be.msg.playerEvent.options.happyPlayer.praiseBack.description",
  "be.news.weeklyDigest.headline",
  "be.msg.boardWarning.subject",
  "be.msg.boardWarning.body",
  "be.msg.boardFinalWarning.subject",
  "be.msg.boardFinalWarning.body",
  "be.msg.boardFired.subject",
  "be.msg.boardFired.body",
  "be.msg.jobOffer.subject",
  "be.msg.jobOffer.body",
  "be.msg.jobOffer.accept",
  "be.msg.jobOffer.decline",
  "be.msg.jobOffer.effects.accepted",
  "be.msg.jobOffer.effects.declined",
  "be.msg.jobOffer.effects.alreadyEmployed",
  "be.msg.jobOffer.effects.unavailable",
  "be.msg.jobOffer.effects.failed",
  "be.msg.jobOfferExpired.subject",
  "be.msg.jobOfferExpired.body",
  "be.msg.jobHired.subject",
  "be.msg.jobHired.body",
  "be.msg.jobRejection.subject",
  "be.msg.jobRejection.body",
  "be.news.managerialChange.headline",
  "be.news.managerialChange.body",
  "be.news.managerialAppointment.headline",
  "be.news.managerialAppointment.body",
  "be.news.seasonAwards.headline",
  "be.news.seasonAwards.bodyBoth",
  "be.news.seasonAwards.bodyGoldenBootOnly",
  "be.news.seasonAwards.bodyPotyOnly",
  "be.news.majorTransfer.headline",
  "be.news.majorTransfer.body",
  "boardObjectives.objective.LeaguePosition",
  "boardObjectives.objective.Wins",
  "boardObjectives.objective.GoalsScored",
] as const;

function getNestedValue(tree: LocaleTree, keyPath: string): unknown {
  return keyPath
    .split(".")
    .reduce<unknown>((value, segment) => {
      if (value === null || typeof value !== "object") {
        return undefined;
      }

      return (value as Record<string, unknown>)[segment];
    }, tree);
}

function collectMissingKeys(
  reference: LocaleTree,
  candidate: LocaleTree,
  path: string[] = [],
): string[] {
  return Object.entries(reference).flatMap(([key, value]) => {
    const nextPath = [...path, key];
    const candidateValue = candidate[key];

    if (value !== null && typeof value === "object" && !Array.isArray(value)) {
      if (
        candidateValue === null ||
        typeof candidateValue !== "object" ||
        Array.isArray(candidateValue)
      ) {
        return [nextPath.join(".")];
      }

      return collectMissingKeys(
        value as LocaleTree,
        candidateValue as LocaleTree,
        nextPath,
      );
    }

    return candidateValue === undefined ? [nextPath.join(".")] : [];
  });
}

describe("backend i18n locale coverage", () => {
  it("keeps required backend-facing translation keys in every supported locale", () => {
    const missingKeysByLocale = Object.entries(LOCALES).reduce<
      Record<string, string[]>
    >((accumulator, [localeCode, translations]) => {
      const missingKeys = REQUIRED_KEYS.filter((keyPath) => {
        return getNestedValue(translations, keyPath) === undefined;
      });

      if (missingKeys.length > 0) {
        accumulator[localeCode] = missingKeys;
      }

      return accumulator;
    }, {});

    expect(missingKeysByLocale).toEqual({});
  });

  it("keeps zh-CN aligned with the English translation key set", () => {
    expect(collectMissingKeys(en, zhCN)).toEqual([]);
  });
});
