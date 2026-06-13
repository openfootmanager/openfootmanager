import { describe, expect, it } from "vitest";
import { resolveLocalMediaPath } from "./mediaAssets";

describe("resolveLocalMediaPath", () => {
  it("normalizes relative local media paths", () => {
    expect(resolveLocalMediaPath("assets/worlds/demo/logo.png")).toBe(
      "/assets/worlds/demo/logo.png",
    );
  });

  it("keeps absolute local media paths", () => {
    expect(resolveLocalMediaPath("/assets/worlds/demo/logo.png")).toBe(
      "/assets/worlds/demo/logo.png",
    );
  });

  it("rejects URI schemes and protocol-relative URLs", () => {
    expect(resolveLocalMediaPath("https://example.com/logo.png")).toBeNull();
    expect(resolveLocalMediaPath("DATA:image/png;base64,abc")).toBeNull();
    expect(resolveLocalMediaPath("javascript:alert(1)")).toBeNull();
    expect(resolveLocalMediaPath("//example.com/logo.png")).toBeNull();
  });
});
