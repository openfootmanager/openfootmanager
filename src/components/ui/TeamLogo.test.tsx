import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { TeamLogo } from "./TeamLogo";

const team = {
  name: "Media FC",
  short_name: "MFC",
};

describe("TeamLogo", () => {
  it("renders the team short name when no logo media path is provided", () => {
    render(<TeamLogo team={team} />);

    expect(screen.getByText("MFC")).toBeInTheDocument();
    expect(screen.queryByRole("img")).not.toBeInTheDocument();
  });

  it("renders a local logo image when logo media path is provided", () => {
    render(
      <TeamLogo
        team={{
          ...team,
          media: { logo: "assets/worlds/test-world/teams/media-fc.png" },
        }}
      />,
    );

    expect(screen.getByRole("img", { name: "Media FC logo" })).toHaveAttribute(
      "src",
      "/assets/worlds/test-world/teams/media-fc.png",
    );
  });

  it("falls back to the team short name when the logo image fails to load", () => {
    render(
      <TeamLogo
        team={{
          ...team,
          media: { logo: "/assets/worlds/test-world/teams/media-fc.png" },
        }}
      />,
    );

    fireEvent.error(screen.getByRole("img", { name: "Media FC logo" }));

    expect(screen.getByText("MFC")).toBeInTheDocument();
  });

  it("tries to render a new logo path after a previous image failed", () => {
    const { rerender } = render(
      <TeamLogo
        team={{
          ...team,
          media: { logo: "/assets/worlds/test-world/teams/missing.png" },
        }}
      />,
    );

    fireEvent.error(screen.getByRole("img", { name: "Media FC logo" }));
    expect(screen.getByText("MFC")).toBeInTheDocument();

    rerender(
      <TeamLogo
        team={{
          ...team,
          media: { logo: "/assets/worlds/test-world/teams/media-fc.png" },
        }}
      />,
    );

    expect(screen.getByRole("img", { name: "Media FC logo" })).toHaveAttribute(
      "src",
      "/assets/worlds/test-world/teams/media-fc.png",
    );
  });
});
