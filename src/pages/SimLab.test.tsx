import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import SimLab from "./SimLab";

const invokeMock = vi.fn((_command: string, _args?: unknown) => new Promise(() => undefined));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (command: string, args?: unknown) => invokeMock(command, args),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: { games?: number }) => `${key}:${options?.games ?? ""}`,
  }),
}));

describe("SimLab", () => {
  /**
   * Given a batch submitted for 1,000 games
   * When the games field is changed to 5,000 while it still runs
   * Then the status announces the submitted 1,000, not the edited 5,000.
   */
  it("announces the game count the running batch was submitted with", async () => {
    render(<SimLab />);

    fireEvent.click(screen.getByRole("button", { name: /Run/ }));

    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent("simLab.simulatingGames:1000"),
    );
    expect(invokeMock).toHaveBeenCalledWith(
      "run_sim_batch",
      expect.objectContaining({ config: expect.objectContaining({ games: 1000 }) }),
    );

    fireEvent.change(screen.getByLabelText(/Games/), { target: { value: "5000" } });

    expect(screen.getByRole("status")).toHaveTextContent("simLab.simulatingGames:1000");
  });
});
