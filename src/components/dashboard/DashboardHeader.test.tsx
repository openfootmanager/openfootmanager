import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { MatchModeType } from "../../hooks/useAdvanceTime";
import type { PlayerData, TeamData } from "../../store/gameStore";
import DashboardHeader, { type DashboardMatchModeMeta } from "./DashboardHeader";

vi.mock("react-i18next", () => ({
    useTranslation: () => ({
        // Honours defaultValue, as the real t does. translatePositionAbbreviation
        // relies on it to fall back to the position code, so a mock that ignored
        // it would render raw translation keys as badge text.
        t: (key: string, options?: { defaultValue?: string }) => {
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
            return options?.defaultValue ?? key;
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

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
    return {
        id: "team-1",
        name: "Alpha FC",
        short_name: "ALP",
        country: "GB",
        city: "London",
        stadium_name: "Alpha Ground",
        stadium_capacity: 30000,
        finance: 500000,
        manager_id: "manager-1",
        reputation: 50,
        wage_budget: 50000,
        transfer_budget: 250000,
        season_income: 0,
        season_expenses: 0,
        formation: "4-4-2",
        play_style: "Balanced",
        training_focus: "General",
        training_intensity: "Balanced",
        training_schedule: "Balanced",
        founded_year: 1900,
        colors: { primary: "#000000", secondary: "#ffffff" },
        starting_xi_ids: [],
        form: [],
        history: [],
        ...overrides,
    };
}

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
        fireEvent.click(screen.getByRole("menuitem", { name: "View team" }));

        expect(onSelectSearchTeam).toHaveBeenCalledWith("team-1");
        expect(onSelectSearchPlayer).not.toHaveBeenCalled();
    });

    // Every position positionBadgeVariant names explicitly. The dashboard used to
    // keep its own four-bucket copy, so each granular position rendered red here
    // while the rest of the app rendered it blue or green.
    it.each([
        ["Goalkeeper", "GK", "bg-accent-100"],
        ["Defender", "DEF", "bg-primary-100"],
        ["RightBack", "RB", "bg-primary-100"],
        ["CenterBack", "CB", "bg-primary-100"],
        ["LeftBack", "LB", "bg-primary-100"],
        ["RightWingBack", "RWB", "bg-primary-100"],
        ["LeftWingBack", "LWB", "bg-primary-100"],
        ["Midfielder", "MID", "bg-green-100"],
        ["DefensiveMidfielder", "DM", "bg-green-100"],
        ["CentralMidfielder", "CM", "bg-green-100"],
        ["AttackingMidfielder", "AM", "bg-green-100"],
        ["RightMidfielder", "RM", "bg-green-100"],
        ["LeftMidfielder", "LM", "bg-green-100"],
        ["Forward", "FWD", "bg-red-100"],
        ["RightWinger", "RW", "bg-red-100"],
        ["LeftWinger", "LW", "bg-red-100"],
        ["Striker", "ST", "bg-red-100"],
    ])("badges a %s the same way the rest of the app does", (position, abbreviation, expectedClass) => {
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
                matchedPlayers={[createPlayer({ position })]}
                matchedTeams={teams}
                modeMeta={createModeMeta()}
                onBack={vi.fn()}
                onContinue={vi.fn()}
                onSave={vi.fn()}
                onSearchBlur={vi.fn()}
                onSearchFocus={vi.fn()}
                onSearchQueryChange={vi.fn()}
                onSelectMatchMode={vi.fn()}
                onSelectSearchPlayer={vi.fn()}
                onSelectSearchTeam={vi.fn()}
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

        expect(screen.getByText(abbreviation)).toHaveClass(expectedClass);
    });
});