import { describe, expect, it } from "vitest";

import { isPackageQualifiedAsset, joinAssetRoot } from "./packageAssets";

describe("isPackageQualifiedAsset", () => {
  it("recognises a path stamped with its owning package", () => {
    expect(isPackageQualifiedAsset("brazil-1962/assets/images/santos.png")).toBe(true);
  });

  it("leaves bundled app assets alone", () => {
    // These ship inside the app and are served from the frontend root; routing
    // them through the asset protocol would break them.
    expect(isPackageQualifiedAsset("/assets/worlds/demo/logo.png")).toBe(false);
    expect(isPackageQualifiedAsset("assets/worlds/demo/logo.png")).toBe(false);
  });

  it("ignores absolute paths and URLs", () => {
    expect(isPackageQualifiedAsset("https://example.com/a.png")).toBe(false);
    expect(isPackageQualifiedAsset("data:image/png;base64,AAAA")).toBe(false);
    expect(isPackageQualifiedAsset("/home/user/a.png")).toBe(false);
  });

  it("treats empty input as not package-qualified", () => {
    expect(isPackageQualifiedAsset("")).toBe(false);
    expect(isPackageQualifiedAsset(null)).toBe(false);
    expect(isPackageQualifiedAsset(undefined)).toBe(false);
  });
});

describe("joinAssetRoot", () => {
  it("joins without doubling separators", () => {
    expect(joinAssetRoot("/home/u/.local/share/app", "brazil-1962/assets/a.png")).toBe(
      "/home/u/.local/share/app/package-assets/brazil-1962/assets/a.png",
    );
  });

  it("tolerates a trailing separator on the data dir", () => {
    // appDataDir() returns a trailing slash on some platforms and not others.
    expect(joinAssetRoot("/home/u/app/", "pkg/assets/a.png")).toBe(
      "/home/u/app/package-assets/pkg/assets/a.png",
    );
  });

  it("keeps Windows separators usable", () => {
    expect(joinAssetRoot("C:\\Users\\u\\AppData\\app", "pkg/assets/a.png")).toBe(
      "C:\\Users\\u\\AppData\\app/package-assets/pkg/assets/a.png",
    );
  });
});
