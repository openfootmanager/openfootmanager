import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { GeneratedCrest } from "./GeneratedCrest";

describe("GeneratedCrest", () => {
  it("renders the label inside an svg crest", () => {
    const { container } = render(<GeneratedCrest name="Media FC" label="MFC" />);

    expect(container.querySelector("svg")).toBeInTheDocument();
    expect(screen.getByText("MFC")).toBeInTheDocument();
    // It is decorative — the accessible name comes from the surrounding label.
    expect(screen.queryByRole("img")).not.toBeInTheDocument();
  });

  it("paints the club's primary colour when provided", () => {
    const { container } = render(
      <GeneratedCrest
        name="Media FC"
        label="MFC"
        colors={{ primary: "#ff0000", secondary: "#0000ff" }}
      />,
    );

    const fills = Array.from(container.querySelectorAll("[fill]")).map((node) =>
      node.getAttribute("fill"),
    );
    expect(fills).toContain("#ff0000");
    expect(fills).toContain("#0000ff");
  });

  it("is deterministic: the same name yields the same crest markup", () => {
    const first = render(<GeneratedCrest name="Alpha United" label="ALP" />);
    const firstHtml = first.container.innerHTML.replace(/id="[^"]*"|url\(#[^)]*\)/g, "");
    first.unmount();

    const second = render(<GeneratedCrest name="Alpha United" label="ALP" />);
    const secondHtml = second.container.innerHTML.replace(/id="[^"]*"|url\(#[^)]*\)/g, "");

    expect(secondHtml).toBe(firstHtml);
  });

  /**
   * Given a green and white crest, whose label used to vanish on the white half
   * When it renders
   * Then the label is the dark ink with a light halo, so it reads on both colours.
   */
  it("keeps the label readable on both halves of a green and white crest", () => {
    render(
      <GeneratedCrest
        name="Alpha"
        label="AL"
        colors={{ primary: "#00ff00", secondary: "#ffffff" }}
      />,
    );

    const label = screen.getByTestId("crest-label");

    expect(label).toHaveAttribute("fill", "#111827");
    expect(label).toHaveAttribute("stroke", "#ffffff");
  });

  /**
   * Given colours written as three hex digits and as a name, which the old parser read as white
   * When the crest renders
   * Then the ink is chosen from what they really are.
   */
  it("reads three-digit and named colours instead of falling back to white", () => {
    render(
      <GeneratedCrest name="Bravo" label="BR" colors={{ primary: "#ff0", secondary: "white" }} />,
    );

    expect(screen.getByTestId("crest-label")).toHaveAttribute("fill", "#111827");
  });

  /**
   * Given a dark crest
   * When it renders
   * Then the label is white with a dark halo.
   */
  it("draws a light label with a dark halo on a dark crest", () => {
    render(
      <GeneratedCrest
        name="Charlie"
        label="CH"
        colors={{ primary: "#001a33", secondary: "#000000" }}
      />,
    );

    const label = screen.getByTestId("crest-label");

    expect(label).toHaveAttribute("fill", "#ffffff");
    expect(label).toHaveAttribute("stroke", "#111827");
  });
});
