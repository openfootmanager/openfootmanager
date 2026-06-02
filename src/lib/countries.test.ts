import { describe, it, expect } from "vitest";
import {
  countryName,
  allCountries,
  allNationalities,
  isValidCountryCode,
  normaliseNationality,
  resolveCountryFlagCode,
} from "./countries";

// ---------------------------------------------------------------------------
// resolveCountryFlagCode
// ---------------------------------------------------------------------------

describe("resolveCountryFlagCode", () => {
  it("returns a valid code when an SVG flag is available", () => {
    expect(resolveCountryFlagCode("US")).toBe("US");
    expect(resolveCountryFlagCode("GB")).toBe("GB");
    expect(resolveCountryFlagCode("br")).toBe("BR");
    expect(resolveCountryFlagCode("IE")).toBe("IE");
    expect(resolveCountryFlagCode("ENG")).toBe("GB-ENG");
    expect(resolveCountryFlagCode("SCO")).toBe("GB-SCT");
    expect(resolveCountryFlagCode("WAL")).toBe("GB-WLS");
    expect(resolveCountryFlagCode("NIR")).toBe("GB-NIR");
  });

  it("normalises demonym values before resolving", () => {
    expect(resolveCountryFlagCode("English")).toBe("GB-ENG");
    expect(resolveCountryFlagCode("Brazilian")).toBe("BR");
    expect(resolveCountryFlagCode("Irish")).toBe("IE");
  });

  it("returns null for invalid values", () => {
    expect(resolveCountryFlagCode("")).toBeNull();
    expect(resolveCountryFlagCode("X")).toBeNull();
    expect(resolveCountryFlagCode("USA")).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// countryName
// ---------------------------------------------------------------------------

describe("countryName", () => {
  it("returns English country name by default", () => {
    expect(countryName("GB")).toMatch(/United Kingdom/);
    expect(countryName("US")).toMatch(/United States/);
    expect(countryName("ENG")).toBe("England");
  });

  it("returns localised name for supported locales", () => {
    const nameDe = countryName("FR", "de");
    expect(nameDe).toBe("Frankreich");

    const nameEs = countryName("DE", "es");
    expect(nameEs).toBe("Alemania");

    const nameIt = countryName("DE", "it");
    expect(nameIt).toBe("Germania");

    const nameRu = countryName("DE", "ru");
    expect(nameRu).toBe("Германия");

    const nameZh = countryName("DE", "zh-CN");
    expect(nameZh).toBe("德国");

    const englandEs = countryName("ENG", "es");
    expect(englandEs).toBe("Inglaterra");

    const englandRu = countryName("ENG", "ru");
    expect(englandRu).toBe("Англия");

    const englandZh = countryName("ENG", "zh-CN");
    expect(englandZh).toBe("英格兰");
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
});

describe("allNationalities", () => {
  it("surfaces football nations and excludes legacy GB from the selectable list", () => {
    const list = allNationalities("en");
    const codes = list.map((country) => country.code);

    expect(codes).toContain("ENG");
    expect(codes).toContain("SCO");
    expect(codes).toContain("WAL");
    expect(codes).toContain("NIR");
    expect(codes).toContain("IE");
    expect(codes).not.toContain("GB");
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
    expect(isValidCountryCode("ENG")).toBe(true);
    expect(isValidCountryCode("NIR")).toBe(true);
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
    expect(normaliseNationality("ENG")).toBe("ENG");
  });

  it("converts known demonyms to alpha-2 codes", () => {
    expect(normaliseNationality("English")).toBe("ENG");
    expect(normaliseNationality("Scottish")).toBe("SCO");
    expect(normaliseNationality("Welsh")).toBe("WAL");
    expect(normaliseNationality("Irish")).toBe("IE");
    expect(normaliseNationality("Northern Irish")).toBe("NIR");
    expect(normaliseNationality("Spanish")).toBe("ES");
    expect(normaliseNationality("Brazilian")).toBe("BR");
    expect(normaliseNationality("German")).toBe("DE");
    expect(normaliseNationality("French")).toBe("FR");
  });

  it("converts recognised country names to alpha-2 codes", () => {
    expect(normaliseNationality("Spain")).toBe("ES");
    expect(normaliseNationality("Germany")).toBe("DE");
    expect(normaliseNationality("Italy")).toBe("IT");
    expect(normaliseNationality("Netherlands")).toBe("NL");
  });

  it("returns the original value for unknown demonyms", () => {
    expect(normaliseNationality("Martian")).toBe("Martian");
  });

  it("returns empty string for empty input", () => {
    expect(normaliseNationality("")).toBe("");
  });
});
