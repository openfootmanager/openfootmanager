import { render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";

import { ThemeProvider } from "../../context/ThemeContext";
import { i18nReady } from "../../i18n";
import { StyleGuide } from "./StyleGuide";
import { readStyleGuideOptions } from "./styleGuideOptions";

const SOURCES = import.meta.glob(["../../components/ui/index.ts", "./StyleGuide.tsx"], {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

/** The components index.ts exports: hooks and types are not drawn, so they are left out. */
function exportedComponents(): string[] {
  const index = SOURCES["../../components/ui/index.ts"];
  return [...index.matchAll(/^export \{([^}]+)\}/gm)]
    .flatMap((match) => match[1].split(","))
    .map((name) => name.trim().replace(/^default as /, ""))
    .filter((name) => /^[A-Z]/.test(name));
}

describe("style guide", () => {
  beforeAll(async () => {
    // jsdom has no matchMedia; ThemeProvider asks it for the system theme.
    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: vi.fn().mockImplementation((query: string) => ({
        matches: false,
        media: query,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      })),
    });
    await i18nReady;
  });

  /**
   * Given every component exported from src/components/ui,
   * when the style guide's source is read,
   * then each one is drawn there, so a new primitive can't ship without an entry.
   */
  it("draws every exported primitive", () => {
    const guide = SOURCES["./StyleGuide.tsx"];
    const missing = exportedComponents().filter(
      (name) => !new RegExp(`<${name}[\\s>/]`).test(guide),
    );
    expect(exportedComponents().length).toBeGreaterThan(10);
    expect(missing).toEqual([]);
  });

  /**
   * Given a URL asking for dark, compact, extra large and high contrast,
   * when the options are read,
   * then each is honoured and the root size is the one ui_scale applies (20px).
   */
  it("reads every option from the URL", () => {
    expect(
      readStyleGuideOptions("?theme=dark&density=compact&scale=xlarge&contrast=high&dialog=open"),
    ).toEqual({
      theme: "dark",
      density: "compact",
      scale: "xlarge",
      highContrast: true,
      rootFontSize: "20px",
      dialogOpen: true,
    });
  });

  /** Given an empty or nonsense URL, then the guide falls back to light, comfortable, normal. */
  it("falls back to the defaults for unknown values", () => {
    expect(readStyleGuideOptions("?theme=sepia&scale=huge")).toMatchObject({
      theme: "light",
      density: "comfortable",
      scale: "normal",
      rootFontSize: "16px",
      highContrast: false,
    });
  });

  /**
   * Given the default options,
   * when the guide renders,
   * then every section is on the page, so a primitive that throws without a backend fails here.
   */
  it("renders every section without a backend", () => {
    render(
      <ThemeProvider>
        <StyleGuide options={readStyleGuideOptions("")} />
      </ThemeProvider>,
    );
    for (const section of ["Tokens", "Controls", "Display", "Cards", "Pitch tokens", "Dialog"]) {
      expect(screen.getByRole("region", { name: section })).toBeInTheDocument();
    }
  });
});
