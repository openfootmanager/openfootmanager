import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { GeneratedAvatar } from "./GeneratedAvatar";

describe("GeneratedAvatar", () => {
  it("renders the initials inside an svg avatar", () => {
    const { container } = render(
      <GeneratedAvatar name="John Smith" initials="JS" />,
    );

    expect(container.querySelector("svg")).toBeInTheDocument();
    expect(screen.getByText("JS")).toBeInTheDocument();
    expect(screen.queryByRole("img")).not.toBeInTheDocument();
  });

  it("is deterministic: the same name yields the same background colour", () => {
    const first = render(<GeneratedAvatar name="John Smith" initials="JS" />);
    const firstFill = first.container
      .querySelector("rect")
      ?.getAttribute("fill");
    first.unmount();

    const second = render(<GeneratedAvatar name="John Smith" initials="JS" />);
    const secondFill = second.container
      .querySelector("rect")
      ?.getAttribute("fill");

    expect(secondFill).toBe(firstFill);
  });

  it("gives different names different background colours", () => {
    const a = render(<GeneratedAvatar name="John Smith" initials="JS" />);
    const aFill = a.container.querySelector("rect")?.getAttribute("fill");
    a.unmount();

    const b = render(<GeneratedAvatar name="Zoe Vanguard" initials="ZV" />);
    const bFill = b.container.querySelector("rect")?.getAttribute("fill");

    expect(bFill).not.toBe(aFill);
  });
});
