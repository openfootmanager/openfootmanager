import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ReportDoneStep } from "./ReportDoneStep";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "ru" } }),
}));

describe("ReportDoneStep", () => {
  /**
   * Given the UI language is Russian
   * When the confirmation names the saved bundle
   * Then its size reads in Russian, the number and the unit both.
   */
  it("shows the bundle size in the UI language", () => {
    render(
      <ReportDoneStep
        summary={{
          path: "/home/x/ofm-report.zip",
          bytes: 1_468_006,
          log_files: ["app.log"],
          included_save: false,
          included_crash: false,
        }}
        browserFailed={false}
      />,
    );

    expect(screen.getByText("/home/x/ofm-report.zip (1,4 МБ)")).toBeInTheDocument();
  });
});
