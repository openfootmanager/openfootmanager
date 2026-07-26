import { afterEach, describe, expect, it } from "vitest";

import { isAndroid } from "./platform";

const originalUserAgent = navigator.userAgent;

function setUserAgent(userAgent: string): void {
  Object.defineProperty(window.navigator, "userAgent", {
    value: userAgent,
    configurable: true,
  });
}

afterEach(() => {
  setUserAgent(originalUserAgent);
});

describe("isAndroid", () => {
  it("returns true for Android WebView user agents", () => {
    setUserAgent(
      "Mozilla/5.0 (Linux; Android 14; Pixel 8 Build/UP1A.231005.007; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/120.0.0.0 Mobile Safari/537.36",
    );

    expect(isAndroid()).toBe(true);
  });

  it("matches android case-insensitively", () => {
    setUserAgent("Some ANDROID Tablet Agent");

    expect(isAndroid()).toBe(true);
  });

  it("returns false for desktop Linux user agents", () => {
    setUserAgent(
      "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    );

    expect(isAndroid()).toBe(false);
  });

  it("returns false for desktop Windows user agents", () => {
    setUserAgent(
      "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    );

    expect(isAndroid()).toBe(false);
  });
});
