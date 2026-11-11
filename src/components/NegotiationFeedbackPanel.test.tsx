import { render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";

import { i18nReady } from "../i18n";
import { takeMissingKeys } from "../i18n/missingKeys";
import NegotiationFeedbackPanel, {
  type NegotiationFeedbackPanelData,
} from "./NegotiationFeedbackPanel";

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => undefined },
  useTranslation: () => ({ t: (key: string) => key }),
}));

beforeAll(async () => {
  await i18nReady;
});

const feedback = (
  overrides: Partial<NegotiationFeedbackPanelData>,
): NegotiationFeedbackPanelData => ({
  mood: "guarded",
  headline_key: "playerProfile.renewalFeedbackBlockedHeadline",
  detail_key: "playerProfile.renewalFeedbackBlockedDetail",
  tension: 40,
  patience: 60,
  round: 1,
  ...overrides,
});

const panel = (data: NegotiationFeedbackPanelData) => (
  <NegotiationFeedbackPanel
    feedback={data}
    titleKey="title"
    roundKey="round"
    patienceKey="patience"
    tensionKey="tension"
  />
);

describe("NegotiationFeedbackPanel", () => {
  /**
   * Given the backend's headline and detail keys, which the locale has
   * When the panel is drawn
   * Then the locale's words are shown.
   */
  it("shows the locale's words for the backend's keys", () => {
    render(panel(feedback({})));

    expect(screen.getByText("They are not reopening the conversation yet.")).toBeInTheDocument();
    expect(screen.queryByText(/playerProfile\./)).toBeNull();
  });

  /** Given a nested key as a headline parameter, when rendered, then both keys resolve. */
  it("resolves a nested headline parameter", () => {
    const { container } = render(
      panel(feedback({ headline_key: "menu.manager", params: { name: "common.cancel" } })),
    );
    expect(screen.getByText("Manager: Cancel")).toBeInTheDocument();
    expect(container.textContent).not.toContain("common.cancel");
  });

  /** Given a nested key as a detail parameter, when rendered, then both keys resolve. */
  it("resolves a nested detail parameter", () => {
    const { container } = render(
      panel(feedback({ detail_key: "menu.manager", params: { name: "common.cancel" } })),
    );
    expect(screen.getByText("Manager: Cancel")).toBeInTheDocument();
    expect(container.textContent).not.toContain("common.cancel");
  });

  /**
   * Given a headline and a detail key that no locale has
   * When the panel is drawn
   * Then neither raw key is shown (the resolver's empty fallback), and the gate sees both.
   */
  it("never draws a raw key the locale lacks", () => {
    takeMissingKeys();
    const { container } = render(
      panel(
        feedback({
          headline_key: "playerProfile.noSuchHeadline",
          detail_key: "playerProfile.noSuchDetail",
        }),
      ),
    );

    expect(container.textContent).not.toContain("playerProfile.noSuch");
    expect(takeMissingKeys()).toEqual([
      "playerProfile.noSuchHeadline",
      "playerProfile.noSuchDetail",
    ]);
  });
});
