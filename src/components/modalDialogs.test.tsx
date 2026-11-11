import { fireEvent, render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { describe, expect, it, vi } from "vitest";

import DashboardBlockerModal from "./dashboard/DashboardBlockerModal";
import DashboardCloseConfirmModal from "./dashboard/DashboardCloseConfirmModal";
import DashboardExitConfirmModal from "./dashboard/DashboardExitConfirmModal";
import DashboardExitSavingModal from "./dashboard/DashboardExitSavingModal";
import DashboardMatchConfirmModal from "./dashboard/DashboardMatchConfirmModal";
import DashboardSimulatingModal from "./dashboard/DashboardSimulatingModal";
import InboxDeleteConfirmModal from "./inbox/InboxDeleteConfirmModal";
import ProfileSaveConfirm from "./menu/ProfileSaveConfirm";
import PlayerProfileRenewalModal from "./playerProfile/PlayerProfileRenewalModal";
import PlayerProfileTerminationModal from "./playerProfile/PlayerProfileTerminationModal";
import SwitchClubConfirmModal from "./SwitchClubConfirmModal";

// jsdom has no scrollIntoView; the digest auto-scrolls its feed.
Element.prototype.scrollIntoView = vi.fn();

vi.mock("react-i18next", () => ({
  // backendI18n pulls in i18n/index, which registers the react plugin at import time.
  initReactI18next: { type: "3rdParty", init: () => undefined },
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "en" } }),
}));

const t = (key: string) => key;

/**
 * Every modal the game draws over a page, and what Escape does in it. Each is a ui/Dialog: a named
 * dialog that takes the focus when it opens, so a keyboard player is in it rather than behind it.
 */
