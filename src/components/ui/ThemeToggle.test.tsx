import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ThemeToggle } from "./ThemeToggle";
import { ThemeProvider } from "../../context/ThemeContext";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => {
      if (key === "settings.switchToLightMode") return "Switch to light mode";
      if (key === "settings.switchToDarkMode") return "Switch to dark mode";
      return key;
    },
  }),
}));

// jsdom doesn't implement matchMedia
Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: query === "(prefers-color-scheme: dark)",
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

beforeEach(() => {
  localStorage.clear();
  document.documentElement.classList.remove("dark");
});

function renderWithTheme() {
  return render(
    <ThemeProvider>
      <ThemeToggle />
    </ThemeProvider>
  );
}

describe("ThemeToggle", () => {
  it("renders a button", () => {
    renderWithTheme();
    expect(screen.getByRole("button")).toBeInTheDocument();
  });

  it("shows 'Switch to light mode' title when in dark mode", () => {
    renderWithTheme();
    expect(screen.getByTitle("Switch to light mode")).toBeInTheDocument();
  });

  it("toggles to light mode on click, updating title", () => {
    renderWithTheme();
    fireEvent.click(screen.getByRole("button"));
    expect(screen.getByTitle("Switch to dark mode")).toBeInTheDocument();
  });

  it("toggles back to dark mode on double click", () => {
    renderWithTheme();
    fireEvent.click(screen.getByRole("button")); // dark -> light
    fireEvent.click(screen.getByRole("button")); // light -> dark
    expect(screen.getByTitle("Switch to light mode")).toBeInTheDocument();
  });

  it("merges custom className", () => {
    render(
      <ThemeProvider>
        <ThemeToggle className="my-toggle" />
      </ThemeProvider>
    );
    expect(screen.getByRole("button").className).toContain("my-toggle");
  });
});
