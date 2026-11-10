import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import CountryLeagueRail from "./CountryLeagueRail";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "en" } }),
}));

const LEAGUES = [
  { id: "top", name: "First Division", clubCount: 13 },
  { id: "low", name: "Second Division", clubCount: 16 },
];

function renderRail(overrides: Partial<Parameters<typeof CountryLeagueRail>[0]> = {}) {
  const handlers = { onSelectCountry: vi.fn(), onSelectLeague: vi.fn() };
  render(
    <CountryLeagueRail
      countryCodes={["SE", "NO"]}
      selectedCountryCode="SE"
      leagues={LEAGUES}
      activeLeagueId="top"
      {...handlers}
      {...overrides}
    />,
  );
  return handlers;
}

describe("CountryLeagueRail", () => {
  /**
   * Given two countries with Sweden picked, and its two leagues
   * When the rail is drawn
   * Then Sweden is open with its leagues and their club counts, the active league is marked, and
   *      Norway is closed with none.
   */
  it("opens the picked country to its leagues, counted, with the active one marked", () => {
    renderRail();

    const nav = screen.getByRole("navigation", { name: "teamSelect.rail.label" });
    expect(within(nav).getByRole("button", { name: /Sweden/ })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
    expect(within(nav).getByRole("button", { name: /Norway/ })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
    expect(within(nav).getByRole("button", { name: /First Division/ })).toHaveAttribute(
      "aria-current",
      "true",
    );
    expect(within(nav).getByRole("button", { name: /Second Division/ })).toHaveTextContent("16");
    expect(within(nav).getByRole("button", { name: /Second Division/ })).not.toHaveAttribute(
      "aria-current",
    );
  });

  it("asks for a country or a league when one is pressed", () => {
    const handlers = renderRail();

    fireEvent.click(screen.getByRole("button", { name: /Norway/ }));
    fireEvent.click(screen.getByRole("button", { name: /Second Division/ }));

    expect(handlers.onSelectCountry).toHaveBeenCalledWith("NO");
    expect(handlers.onSelectLeague).toHaveBeenCalledWith("low");
  });

  /**
   * Given a world whose competitions place no country
   * When the rail is drawn
   * Then the leagues stand alone and can still be picked.
   */
  it("lists the leagues alone when there is no country to pick", () => {
    const handlers = renderRail({ countryCodes: [], selectedCountryCode: null });

    fireEvent.click(screen.getByRole("button", { name: /First Division/ }));

    expect(handlers.onSelectLeague).toHaveBeenCalledWith("top");
    expect(screen.queryByRole("button", { name: /Sweden/ })).toBeNull();
  });
});
