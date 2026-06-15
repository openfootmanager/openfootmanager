import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { TraitBadge } from "./TraitBadge";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const translations: Record<string, string> = {
        "traits.Speedster.label": "Velocista",
        "traits.Speedster.desc": "Explosivo",
        "traits.HotHead.label": "Temperamental",
        "traits.HotHead.desc": "Juega al límite",
        "traits.Wonderkid.label": "Joya",
        "traits.Wonderkid.desc": "Talento especial",
        "common.attributes.pace": "Ritmo",
        "common.attributes.aggression": "Agresividad",
        "common.attributes.composure": "Compostura",
        "youthAcademy.age": "Edad",
        "youthAcademy.potential": "Potencial",
        "youthAcademy.growth": "Crecimiento",
      };

      return translations[key] ?? key;
    },
  }),
}));

describe("TraitBadge", () => {
  it("uses translated labels for minimum-threshold requirements in the tooltip", () => {
    render(<TraitBadge trait="Speedster" />);

    expect(screen.getByLabelText("Explosivo | Ritmo 85+")).toBeInTheDocument();
  });

  it("uses numeric operators instead of embedded English phrases", () => {
    render(<TraitBadge trait="HotHead" />);

    expect(
      screen.getByLabelText(
        "Juega al límite | Agresividad 85+, Compostura < 50",
      ),
    ).toBeInTheDocument();
  });

  it("uses translated non-attribute labels for wonderkid requirements", () => {
    render(<TraitBadge trait="Wonderkid" />);

    expect(
      screen.getByLabelText(
        "Talento especial | Edad <= 20, Potencial 90+, Crecimiento 14+",
      ),
    ).toBeInTheDocument();
  });
});
