import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import DashboardBlockerModal from "./DashboardBlockerModal";

vi.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string) => {
            if (key === "notifications.attentionRequired") return "Attention Required";
            if (key === "notifications.resolveBeforeContinuing") return "Resolve these issues before continuing";
            if (key === "notifications.goTo") return "Go to";
            if (key === "notifications.reviewIssues") return "Review Issues";
            if (key === "notifications.continueAnyway") return "Continue Anyway";
            if (key === "dashboard.squad") return "Squad Label";
            return key;
        },
    }),
}));

vi.mock("../../utils/backendI18n", () => ({
    resolveBackendText: (
        key?: string,
        fallback?: string,
        params?: Record<string, string>,
    ) => {
        if (key === "notifications.blockers.keyContractRisk") {
            return `Key contract risk: ${params?.players ?? ""}`.trim();
        }

        return fallback ?? "";
    },
}));

describe("DashboardBlockerModal", () => {
    it("renders translated blocker text and translated destination tab", () => {
        const onNavigate = vi.fn();

        render(
            <DashboardBlockerModal
                blockerModal={{
                    blockers: [
                        {
                            id: "key_contract_risk",
                            severity: "warn",
                            text: "Key player contract risk in squad planning: Barbosa",
                            text_key: "notifications.blockers.keyContractRisk",
                            text_params: { players: "Barbosa" },
                            tab: "Squad",
                        },
                    ],
                }}
                onClose={vi.fn()}
                onContinueAnyway={vi.fn()}
                onNavigate={onNavigate}
            />,
        );

        fireEvent.click(screen.getByRole("button", { name: /Key contract risk: Barbosa/i }));

        expect(screen.getByText("Key contract risk: Barbosa")).toBeInTheDocument();
        expect(screen.getByText("Go to Squad Label →")).toBeInTheDocument();
        expect(onNavigate).toHaveBeenCalledWith("Squad");
    });
});