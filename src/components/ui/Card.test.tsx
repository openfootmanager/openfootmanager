import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Card, CardHeader, CardBody } from "./Card";

describe("Card", () => {
  it("renders children", () => {
    render(
      <Card>
        <p>Card content</p>
      </Card>,
    );
    expect(screen.getByText("Card content")).toBeInTheDocument();
  });

  it("renders as a div", () => {
    const { container } = render(<Card>Content</Card>);
    expect(container.firstChild?.nodeName).toBe("DIV");
  });

  it("applies 'none' accent by default (no top border)", () => {
    const { container } = render(<Card>Default</Card>);
    const el = container.firstChild as HTMLElement;
    expect(el.className).toContain("border-gray-200");
    expect(el.className).not.toContain("border-t-4");
  });

  it("applies primary accent border", () => {
    const { container } = render(<Card accent="primary">Primary</Card>);
    const el = container.firstChild as HTMLElement;
    expect(el.className).toContain("border-t-4");
    expect(el.className).toContain("border-t-primary-500");
  });

  it("applies accent accent border", () => {
    const { container } = render(<Card accent="accent">Accent</Card>);
    const el = container.firstChild as HTMLElement;
    expect(el.className).toContain("border-t-accent-400");
  });

  it("applies success accent border", () => {
    const { container } = render(<Card accent="success">Success</Card>);
    const el = container.firstChild as HTMLElement;
    expect(el.className).toContain("border-t-success-400");
  });

  it("applies danger accent border", () => {
    const { container } = render(<Card accent="danger">Danger</Card>);
    const el = container.firstChild as HTMLElement;
    expect(el.className).toContain("border-t-red-500");
  });

  it("merges custom className", () => {
    const { container } = render(<Card className="my-card">Custom</Card>);
    const el = container.firstChild as HTMLElement;
    expect(el.className).toContain("my-card");
  });
});

describe("CardHeader", () => {
  it("renders children as heading text", () => {
    render(<CardHeader>My Title</CardHeader>);
    expect(screen.getByText("My Title")).toBeInTheDocument();
  });

  it("renders action slot when provided", () => {
    render(<CardHeader action={<button>Edit</button>}>Title</CardHeader>);
    expect(screen.getByText("Edit")).toBeInTheDocument();
  });

  it("does not render action when not provided", () => {
    const { container } = render(<CardHeader>No Action</CardHeader>);
    // Only the h3 child, no extra action element
    const wrapper = container.firstChild as HTMLElement;
    expect(wrapper.children).toHaveLength(1);
  });

  it("merges custom className", () => {
    const { container } = render(<CardHeader className="custom-header">Title</CardHeader>);
    const el = container.firstChild as HTMLElement;
    expect(el.className).toContain("custom-header");
  });
});

describe("CardBody", () => {
  it("renders children", () => {
    render(
      <CardBody>
        <span>Body text</span>
      </CardBody>,
    );
    expect(screen.getByText("Body text")).toBeInTheDocument();
  });

  it("applies default padding", () => {
    const { container } = render(<CardBody>Padded</CardBody>);
    const el = container.firstChild as HTMLElement;
    expect(el.className).toContain("p-6");
  });

  it("merges custom className", () => {
    const { container } = render(<CardBody className="extra">Content</CardBody>);
    const el = container.firstChild as HTMLElement;
    expect(el.className).toContain("extra");
  });
});
