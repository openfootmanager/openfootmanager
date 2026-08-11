import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CountryFlag } from "./CountryFlag";

describe("CountryFlag", () => {
  it("renders an SVG flag for valid country codes", () => {
    const { container } = render(<CountryFlag code="GB" locale="en" />);

    expect(screen.getByRole("img", { name: "United Kingdom" })).toBeInTheDocument();
    expect(container.querySelector("svg")).not.toBeNull();
  });

  it("returns null for invalid country codes", () => {
    const { container } = render(<CountryFlag code="ZZ" />);

    expect(container).toBeEmptyDOMElement();
  });

  it("renders an SVG flag for UK home nations", () => {
    const { container } = render(<CountryFlag code="ENG" locale="en" />);

    expect(screen.getByRole("img", { name: "England" })).toBeInTheDocument();
    expect(container.querySelector("svg")).not.toBeNull();
  });

  it("renders an SVG flag when given a recognised country name", () => {
    const { container } = render(<CountryFlag code="Spain" locale="en" />);

    expect(screen.getByRole("img", { name: "Spain" })).toBeInTheDocument();
    expect(container.querySelector("svg")).not.toBeNull();
  });

  it("says nothing at all when it is decoration", () => {
    // Used beside the country's own name, the flag repeats it. Hiding the image
    // but leaving a `title` puts the name straight back — a title is announced
    // as well as shown — which is how the option read "EnglandEngland".
    const { container } = render(<CountryFlag code="ENG" locale="en" decorative />);

    expect(screen.queryByRole("img")).not.toBeInTheDocument();
    expect(container.querySelector("[title]")).toBeNull();
  });
});