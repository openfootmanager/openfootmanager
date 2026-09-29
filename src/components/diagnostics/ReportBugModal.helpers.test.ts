import { describe, expect, it } from "vitest";

import {
  EMPTY_DRAFT,
  composeReportText,
  describeBundle,
  describeContext,
  describeMachine,
  describeResolution,
  formatBytes,
  missingRequiredFields,
} from "./ReportBugModal.helpers";

describe("missingRequiredFields", () => {
  it("names both required fields when the form is untouched", () => {
    expect(missingRequiredFields(EMPTY_DRAFT)).toEqual(["whatHappened", "expected"]);
  });

  it("treats whitespace as blank", () => {
    expect(missingRequiredFields({ ...EMPTY_DRAFT, whatHappened: "   " })).toContain(
      "whatHappened",
    );
  });

  it("does not require the optional steps field", () => {
    const filled = {
      ...EMPTY_DRAFT,
      whatHappened: "froze",
      expected: "should start",
    };

    expect(missingRequiredFields(filled)).toEqual([]);
  });
});

describe("formatBytes", () => {
  it("keeps small sizes in bytes", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(999)).toBe("999 B");
  });

  it("shows one decimal below ten so a size does not round away", () => {
    // "1 MB" for a 1.4 MB bundle understates it on the one screen that promises full disclosure.
    expect(formatBytes(1_468_006)).toBe("1.4 MB");
  });

  it("drops the decimal above ten, where it is noise", () => {
    expect(formatBytes(24 * 1024 * 1024)).toBe("24 MB");
  });

  it("steps up through the units", () => {
    expect(formatBytes(2048)).toBe("2.0 KB");
    expect(formatBytes(3 * 1024 ** 3)).toBe("3.0 GB");
  });

  it("does not invent a size it was not given", () => {
    expect(formatBytes(Number.NaN)).toBe("—");
    expect(formatBytes(-1)).toBe("—");
  });
});

describe("describeMachine", () => {
  const diagnostics = {
    app_version: "0.3.0",
    os: "linux",
    arch: "x86_64",
    webview_version: "2.50.1",
    log_directory: "~/.local/share/ofm/logs",
    crash_on_previous_run: false,
    has_active_save: false,
    log_files: [],
    save_bytes: null,
  };

  it("names the platform and the webview", () => {
    expect(describeMachine(diagnostics)).toBe("linux x86_64 · webview 2.50.1");
  });

  it("is empty until the backend has answered", () => {
    expect(describeMachine(null)).toBe("");
  });
});

describe("describeResolution", () => {
  it("formats a screen size", () => {
    expect(describeResolution({ width: 2560, height: 1440 })).toBe("2560x1440");
  });

  it("returns nothing rather than a nonsense size", () => {
    expect(describeResolution({ width: 0, height: 0 })).toBe("");
  });
});

describe("describeContext", () => {
  it("joins the career line and the frequency", () => {
    const out = describeContext("Every time", "Matchday 5, Wanderers FC");

    expect(out).toBe("Matchday 5, Wanderers FC\nEvery time");
  });

  it("skips a career line when no game is open", () => {
    expect(describeContext("Every time", "")).toBe("Every time");
  });
});

describe("describeBundle", () => {
  it("names the file and its size", () => {
    const out = describeBundle({
      path: "/home/x/ofm-report.zip",
      bytes: 1_468_006,
      log_files: ["a.log"],
      included_save: false,
      included_crash: false,
    });

    expect(out).toBe("/home/x/ofm-report.zip (1.4 MB)");
  });
});

describe("composeReportText", () => {
  const LABELS = {
    whatHappened: "What happened",
    expected: "What did you expect",
    steps: "Steps to reproduce",
    frequency: "Every time",
  };

  const filled = {
    ...EMPTY_DRAFT,
    whatHappened: "It froze",
    expected: "It should have started",
    steps: "1. Advance",
  };

  it("keeps everything the player wrote, at full length", () => {
    // This file is the copy that survives the URL being trimmed, so it must never be the one that
    // does the trimming.
    const long = "x".repeat(20_000);

    const out = composeReportText({ ...filled, whatHappened: long }, LABELS);

    expect(out).toContain(long);
  });

  it("labels each section so the file reads on its own", () => {
    const out = composeReportText(filled, LABELS);

    expect(out).toContain("## What happened");
    expect(out).toContain("It froze");
    expect(out).toContain("## Steps to reproduce");
    expect(out).toContain("Every time");
  });

  it("leaves out a section the player skipped", () => {
    const out = composeReportText({ ...filled, steps: "   " }, LABELS);

    expect(out).not.toContain("## Steps to reproduce");
    expect(out).toContain("## What happened");
  });
});
