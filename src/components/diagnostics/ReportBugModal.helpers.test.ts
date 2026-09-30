import { describe, expect, it } from "vitest";

import type { GameStateData } from "../../store/types";
import {
  EMPTY_DRAFT,
  composeReportText,
  describeCareer,
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
    frequencyLabel: "How often does it happen",
    frequency: "Every time",
    career: "Your career",
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
    // The question as a heading, the answer underneath — not a heading that IS the answer.
    expect(out).toContain("## How often does it happen\n\nEvery time");
  });

  it("leaves out a section the player skipped", () => {
    const out = composeReportText({ ...filled, steps: "   " }, LABELS);

    expect(out).not.toContain("## Steps to reproduce");
    expect(out).toContain("## What happened");
  });

  it("carries the career line when there is one", () => {
    const out = composeReportText(filled, LABELS, "Boca Juniors · Primera 2026");

    expect(out).toContain("## Your career");
    expect(out).toContain("Boca Juniors · Primera 2026");
  });

  it("leaves the career heading out when no career is open", () => {
    expect(composeReportText(filled, LABELS)).not.toContain("## Your career");
  });
});

describe("describeCareer", () => {
  const gameState = {
    clock: { current_date: "2026-03-14", start_date: "2025-07-01" },
    manager: { team_id: "t1" },
    teams: [
      { id: "t1", name: "Boca Juniors" },
      { id: "t2", name: "River Plate" },
    ],
    league: { id: "l1", name: "Primera División", season: 2026 },
    package_lockfile: [{ id: "argentina-1962", version: "1.2.0", hash: "abc" }],
  } as unknown as GameStateData;

  it("names where the player was when it broke", () => {
    // What a triager needs to reproduce: the club, the competition, the in-game date, and above
    // all the packages the world was built from.
    expect(describeCareer(gameState)).toBe(
      "Boca Juniors · Primera División 2026 · 2026-03-14 · packages: argentina-1962@1.2.0",
    );
  });

  it("never names the manager", () => {
    // The manager's name is usually the player's own, and this line goes to a public issue.
    const named = {
      ...gameState,
      manager: { team_id: "t1", first_name: "Alice", last_name: "Sørensen" },
    } as unknown as GameStateData;

    expect(describeCareer(named)).not.toContain("Alice");
  });

  it("is empty with no career open", () => {
    expect(describeCareer(null)).toBe("");
  });

  it("drops the parts a world does not have", () => {
    const sparse = {
      clock: { current_date: "2026-03-14" },
      manager: { team_id: null },
      teams: [],
      league: null,
    } as unknown as GameStateData;

    expect(describeCareer(sparse)).toBe("2026-03-14");
  });
});
