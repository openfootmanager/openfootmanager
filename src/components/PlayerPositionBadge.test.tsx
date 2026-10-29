import { render, screen, waitFor } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import { resetPositionGroupsCache } from "../hooks/usePositionGroups";
import { i18nReady } from "../i18n";
import PlayerPositionBadge from "./PlayerPositionBadge";

vi.mock("../services/positionService", () => ({ getPositionGroups: vi.fn() }));

const { getPositionGroups } = await import("../services/positionService");

describe("PlayerPositionBadge", () => {
  beforeAll(async () => {
    await i18nReady;
  });

  beforeEach(() => {
    resetPositionGroupsCache();
    vi.mocked(getPositionGroups).mockResolvedValue([{ position: "Striker", group: "Forward" }]);
  });

  /**
   * Given a striker
   * When his position badge is drawn without options
   * Then it shows the short label and names the position in words for a screen reader.
   */
  it("shows the short label and names the position", () => {
    render(<PlayerPositionBadge position="Striker" />);

    expect(screen.getByText("ST")).toBeInTheDocument();
    expect(screen.getByText("Striker")).toHaveClass("sr-only");
  });

  /**
   * Given a screen that has room for the position's name
   * When the badge is drawn long
   * Then it shows the translated name instead of the short label.
   */
  it("shows the position's name when asked for the long form", () => {
    render(<PlayerPositionBadge position="CenterBack" long />);

    expect(screen.getByText("Center Back")).toBeInTheDocument();
    expect(screen.queryByText("CB")).not.toBeInTheDocument();
  });

  /**
   * Given a backend that puts the striker in the forward group
   * When the badge is drawn
   * Then its tone follows that group, once the table has arrived.
   */
  it("takes its tone from the backend's group", async () => {
    const { container } = render(<PlayerPositionBadge position="Striker" />);

    await waitFor(() => expect(container.querySelector(".bg-danger-tint")).not.toBeNull());
  });
});
