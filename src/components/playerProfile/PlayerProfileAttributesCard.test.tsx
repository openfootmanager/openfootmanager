import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { PlayerData } from "../../store/gameStore";
import type { PlayerAttributeGroup } from "./PlayerProfile.attributes";
import PlayerProfileAttributesCard from "./PlayerProfileAttributesCard";

const groups: PlayerAttributeGroup[] = [
  { label: "Physical", attrs: [{ name: "Pace", value: 71 }], average: 71 },
];

function renderCard(props: Partial<Parameters<typeof PlayerProfileAttributesCard>[0]>) {
  render(
    <PlayerProfileAttributesCard
      attrGroups={groups}
      player={{} as PlayerData}
      attributesVisible={false}
      title="Attributes"
      averageLabel="Average"
      hiddenTitle="Attributes Hidden"
      hiddenBody="Scout this player"
      listLabel="List"
      radarLabel="Radar"
      {...props}
    />,
  );
}

describe("PlayerProfileAttributesCard", () => {
  /**
   * Given a player nobody has scouted or signed
   * When the card renders
   * Then the attributes are hidden behind the placeholder
   */
  it("hides the attributes when they are not visible", () => {
    renderCard({});

    expect(screen.getByText("Attributes Hidden")).toBeInTheDocument();
    expect(screen.queryByText("71")).not.toBeInTheDocument();
  });

  /**
   * Given a scouted player with a current report
   * When the card renders
   * Then the scouted values show under a "scouted on" note, without the out-of-date mark
   */
  it("shows the scouted attributes with the date they were scouted", () => {
    renderCard({
      attributesVisible: true,
      scoutNote: {
        scoutedLabel: "Scouted on 10 Sep 2026",
        outOfDate: false,
        outOfDateLabel: "Out of date",
      },
    });

    expect(screen.getByText("Scouted on 10 Sep 2026")).toBeInTheDocument();
    expect(screen.getAllByText("71").length).toBeGreaterThan(0);
    expect(screen.queryByText("Out of date")).not.toBeInTheDocument();
  });

  /**
   * Given a report from before this season
   * When the card renders
   * Then it is marked out of date
   */
  it("marks an old report as out of date", () => {
    renderCard({
      attributesVisible: true,
      scoutNote: {
        scoutedLabel: "Scouted on 10 Mar 2026",
        outOfDate: true,
        outOfDateLabel: "Out of date",
      },
    });

    expect(screen.getByText("Out of date")).toBeInTheDocument();
  });

  /**
   * Given a player at the manager's own club
   * When the card renders
   * Then no scouting note appears
   */
  it("shows no scouting note for a first-team player", () => {
    renderCard({ attributesVisible: true });

    expect(screen.queryByText(/Scouted on/)).not.toBeInTheDocument();
  });

  /**
   * Given a visible attributes card
   * When the list and radar toggle render
   * Then both carry a visible focus ring in light and dark themes
   */
  it("gives the view toggle buttons a focus ring for both themes", () => {
    renderCard({ attributesVisible: true });

    for (const name of ["List", "Radar"]) {
      const button = screen.getByRole("button", { name });
      expect(button.className).toContain("focus-visible:ring-2");
      expect(button.className).toContain("dark:focus-visible:ring-primary-400");
    }
  });

  /**
   * Given a player nobody has scouted
   * When the card renders
   * Then it says the attributes are unknown, and shows no attribute names, bars or numbers
   */
  it("shows an explicit unknown state instead of invented bars", () => {
    renderCard({});

    expect(screen.getByText("Attributes Hidden")).toBeInTheDocument();
    expect(screen.getByText("Scout this player")).toBeInTheDocument();
    expect(screen.queryByText("Physical")).not.toBeInTheDocument();
    expect(screen.queryByText("Pace")).not.toBeInTheDocument();
    expect(screen.queryByText("??")).not.toBeInTheDocument();
  });

  /**
   * Given the same unscouted player in two game languages (attribute names translated)
   * When the card renders in each
   * Then the unknown state is identical: nothing in it is derived from the translated names
   */
  it("does not change with the language of the attribute names", () => {
    const rendered = (label: string, name: string) => {
      const { container, unmount } = render(
        <PlayerProfileAttributesCard
          attrGroups={[{ label, attrs: [{ name, value: 71 }], average: 71 }]}
          player={{} as PlayerData}
          attributesVisible={false}
          title="Attributes"
          averageLabel="Average"
          hiddenTitle="Attributes Hidden"
          hiddenBody="Scout this player"
          listLabel="List"
          radarLabel="Radar"
        />,
      );
      const html = container.innerHTML;
      unmount();
      return html;
    };

    expect(rendered("Physical", "Pace")).toBe(rendered("Físico", "Ritmo"));
  });
});
