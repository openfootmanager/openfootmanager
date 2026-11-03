import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { EMPTY_DRAFT } from "./ReportBugModal.helpers";
import { ReportDescribeStep } from "./ReportDescribeStep";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

describe("ReportDescribeStep", () => {
  /**
   * Given the frequency choice
   * When its buttons render
   * Then each has a focus ring with the offset in both themes
   */
  it("gives the frequency buttons a focus ring offset for light and dark", () => {
    render(
      <ReportDescribeStep
        draft={EMPTY_DRAFT}
        onDraftChange={vi.fn()}
        showErrors={false}
        missing={[]}
      />,
    );

    for (const name of ["everyTime", "sometimes", "once"]) {
      const button = screen.getByRole("button", { name: `reportBug.frequency.${name}` });
      expect(button.className).toContain("focus:ring-offset-2");
      expect(button.className).toContain("dark:focus:ring-offset-navy-800");
    }
  });
});
