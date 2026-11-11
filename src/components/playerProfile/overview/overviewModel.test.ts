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
import { GOALKEEPER_AXES, OUTFIELD_AXES, shortLabels } from "./overviewModel";

const LOCALES = { en, es, pt, "pt-BR": ptBR, fr, de, it: itLocale, ru, "zh-CN": zhCN, cs, tr, id };

describe("shortLabels", () => {
  /**
   * Given the eight attribute names of a Latin-script language
   * When they are shortened for the radar
   * Then each is its first three letters, upper-cased
   */
  it("takes the first three letters of each name", () => {
    expect(shortLabels(["Pace", "Shooting", "Aerial"])).toEqual(["PAC", "SHO", "AER"]);
  });

  /**
   * Given two names that begin alike (the Russian "Игра на выходах" and "Игра головой")
   * When they are shortened
   * Then each is lengthened until it differs from the other, so no two axes carry the same label
   */
  it("lengthens names that would collide", () => {
    const labels = shortLabels(["Игра на выходах", "Игра головой", "Реакция"]);

    expect(new Set(labels).size).toBe(3);
    expect(labels[2]).toBe("РЕА");
  });

  /**
   * Given Chinese names, which carry a word in two characters
   * When they are shortened
   * Then each keeps its first two characters
   */
  it("keeps two characters of a Chinese name", () => {
    expect(shortLabels(["速度", "射门", "制空权"])).toEqual(["速度", "射门", "制空"]);
  });

  /**
   * Given each of the twelve locales and both axis sets (outfield and goalkeeper)
   * When the axes are shortened from the locale's own attribute names
   * Then the eight labels of a set are all different, and each is at most five characters, so a
   *      label fits beside the radar in any language the game ships in
   */
  it.each(Object.entries(LOCALES))(
    "gives %s eight different short labels per axis set",
    (_code, locale) => {
      for (const axes of [OUTFIELD_AXES, GOALKEEPER_AXES]) {
        const names = axes.map((axis) => locale.common.attributes[axis]);
        const labels = shortLabels(names);
        expect(new Set(labels).size).toBe(axes.length);
        for (const label of labels) expect([...label].length).toBeLessThanOrEqual(5);
      }
    },
  );
});
