import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { PreviewRow } from "./PreviewRow";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "ru" } }),
}));

describe("PreviewRow", () => {
  /**
   * Given the UI language is Russian
   * When the preview lists a log file
   * Then its size reads in Russian, the number and the unit both.
   */
  it("shows each file's size in the UI language", () => {
    render(
      <PreviewRow
        icon={null}
        name="Logs"
        detail="Recent logs"
        files={[{ name: "app.log", bytes: 1_468_006 }]}
      />,
    );

    expect(screen.getByText("1,4 МБ")).toBeInTheDocument();
  });
});