const MODALS: Array<{
  modal: string;
  /** The dialog's accessible name: its own heading. */
  name: string;
  /** Escape asks for this handler; null when the modal can't be closed (busy). */
  render: (onEscape: () => void) => ReactElement;
  closes: boolean;
}> = [
  {
    modal: "the Exit confirmation",
    name: "exitConfirm.title",
    render: (onEscape) => <DashboardExitConfirmModal onCancel={onEscape} onConfirm={vi.fn()} />,
    closes: true,
  },
  {
    modal: "the window close confirmation",
    name: "closeConfirm.title",
    render: (onEscape) => (
      <DashboardCloseConfirmModal
        onCancel={onEscape}
        onQuitWithoutSave={vi.fn()}
        onSaveAndQuit={vi.fn()}
      />
    ),
    closes: true,
  },
  {
    modal: "the match-day confirmation",
    name: "continueMenu.matchDayTitle",
    render: (onEscape) => (
      <DashboardMatchConfirmModal
        matchMode="live"
        modeMeta={{ desc: "Watch it live", icon: null, label: "Live" }}
        onCancel={onEscape}
        onConfirm={vi.fn()}
        teams={[]}
        todayMatchFixture={null}
      />
    ),
    closes: true,
  },
  {
    modal: "the blocker list",
    name: "notifications.attentionRequired",
    render: (onEscape) => (
      <DashboardBlockerModal
        blockerModal={{
          blockers: [{ id: "b1", severity: "warn", text: "Set a lineup", tab: "Squad" }],
        }}
        onClose={onEscape}
        onContinueAnyway={null}
        onNavigate={vi.fn()}
      />
    ),
    closes: true,
  },
  {
    modal: "the inbox delete confirmation",
    name: "inbox.deleteMessageTitle",
    render: (onEscape) => (
      <InboxDeleteConfirmModal
        deleteModalState={{ mode: "single", messageId: "m1", subject: "Board meeting" }}
        isDeleting={false}
        onCancel={onEscape}
        onConfirm={vi.fn()}
      />
    ),
    closes: true,
  },
  {
    modal: "the manager profile save confirmation",
    name: "managerProfiles.saveConfirm.title",
    render: (onEscape) => (
      <ProfileSaveConfirm
        loadedProfile={{
          id: "p1",
          first_name: "Alex",
          last_name: "Morgan",
          date_of_birth: "1980-04-12",
          nationality: "Scotland",
          created_at: "2026-01-01",
          last_used_at: null,
        }}
        onUpdate={vi.fn()}
        onSaveNew={vi.fn()}
        onSkip={vi.fn()}
        onClose={onEscape}
      />
    ),
    closes: true,
  },
  {
    modal: "the contract renewal",
    name: "playerProfile.renewalTitle",
    render: (onEscape) => (
      <PlayerProfileRenewalModal
        show
        playerName="Anderson"
        t={t}
        renewalWage="1200"
        renewalLength="2"
        renewalIsTerminal={false}
        isRenewalWageValid
        renewalViolatesSoftCap={false}
        renewalProjection={null}
        renewalStatusMessage={null}
        renewalStatusClassName=""
        renewalCooledOff={false}
        renewalFeedback={null}
        renewalSubmitDisabled={false}
        delegateRenewalDisabled={false}
        onWageChange={vi.fn()}
        onLengthChange={vi.fn()}
        onClose={onEscape}
        onDelegate={vi.fn()}
        onSubmit={vi.fn()}
      />
    ),
    closes: true,
  },
  {
    modal: "the contract termination",
    name: "playerProfile.terminateContractTitle",
    render: (onEscape) => (
      <PlayerProfileTerminationModal
        show
        playerName="Anderson"
        t={t}
        preview={null}
        errorMessage={null}
        submitting={false}
        onCancel={onEscape}
        onConfirm={vi.fn()}
      />
    ),
    closes: true,
  },
  {
    modal: "the club switch confirmation",
    name: "jobs.switchConfirmTitle",
    render: (onEscape) => (
      <SwitchClubConfirmModal
        open
        currentClubName="Glasgow FC"
        newClubName="Aberdeen City"
        onCancel={onEscape}
        onConfirm={vi.fn()}
      />
    ),
    closes: true,
  },
  {
    modal: "a finished advance's digest",
    name: "dashboard.digestDone",
    render: (onEscape) => (
      <DashboardSimulatingModal digestEntries={[]} isDigestRunning={false} onDismiss={onEscape} />
    ),
    closes: true,
  },
  {
    modal: "a running advance's digest",
    name: "dashboard.digestAdvancing",
    render: (onEscape) => (
      <DashboardSimulatingModal
        digestEntries={[]}
        isDigestRunning
        onStop={vi.fn()}
        onDismiss={onEscape}
      />
    ),
    closes: false,
  },
  {
    modal: "the single-day advance spinner",
    name: "dashboard.simulating",
    render: () => <DashboardSimulatingModal />,
    closes: false,
  },
  {
    modal: "the save-and-exit spinner",
    name: "exitConfirm.savingTitle",
    render: () => <DashboardExitSavingModal />,
    closes: false,
  },
  // A confirmation whose action is running disables Cancel; Escape mustn't cancel it either.
  {
    modal: "the inbox delete confirmation while it deletes",
    name: "inbox.deleteMessageTitle",
    render: (onEscape) => (
      <InboxDeleteConfirmModal
        deleteModalState={{ mode: "single", messageId: "m1", subject: "Board meeting" }}
        isDeleting
        onCancel={onEscape}
        onConfirm={vi.fn()}
      />
    ),
    closes: false,
  },
  {
    modal: "the club switch confirmation while it switches",
    name: "jobs.switchConfirmTitle",
    render: (onEscape) => (
      <SwitchClubConfirmModal
        open
        busy
        currentClubName="Glasgow FC"
        newClubName="Aberdeen City"
        onCancel={onEscape}
        onConfirm={vi.fn()}
      />
    ),
    closes: false,
  },
  {
    modal: "the contract termination while it submits",
    name: "playerProfile.terminateContractTitle",
    render: (onEscape) => (
      <PlayerProfileTerminationModal
        show
        playerName="Anderson"
        t={t}
        preview={null}
        errorMessage={null}
        submitting
        onCancel={onEscape}
        onConfirm={vi.fn()}
      />
    ),
    closes: false,
  },
];

describe("the game's modals", () => {
  /**
   * Given a modal opened over a page
   * When it appears, and the player presses Escape
   * Then it is a dialog named by its heading with the focus inside it (not left on the button
   * behind it), and Escape closes it, or, for one that can't close yet, does nothing and says it is
   * busy.
   */
  it.each(MODALS)(
    "draws $modal as a named dialog that takes the focus",
    ({ name, render: draw, closes }) => {
      const onEscape = vi.fn();
      render(draw(onEscape));

      const dialog = screen.getByRole("dialog", { name });
      expect(dialog).toHaveAttribute("aria-modal", "true");
      expect(dialog).toContainElement(document.activeElement as HTMLElement);

      fireEvent.keyDown(window, { key: "Escape" });
      if (closes) {
        expect(onEscape).toHaveBeenCalledOnce();
      } else {
        expect(onEscape).not.toHaveBeenCalled();
        expect(dialog).toHaveAttribute("aria-busy", "true");
      }
    },
  );
});
