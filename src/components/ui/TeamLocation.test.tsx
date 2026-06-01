import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { TeamLocation } from "./TeamLocation";

describe("TeamLocation", () => {
  it("renders the city, localised country name, and flag", () => {
    render(<TeamLocation city="London" countryCode="GB" locale="en" />);

    expect(screen.getByText(/London, United Kingdom/i)).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "United Kingdom" })).toBeInTheDocument();
  });

  it("renders football nation labels for non-ISO codes", () => {
    render(<TeamLocation city="Cardiff" countryCode="WAL" locale="en" />);

    expect(screen.getByText(/Cardiff, Wales/i)).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "Wales" })).toBeInTheDocument();
  });
});