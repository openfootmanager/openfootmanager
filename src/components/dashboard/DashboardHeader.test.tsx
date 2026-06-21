import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { MatchModeType } from "../../hooks/useAdvanceTime";
import type { PlayerData } from "../../store/gameStore";
import { createTeam } from "../../test-utils/factories";
import DashboardHeader, { type DashboardMatchModeMeta } from "./DashboardHeader";

vi.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string) => {
            if (key === "dashboard.noResults") return "No results";
            if (key === "dashboard.searchPlayers") return "Players";
            if (key === "dashboard.searchTeams") return "Teams";
            if (key === "dashboard.searchPlaceholder") return "Search";
            if (key === "dashboard.saveGame") return "Save game";
            if (key === "common.save") return "Save";
            if (key === "dashboard.continue") return "Continue";
            if (key === "common.back") return "Back";
            if (key === "squad.viewProfile") return "View profile";
            if (key === "common.viewTeam") return "View team";
            return key;
        },
    }),
}));

vi.mock("../ui", async (importOriginal) => {
    const actual = await importOriginal<typeof import("../ui")>();

    return {
        ...actual,
        ThemeToggle: () => <div data-testid="theme-toggle" />,
    };
});

function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
    return {
        id: "player-1",
        full_name: "John Striker",
        match_name: "J. Striker",
        team_id: "team-1",
        position: "Forward",
        natural_position: "Forward",
        date_of_birth: "2000-01-01",
        nationality: "GB",
        market_value: 1000000,
        condition: 100,
        ...overrides,
    } as PlayerData;
}

function createModeMeta(): Record<MatchModeType, DashboardMatchModeMeta> {
    return {
        live: {
            buttonColorClass: "from-primary-500 to-primary-600",
            desc: "Live",
            dropdownColorClass: "from-primary-600 to-primary-700",
            icon: <span>L</span>,
            label: "Live",
        },
        spectator: {
            buttonColorClass: "from-primary-500 to-primary-600",
            desc: "Spectator",
            dropdownColorClass: "from-primary-600 to-primary-700",
            icon: <span>S</span>,
            label: "Spectator",
        },
        delegate: {
            buttonColorClass: "from-primary-500 to-primary-600",
            desc: "Delegate",
            dropdownColorClass: "from-primary-600 to-primary-700",
            icon: <span>D</span>,
            label: "Delegate",
        },
    };
}

describe("DashboardHeader", () => {
    it("offers context menu actions for player search results", () => {
        const onSelectSearchPlayer = vi.fn();
        const onSelectSearchTeam = vi.fn();
        const teams = [createTeam()];

        render(
            <DashboardHeader
                activeTabLabel="Dashboard"
                currentDate="2026-08-10"
                hasProfileHistory={false}
                hasMatchToday={false}
                isAdvancing={false}
                isUnemployed={false}
                isSaving={false}
                matchMode="live"
                matchedPlayers={[createPlayer()]}
                matchedTeams={teams}
                modeMeta={createModeMeta()}
                onBack={vi.fn()}
                onContinue={vi.fn()}
                onSave={vi.fn()}
                onSearchBlur={vi.fn()}
                onSearchFocus={vi.fn()}
                onSearchQueryChange={vi.fn()}
                onSelectMatchMode={vi.fn()}
                onSelectSearchPlayer={onSelectSearchPlayer}
                onSelectSearchTeam={onSelectSearchTeam}
                onSkipToMatchDay={vi.fn()}
                onToggleContinueMenu={vi.fn()}
                saveFlash={false}
                searchOpen
                searchQuery="jo"
                seasonComplete={false}
                showContinueMenu={false}
                teams={teams}
            />,
        );

        fireEvent.contextMenu(screen.getByTestId("dashboard-search-player-player-1"));
        fireEvent.click(screen.getByRole("button", { name: "View team" }));

        expect(onSelectSearchTeam).toHaveBeenCalledWith("team-1");
        expect(onSelectSearchPlayer).not.toHaveBeenCalled();
    });
});