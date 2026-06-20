import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { GeneratedCrest } from "./GeneratedCrest";

describe("GeneratedCrest", () => {
  it("renders the label inside an svg crest", () => {
    const { container } = render(
      <GeneratedCrest name="Media FC" label="MFC" />,
    );

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
    const secondHtml = second.container.innerHTML.replace(
      /id="[^"]*"|url\(#[^)]*\)/g,
      "",
    );

    expect(secondHtml).toBe(firstHtml);
  });
});
