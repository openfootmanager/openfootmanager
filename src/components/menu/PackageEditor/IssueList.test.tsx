import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { IssueList } from "./IssueList";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, unknown>) =>
      options ? `${key}|${JSON.stringify(options)}` : key,
  }),
}));

const countryWarning = {
  code: "be.warning.package.countryWithoutRegion",
  file: "countries/qq.json",
  params: { name: "Qland", code: "QQ" },
};

describe("IssueList", () => {
  // Given a warning from the package (#830)
  // When the list renders it as a warning
  // Then it shows the file and the translated message with its params, under the warnings heading.
  it("shows a warning with its file, message and parameters", () => {
    render(<IssueList issues={[countryWarning]} severity="warning" />);

    expect(screen.getByText(/countries\/qq\.json/)).toBeInTheDocument();
    expect(
      screen.getByText(/be\.warning\.package\.countryWithoutRegion\|.*"code":"QQ"/),
    ).toBeInTheDocument();
    expect(screen.getByText(/worldEditor\.warnings/)).toBeInTheDocument();
    expect(screen.queryByText(/worldEditor\.issues\|/)).not.toBeInTheDocument();
  });

  // Given errors
  // When the list renders them (the default)
  // Then the errors heading is used, as before.
  it("keeps the errors heading for errors", () => {
    render(<IssueList issues={[{ ...countryWarning, code: "be.error.package.x" }]} />);

    expect(screen.getByText(/worldEditor\.issues\|/)).toBeInTheDocument();
  });

  // Given nothing to report
  // When it renders
  // Then nothing is drawn.
  it("draws nothing for an empty list", () => {
    const { container } = render(<IssueList issues={[]} severity="warning" />);
    expect(container).toBeEmptyDOMElement();
  });
});
