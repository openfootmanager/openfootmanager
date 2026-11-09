import french from "../../i18n/locales/fr.json";
import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import HomeLeaguePositionCard from "./HomeLeaguePositionCard";

const locale = vi.hoisted(() => ({ language: "en" }));

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => undefined },
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (locale.language === "fr" && key.startsWith("common.ordinal.place.")) {
        const forms: Record<string, string> = french.common.ordinal.place;
        return (forms[key.slice(key.lastIndexOf(".") + 1)] ?? key).replace(
          "{{n}}",
          String(params?.n),
        );
      }
      if (locale.language === "ru" && key.startsWith("common.ordinal.")) {
        return `${params?.n}-${key.includes(".place.") ? "е" : "й"}`;
      }
      key = key.replace("common.ordinal.place.", "common.ordinal.");
      if (key === "home.standings") return "Standings";
      if (key === "home.leaguePosition") return "League Position";
      if (key === "season.phases.Preseason") return "Preseason";
      if (key === "season.startsOn") return `Starts on ${params?.date}`;
      if (key === "season.noOpener") return "No opener";
      if (key === "season.standingsLocked") return "Standings are locked before kickoff.";
      if (key === "common.ordinal.one") return `${params?.n}st`;
      if (key === "common.ordinal.two") return `${params?.n}nd`;
      if (key === "common.ordinal.few") return `${params?.n}rd`;
      if (key === "common.ordinal.other") return `${params?.n}th`;
      if (key === "home.winningStreak") return "Winning Streak";
      if (key === "home.noLeague") return "No league data";
      // Portuguese, so the backend's English letters or a hard-coded English header would fail.
      if (key === "common.recentForm") return "Recent form";
      if (key === "common.formResult.W") return "V";
      if (key === "common.formResult.D") return "E";
      if (key === "common.formResult.L") return "D";
      if (key === "common.played") return "J";
      if (key === "common.won") return "V";
      if (key === "common.drawn") return "E";
      if (key === "common.lost") return "D";
      if (key === "common.playedFull") return "Jogos";
      if (key === "common.wonFull") return "Vitórias";
      if (key === "common.drawnFull") return "Empates";
      if (key === "common.lostFull") return "Derrotas";
      if (key === "common.pts") return "Pts";
      return key;
    },
    i18n: locale,
  }),
}));

