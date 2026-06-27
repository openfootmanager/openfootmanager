import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { PlayerAvatar } from "./PlayerAvatar";

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: vi.fn((path: string) => path),
  invoke: vi.fn(),
  isTauri: vi.fn(() => false),
}));

const player = {
  id: "player-1",
  full_name: "John Smith",
  match_name: "J. Smith",
  nationality: "GB",
  date_of_birth: "2000-01-01",
};

const mockedInvoke = vi.mocked(invoke);
const mockedIsTauri = vi.mocked(isTauri);

describe("PlayerAvatar", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedIsTauri.mockReturnValue(false);
  });

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

  it("keeps initials visible until a runtime portrait image loads", async () => {
    mockedIsTauri.mockReturnValue(true);
    mockedInvoke.mockResolvedValue({
      generator: "test",
      cacheKey: "player-1",
      sourceId: "player-1",
      cachePath: "/tmp/player-1.png",
      dataUrl: null,
      generated: true,
      renderMs: 10,
      elapsedMs: 10,
      width: 128,
      height: 128,
    });

    render(<PlayerAvatar player={player} />);

    expect(screen.getByText("J.")).toBeInTheDocument();
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("generate_player_portrait", {
        request: expect.objectContaining({ playerId: "player-1" }),
      });
    });
    const portrait = await screen.findByRole("img", { name: "John Smith" });

    expect(screen.getByText("J.")).toBeInTheDocument();
    expect(portrait).toHaveClass("opacity-0");

    fireEvent.load(portrait);

    expect(portrait).toHaveClass("opacity-100");
  });

  it("hides the runtime fallback from assistive tech after the portrait loads", async () => {
    mockedIsTauri.mockReturnValue(true);
    mockedInvoke.mockResolvedValue({
      generator: "test",
      cacheKey: "player-1",
      sourceId: "player-1",
      cachePath: "/tmp/player-1.png",
      dataUrl: null,
      generated: true,
      renderMs: 10,
      elapsedMs: 10,
      width: 128,
      height: 128,
    });

    render(<PlayerAvatar player={player} fallback={<span>JS</span>} />);

    const fallback = screen.getByText("JS");
    expect(fallback.parentElement).not.toHaveAttribute("aria-hidden");

    const portrait = await screen.findByRole("img", { name: "John Smith" });
    fireEvent.load(portrait);

    expect(screen.getByText("JS").parentElement).toHaveAttribute(
      "aria-hidden",
      "true",
    );
  });

  it("can skip runtime portrait generation when explicitly disabled", async () => {
    mockedIsTauri.mockReturnValue(true);
    mockedInvoke.mockResolvedValue({
      generator: "test",
      cacheKey: "player-1",
      sourceId: "player-1",
      cachePath: "/tmp/player-1.png",
      dataUrl: null,
      generated: true,
      renderMs: 10,
      elapsedMs: 10,
      width: 128,
      height: 128,
    });

    render(<PlayerAvatar player={player} enableRuntimePortrait={false} />);

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(mockedInvoke).not.toHaveBeenCalledWith(
      "generate_player_portrait",
      expect.anything(),
    );
    expect(screen.getByText("J.")).toBeInTheDocument();
  });
});
