import { describe, it, expect } from "vitest";
import {
  countryFlag,
  countryMarker,
  countryName,
  allCountries,
  isValidCountryCode,
  normaliseNationality,
  shouldUseCountryCodeBadge,
} from "./countries";

// ---------------------------------------------------------------------------
// countryFlag
// ---------------------------------------------------------------------------

describe("countryFlag", () => {
  it("returns flag emoji for valid alpha-2 codes", () => {
    // US flag = 🇺🇸 (regional indicator U + S)
    expect(countryFlag("US")).toBe("🇺🇸");
    expect(countryFlag("GB")).toBe("🇬🇧");
    expect(countryFlag("BR")).toBe("🇧🇷");
  });

  it("handles lowercase input", () => {
    expect(countryFlag("us")).toBe("🇺🇸");
  });

  it("returns empty string for invalid input", () => {
    expect(countryFlag("")).toBe("");
    expect(countryFlag("X")).toBe("");
    expect(countryFlag("USA")).toBe("");
  });
});

describe("countryMarker", () => {
  it("uses ISO country code badges on Windows platforms", () => {
    expect(shouldUseCountryCodeBadge("Win32")).toBe(true);
    expect(countryMarker("br", "Windows NT 10.0")).toBe("BR");
  });

  it("uses flag emoji on non-Windows platforms", () => {
    expect(shouldUseCountryCodeBadge("MacIntel")).toBe(false);
    expect(countryMarker("BR", "Linux x86_64")).toBe("🇧🇷");
  });
});

// ---------------------------------------------------------------------------
// countryName
// ---------------------------------------------------------------------------

describe("countryName", () => {
  it("returns English country name by default", () => {
    expect(countryName("GB")).toMatch(/United Kingdom/);
    expect(countryName("US")).toMatch(/United States/);
  });

  it("returns localised name for supported locales", () => {
    const nameDe = countryName("FR", "de");
    expect(nameDe).toBe("Frankreich");

    const nameEs = countryName("DE", "es");
    expect(nameEs).toBe("Alemania");

    const nameIt = countryName("DE", "it");
    expect(nameIt).toBe("Germania");

    const nameTr = countryName("DE", "tr");
    expect(nameTr).toBe("Almanya");
  });

  it("falls back to English for unknown locale", () => {
    const name = countryName("IT", "xx");
    expect(name).toMatch(/Italy/);
  });

  it("returns empty string for empty input", () => {
    expect(countryName("")).toBe("");
  });
});

// ---------------------------------------------------------------------------
// allCountries
// ---------------------------------------------------------------------------

describe("allCountries", () => {
  it("returns an array of { code, name } objects", () => {
    const list = allCountries("en");
    expect(list.length).toBeGreaterThan(100);
    expect(list[0]).toHaveProperty("code");
    expect(list[0]).toHaveProperty("name");
  });

  it("is sorted by name", () => {
    const list = allCountries("en");
    for (let i = 1; i < list.length; i++) {
      expect(list[i].name.localeCompare(list[i - 1].name, "en")).toBeGreaterThanOrEqual(0);
    }
  });

  it("contains well-known countries", () => {
    const list = allCountries("en");
    const codes = list.map(c => c.code);
    expect(codes).toContain("US");
    expect(codes).toContain("GB");
    expect(codes).toContain("BR");
  });

  it("returns Italian country names when requested", () => {
    const list = allCountries("it");
    const germany = list.find((country) => country.code === "DE");

    expect(germany?.name).toBe("Germania");
  });

  it("returns Turkish country names when requested", () => {
    const list = allCountries("tr");
    const germany = list.find((country) => country.code === "DE");

    expect(germany?.name).toBe("Almanya");
  });
});

// ---------------------------------------------------------------------------
// isValidCountryCode
// ---------------------------------------------------------------------------

describe("isValidCountryCode", () => {
  it("returns true for valid alpha-2 codes", () => {
    expect(isValidCountryCode("US")).toBe(true);
    expect(isValidCountryCode("GB")).toBe(true);
    expect(isValidCountryCode("br")).toBe(true); // case-insensitive
  });

  it("returns false for invalid codes", () => {
    expect(isValidCountryCode("")).toBe(false);
    expect(isValidCountryCode("X")).toBe(false);
    expect(isValidCountryCode("ZZ")).toBe(false);
    expect(isValidCountryCode("USA")).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// normaliseNationality
// ---------------------------------------------------------------------------

describe("normaliseNationality", () => {
  it("returns alpha-2 code as-is if already valid", () => {
    expect(normaliseNationality("GB")).toBe("GB");
    expect(normaliseNationality("ES")).toBe("ES");
  });

  it("converts known demonyms to alpha-2 codes", () => {
    expect(normaliseNationality("English")).toBe("GB");
    expect(normaliseNationality("Spanish")).toBe("ES");
    expect(normaliseNationality("Brazilian")).toBe("BR");
    expect(normaliseNationality("German")).toBe("DE");
    expect(normaliseNationality("French")).toBe("FR");
  });

  it("returns the original value for unknown demonyms", () => {
    expect(normaliseNationality("Martian")).toBe("Martian");
  });

  it("returns empty string for empty input", () => {
    expect(normaliseNationality("")).toBe("");
  });
});
