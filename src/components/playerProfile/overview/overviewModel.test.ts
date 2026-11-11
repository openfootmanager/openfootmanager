import { describe, expect, it } from "vitest";

import cs from "../../../i18n/locales/cs.json";
import de from "../../../i18n/locales/de.json";
import en from "../../../i18n/locales/en.json";
import es from "../../../i18n/locales/es.json";
import fr from "../../../i18n/locales/fr.json";
import id from "../../../i18n/locales/id.json";
import itLocale from "../../../i18n/locales/it.json";
import ptBR from "../../../i18n/locales/pt-BR.json";
import pt from "../../../i18n/locales/pt.json";
import ru from "../../../i18n/locales/ru.json";
import tr from "../../../i18n/locales/tr.json";
import zhCN from "../../../i18n/locales/zh-CN.json";
import { GOALKEEPER_AXES, OUTFIELD_AXES } from "./overviewModel";

const LOCALES = { en, es, pt, "pt-BR": ptBR, fr, de, it: itLocale, ru, "zh-CN": zhCN, cs, tr, id };

describe("the radar's axis labels", () => {
  /**
   * Given each of the twelve locales and both axis sets (outfield and goalkeeper)
   * When the written abbreviations of their axes are read
   * Then every axis has one, the eight of a set are all different, and each is at most five
   *      characters, so a label fits beside the radar in any language the game ships in
   */
  it.each(Object.entries(LOCALES))(
    "gives %s eight different short labels per axis set",
    (_code, locale) => {
      const written = locale.playerProfile.overview.axes as Record<string, string>;
      for (const axes of [OUTFIELD_AXES, GOALKEEPER_AXES]) {
        const labels = axes.map((axis) => written[axis]);
        expect(labels.every((label) => typeof label === "string" && label.length > 0)).toBe(true);
        expect(new Set(labels).size).toBe(axes.length);
        for (const label of labels) expect([...label].length).toBeLessThanOrEqual(5);
      }
    },
  );
});
