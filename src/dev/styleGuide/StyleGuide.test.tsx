import { readFileSync } from "node:fs";

import { render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";

import { ThemeProvider } from "../../context/ThemeContext";
import { i18nReady } from "../../i18n";
import { StyleGuide } from "./StyleGuide";
import { applyStyleGuideOptions, readStyleGuideOptions } from "./styleGuideOptions";

const SOURCES = import.meta.glob(["../../components/ui/index.ts", "./StyleGuide.tsx"], {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

/** Every design token `@theme` in App.css defines (a `--text-*--line-height` belongs to its size). */
function themeTokens(): string[] {
  // Read with node:fs: Vitest stubs CSS imports to "" even with ?raw.
  const css = readFileSync("src/App.css", "utf-8");
  const theme = css.slice(css.indexOf("@theme {"), css.indexOf("\n}", css.indexOf("@theme {")));
  return [...theme.matchAll(/^\s*(--[a-z0-9-]+):/gm)]
    .map((match) => match[1])
    .filter((name) => !name.includes("--line-height"));
}

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
   * Given every token App.css's @theme defines,
   * when the style guide's source is read,
   * then each one is named there, so the guide shows every token and a new one can't ship unseen.
   */
  it("names every theme token", () => {
    const guide = SOURCES["./StyleGuide.tsx"];
    const tokens = themeTokens();
    expect(tokens.length).toBeGreaterThan(80);
    expect(tokens.filter((token) => !guide.includes(`"${token}"`))).toEqual([]);
  });

  /**
   * Given the type specimens,
   * when the guide renders,
   * then each is set by its static role class, never an inline size, as the type rule says.
   */
  it("sets each type specimen with its role class", () => {
    render(
      <ThemeProvider>
        <StyleGuide options={readStyleGuideOptions("")} />
      </ThemeProvider>,
    );
    for (const role of ["caption", "body", "lead", "heading", "title", "display"]) {
      const specimen = screen.getByText(new RegExp(`^${role}: Callum Fraser`));
      expect(specimen).toHaveClass(`text-${role}`);
      expect(specimen.style.fontSize).toBe("");
    }
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
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  /**
   * Given a URL with dialog=open,
   * when the guide renders,
   * then the dialog is already open, named and showing, so it can be screenshotted.
   */
  it("opens the dialog when the URL asks for it", () => {
    render(
      <ThemeProvider>
        <StyleGuide options={readStyleGuideOptions("?dialog=open")} />
      </ThemeProvider>,
    );
    expect(screen.getByRole("dialog", { name: "Go to the field?" })).toBeInTheDocument();
  });

  /**
   * Given dark, compact, extra large and high contrast in the URL,
   * when the options are put on the document,
   * then the root carries the three classes and the 20px root size, and the theme is stored under
   * the guide's own key, never the app's; light, comfortable and normal take the classes off again.
   */
  it("puts every option on the document", () => {
    const root = document.createElement("html");
    const stored = new Map<string, string>();
    const storage = { setItem: (key: string, value: string) => stored.set(key, value) };

    applyStyleGuideOptions(
      readStyleGuideOptions("?theme=dark&density=compact&scale=xlarge&contrast=high"),
      root,
      storage,
    );
    expect([...root.classList].sort()).toEqual(["dark", "density-compact", "high-contrast"]);
    expect(root.style.fontSize).toBe("20px");
    expect(stored.get("ofm-style-guide-theme")).toBe("dark");
    expect(stored.has("ofm-theme")).toBe(false);

    applyStyleGuideOptions(readStyleGuideOptions(""), root, storage);
    expect([...root.classList]).toEqual([]);
    expect(root.style.fontSize).toBe("16px");
    expect(stored.get("ofm-style-guide-theme")).toBe("light");
  });
});
