/**
 * Which GitHub issue-form file backs each of the game's locales.
 *
 * The app opens a prefilled bug report by URL, so two things have to stay true together: the file
 * has to exist, and its field ids have to match the ones the app fills in. Neither is visible from
 * the TypeScript side — a renamed template, or a translator renaming `os` to `sistema-operativo`,
 * would produce a form with empty boxes and no error anywhere. `issueTemplates.test.ts` asserts
 * both against the real files in `.github/ISSUE_TEMPLATE/`.
 *
 * The suffixes are GitHub's filenames, not locale codes: `pt-BR` is `ptbr` and `zh-CN` is `zh_cn`,
 * because that is what the files were called when they were written.
 */
export const BUG_REPORT_TEMPLATE_BY_LOCALE: Record<string, string> = {
  en: "bug_report.yml",
  es: "bug_report_es.yml",
  pt: "bug_report_pt.yml",
  fr: "bug_report_fr.yml",
  de: "bug_report_de.yml",
  it: "bug_report_it.yml",
  ru: "bug_report_ru.yml",
  "pt-BR": "bug_report_ptbr.yml",
  "zh-CN": "bug_report_zh_cn.yml",
  cs: "bug_report_cs.yml",
  tr: "bug_report_tr.yml",
  id: "bug_report_id.yml",
};

/** The English form, used for any locale without one of its own. */
export const DEFAULT_BUG_REPORT_TEMPLATE = "bug_report.yml";

/**
 * Field ids the app prefills. Every localized template must expose all of them under the same
 * ids; `version` is what lets a triager tell which build broke without having to ask.
 *
 * `frequency` is deliberately absent. It is a dropdown whose *options* are translated, so
 * prefilling it by value would work in English and silently fail in every other language.
 */
export const PREFILLED_FIELD_IDS = [
  "what-happened",
  "expected-behavior",
  "steps-to-reproduce",
  "game-context",
  "version",
  "os",
  "resolution",
] as const;

export function bugReportTemplateFor(locale: string): string {
  return BUG_REPORT_TEMPLATE_BY_LOCALE[locale] ?? DEFAULT_BUG_REPORT_TEMPLATE;
}