describe("HomeLeaguePositionCard", () => {
  /**
   * Given a club in 21st, 22nd and 23rd place of a 24-team league (English endings that are not
   *       "th")
   * When the card is drawn
   * Then the place reads 21st, 22nd, 23rd, never "21th".
   */
  it.each([
    [21, "21st"],
    [22, "22nd"],
    [23, "23rd"],
    [24, "24th"],
  ])("writes place %i as %s", (place, written) => {
    render(
      <HomeLeaguePositionCard
        isPreseason={false}
        phase="RegularSeason"
        seasonStartLabel={null}
        myStanding={place}
        myStandingData={{
          team_id: "team-1",
          played: 5,
          won: 1,
          drawn: 1,
          lost: 3,
          goals_for: 4,
          goals_against: 9,
          points: 4,
        }}
        teamForm={[]}
      />,
    );

    expect(screen.getByText(written)).toBeInTheDocument();
  });

  /** Given a French club in 21st place, when the card renders, then it uses French ordinal rules. */
  it("writes a French 21st-place league position", () => {
    locale.language = "fr";
    try {
      render(
        <HomeLeaguePositionCard
          isPreseason={false}
          phase="RegularSeason"
          seasonStartLabel={null}
          myStanding={21}
          myStandingData={{
            team_id: "team-1",
            played: 1,
            won: 1,
            drawn: 0,
            lost: 0,
            goals_for: 1,
            goals_against: 0,
            points: 3,
          }}
          teamForm={[]}
        />,
      );
      expect(screen.getByText("21e")).toBeInTheDocument();
    } finally {
      locale.language = "en";
    }
  });

  /** Given a Russian club in second place, when the card renders, then its place is neuter. */
  it("writes a Russian league place in the neuter form", () => {
    locale.language = "ru";
    try {
      render(
        <HomeLeaguePositionCard
          isPreseason={false}
          phase="RegularSeason"
          seasonStartLabel={null}
          myStanding={2}
          myStandingData={{
            team_id: "team-1",
            played: 1,
            won: 1,
            drawn: 0,
            lost: 0,
            goals_for: 1,
            goals_against: 0,
            points: 3,
          }}
          teamForm={[]}
        />,
      );
      expect(screen.getByText("2-е")).toBeInTheDocument();
    } finally {
      locale.language = "en";
    }
  });

  it("renders preseason standings lock messaging", () => {
    render(
      <HomeLeaguePositionCard
        isPreseason={true}
        phase="Preseason"
        seasonStartLabel="Jan 12"
        myStanding={null}
        myStandingData={null}
        teamForm={[]}
      />,
    );

    expect(screen.getByText("Preseason")).toBeInTheDocument();
    expect(screen.getByText("Starts on Jan 12")).toBeInTheDocument();
    expect(screen.getByText("Standings are locked before kickoff.")).toBeInTheDocument();
  });

  it("renders league table summary and form streak data", () => {
    render(
      <HomeLeaguePositionCard
        isPreseason={false}
        phase="RegularSeason"
        seasonStartLabel={null}
        myStanding={2}
        myStandingData={{
          team_id: "team-1",
          played: 5,
          won: 3,
          drawn: 1,
          lost: 1,
          goals_for: 9,
          goals_against: 4,
          points: 10,
        }}
        teamForm={["W", "W", "W"]}
      />,
    );

    expect(screen.getByText("League Position")).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
    expect(screen.getByText("2nd")).toBeInTheDocument();
    expect(screen.getByText("10 Pts")).toBeInTheDocument();
    expect(screen.getByText("Winning Streak")).toBeInTheDocument();
  });

  /**
   * Given the club's standing (5 played: 3 won, 1 drawn, 1 lost)
   * When the card draws its four columns, in Portuguese
   * Then each is headed by the locale's abbreviation (J, V, E, D) and read as its word (Jogos,
   *      Vitórias, Empates, Derrotas), never an English P/W/D/L written into the card.
   */
  it("heads the standing's columns in the player's language", () => {
    render(
      <HomeLeaguePositionCard
        isPreseason={false}
        phase="RegularSeason"
        seasonStartLabel={null}
        myStanding={2}
        myStandingData={{
          team_id: "team-1",
          played: 5,
          won: 3,
          drawn: 1,
          lost: 1,
          goals_for: 9,
          goals_against: 4,
          points: 10,
        }}
        teamForm={[]}
      />,
    );

    for (const [letter, word] of [
      ["J", "Jogos"],
      ["V", "Vitórias"],
      ["E", "Empates"],
      ["D", "Derrotas"],
    ]) {
      expect(screen.getByText(letter)).toHaveAttribute("aria-hidden", "true");
      expect(screen.getByText(word)).toHaveClass("sr-only");
    }
    expect(screen.queryByText("P")).toBeNull();
    expect(screen.queryByText("W")).toBeNull();
  });

  /**
   * Given the club's form from the backend, its results as English letters (W, W, L)
   * When the league position card draws it, in Portuguese
   * Then it is the shared form list, each result in the locale's letter (V, V, D) and read as its
   *      word, not the backend's English letter on a raw-palette square as before.
   */
  it("draws the club's form in the player's own letters", () => {
    render(
      <HomeLeaguePositionCard
        isPreseason={false}
        phase="RegularSeason"
        seasonStartLabel={null}
        myStanding={2}
        myStandingData={{
          team_id: "team-1",
          played: 3,
          won: 2,
          drawn: 0,
          lost: 1,
          goals_for: 5,
          goals_against: 3,
          points: 6,
        }}
        teamForm={["W", "W", "L"]}
      />,
    );

    const form = screen.getByRole("list", { name: "Recent form" });
    const letters = within(form)
      .getAllByRole("listitem")
      .map((item) => item.querySelector('[aria-hidden="true"]')?.textContent);
    expect(letters).toEqual(["V", "V", "D"]);
  });
});
