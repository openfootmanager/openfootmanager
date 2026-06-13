import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { PlayerAvatar } from "./PlayerAvatar";

const player = {
  full_name: "John Smith",
  match_name: "J. Smith",
};

describe("PlayerAvatar", () => {
  it("renders player initials when no face media path is provided", () => {
    render(<PlayerAvatar player={player} />);

    expect(screen.getByText("J.")).toBeInTheDocument();
    expect(screen.queryByRole("img")).not.toBeInTheDocument();
  });

  it("renders a local face image when face media path is provided", () => {
    render(
      <PlayerAvatar
        player={{
          ...player,
          media: { face: "assets/worlds/test-world/players/player-1.png" },
        }}
      />,
    );

    expect(screen.getByRole("img", { name: "John Smith" })).toHaveAttribute(
      "src",
      "/assets/worlds/test-world/players/player-1.png",
    );
  });

  it("falls back to initials when the face image fails to load", () => {
    render(
      <PlayerAvatar
        player={{
          ...player,
          media: { face: "/assets/worlds/test-world/players/player-1.png" },
        }}
      />,
    );

    fireEvent.error(screen.getByRole("img", { name: "John Smith" }));

    expect(screen.getByText("J.")).toBeInTheDocument();
  });
});
