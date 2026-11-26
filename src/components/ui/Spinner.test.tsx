import { render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it } from "vitest";

import i18n, { i18nReady } from "../../i18n";
import { Spinner } from "./Spinner";

beforeAll(async () => {
  await i18nReady;
  await i18n.changeLanguage("en");
});

describe("Spinner", () => {
  /** Given no label, when the spinner renders, then its status reads "Loading...". */
  it("announces the default loading status", () => {
    render(<Spinner />);

    expect(screen.getByRole("status")).toHaveTextContent("Loading...");
  });

  /** Given a caller's label, when the spinner renders, then its status reads that label. */
  it("announces the label the caller gives it", () => {
    render(<Spinner label="Loading saves..." />);

    expect(screen.getByRole("status")).toHaveTextContent("Loading saves...");
  });

  /** Given a decorative spinner, when it renders, then it exposes no status of its own. */
  it("exposes no status when it is decorative", () => {
    render(<Spinner decorative />);

    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });
});
