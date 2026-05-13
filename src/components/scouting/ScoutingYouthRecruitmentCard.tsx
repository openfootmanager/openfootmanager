import { GraduationCap, RefreshCcw, ScanSearch, Clock, XCircle } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { StaffData, YouthScoutingAssignment } from "../../store/gameStore";
import { Badge, Button, Card, CardBody, CardHeader, Select } from "../ui";

interface ScoutingYouthRecruitmentCardProps {
    title?: string;
    hint?: string;
    youthAssignments: YouthScoutingAssignment[];
    scouts: StaffData[];
    availableScouts: StaffData[];
    isStarting: boolean;
    selectedScoutId: string;
    region: string;
    objective: string;
    targetPosition: string;
    errorMessage?: string | null;
    onScoutChange: (value: string) => void;
    onRegionChange: (value: string) => void;
    onObjectiveChange: (value: string) => void;
    onTargetPositionChange: (value: string) => void;
    onStartSearch: () => void;
    onCancelSearch: (assignmentId: string) => void;
    onReassignSearch: (assignmentId: string, scoutId: string) => void;
}

export default function ScoutingYouthRecruitmentCard({
    title,
    hint,
    youthAssignments,
    scouts,
    availableScouts,
    isStarting,
    selectedScoutId,
    region,
    objective,
    targetPosition,
    errorMessage,
    onScoutChange,
    onRegionChange,
    onObjectiveChange,
    onTargetPositionChange,
    onStartSearch,
    onCancelSearch,
    onReassignSearch,
}: ScoutingYouthRecruitmentCardProps) {
    const { t } = useTranslation();
    const [reassignTargets, setReassignTargets] = useState<Record<string, string>>({});
    const availableScoutCount = availableScouts.length;
    const canStart = availableScoutCount > 0 && selectedScoutId.length > 0 && !isStarting;

    function formatRegion(value?: string): string {
        if (value === "International") return t("scouting.regionInternational");
        return t("scouting.regionDomestic");
    }

    function formatObjective(value?: string): string {
        if (value === "HighPotential") return t("scouting.objectiveHighPotential");
        if (value === "ReadySoon") return t("scouting.objectiveReadySoon");
        return t("scouting.objectiveBalanced");
    }

    function getReassignChoices(assignment: YouthScoutingAssignment): StaffData[] {
        return scouts.filter(
            (scout) =>
                scout.id !== assignment.scout_id &&
                availableScouts.some((candidate) => candidate.id === scout.id),
        );
    }

    return (
        <Card accent="primary">
            <CardHeader
                action={
                    <Button
                        size="sm"
                        icon={<ScanSearch />}
                        disabled={!canStart}
                        onClick={onStartSearch}
                    >
                        {t("scouting.startYouthSearch")}
                    </Button>
                }
            >
                {title ?? t("scouting.youthRecruitment")}
            </CardHeader>
            <CardBody className="flex flex-col gap-4">
                <p className="text-sm text-gray-500 dark:text-gray-400">
                    {hint ?? t("scouting.youthRecruitmentHint")}
                </p>

                <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-3">
                    <div className="flex flex-col gap-1.5">
                        <span className="text-xs font-heading uppercase tracking-wider text-gray-500 dark:text-gray-400">
                            {t("scouting.youthSearchScoutLabel")}
                        </span>
                        <Select
                            selectSize="sm"
                            value={selectedScoutId}
                            aria-label={t("scouting.youthSearchScoutLabel")}
                            onChange={(event) => onScoutChange(event.target.value)}
                        >
                            <option value="">{t("scouting.selectScout")}</option>
                            {availableScouts.map((scout) => (
                                <option key={scout.id} value={scout.id}>
                                    {scout.first_name} {scout.last_name}
                                </option>
                            ))}
                        </Select>
                    </div>

                    <div className="flex flex-col gap-1.5">
                        <span className="text-xs font-heading uppercase tracking-wider text-gray-500 dark:text-gray-400">
                            {t("scouting.youthSearchRegionLabel")}
                        </span>
                        <Select
                            selectSize="sm"
                            value={region}
                            aria-label={t("scouting.youthSearchRegionLabel")}
                            onChange={(event) => onRegionChange(event.target.value)}
                        >
                            <option value="Domestic">{t("scouting.regionDomestic")}</option>
                            <option value="International">{t("scouting.regionInternational")}</option>
                        </Select>
                    </div>

                    <div className="flex flex-col gap-1.5">
                        <span className="text-xs font-heading uppercase tracking-wider text-gray-500 dark:text-gray-400">
                            {t("scouting.youthSearchObjectiveLabel")}
                        </span>
                        <Select
                            selectSize="sm"
                            value={objective}
                            aria-label={t("scouting.youthSearchObjectiveLabel")}
                            onChange={(event) => onObjectiveChange(event.target.value)}
                        >
                            <option value="Balanced">{t("scouting.objectiveBalanced")}</option>
                            <option value="HighPotential">{t("scouting.objectiveHighPotential")}</option>
                            <option value="ReadySoon">{t("scouting.objectiveReadySoon")}</option>
                        </Select>
                    </div>

                    <div className="flex flex-col gap-1.5">
                        <span className="text-xs font-heading uppercase tracking-wider text-gray-500 dark:text-gray-400">
                            {t("scouting.youthTargetLabel")}
                        </span>
                        <Select
                            selectSize="sm"
                            value={targetPosition}
                            aria-label={t("scouting.youthTargetLabel")}
                            onChange={(event) => onTargetPositionChange(event.target.value)}
                        >
                            <option value="">{t("scouting.youthAnyPosition")}</option>
                            <option value="Defender">{t("common.positions.Defender")}</option>
                            <option value="Midfielder">{t("common.positions.Midfielder")}</option>
                            <option value="Forward">{t("common.positions.Forward")}</option>
                        </Select>
                    </div>
                </div>

                <div className="flex flex-wrap items-center gap-2">
                    <Badge variant="primary" size="sm">
                        {t("scouting.activeYouthSearches", { count: youthAssignments.length })}
                    </Badge>
                    {availableScoutCount === 0 ? (
                        <span className="text-xs text-gray-500 dark:text-gray-400">
                            {t("scouting.noScoutsFree")}
                        </span>
                    ) : null}
                    {errorMessage ? (
                        <span className="text-xs text-red-500">{errorMessage}</span>
                    ) : null}
                </div>

                {youthAssignments.length === 0 ? (
                    <div className="flex items-center gap-3 rounded-xl border border-dashed border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-800/40 px-4 py-4">
                        <GraduationCap className="w-5 h-5 text-primary-500 shrink-0" />
                        <p className="text-sm text-gray-500 dark:text-gray-400">
                            {t("scouting.noYouthSearches")}
                        </p>
                    </div>
                ) : (
                    <div className="flex flex-col gap-3">
                        {youthAssignments.map((assignment) => {
                            const scout = scouts.find((staffMember) => staffMember.id === assignment.scout_id);
                            if (!scout) {
                                return null;
                            }

                            const reassignChoices = getReassignChoices(assignment);
                            const reassignTarget = reassignTargets[assignment.id] ?? reassignChoices[0]?.id ?? "";

                            return (
                                <div
                                    key={assignment.id}
                                    className="rounded-xl border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-800/60 px-4 py-3"
                                >
                                    <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
                                        <div className="min-w-0">
                                            <p className="font-heading font-bold text-sm text-gray-800 dark:text-gray-100">
                                                {t("scouting.youthProspectSearch")}
                                            </p>
                                            <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                                                {t("scouting.scoutLabel", {
                                                    name: `${scout.first_name} ${scout.last_name}`,
                                                })}
                                            </p>
                                            <div className="mt-2 flex flex-wrap gap-2">
                                                <Badge variant="neutral" size="sm">
                                                    {formatRegion(assignment.region)}
                                                </Badge>
                                                <Badge variant="neutral" size="sm">
                                                    {formatObjective(assignment.objective)}
                                                </Badge>
                                                <Badge variant="neutral" size="sm">
                                                    {assignment.target_position
                                                        ? t(`common.positions.${assignment.target_position}`)
                                                        : t("scouting.youthAnyPosition")}
                                                </Badge>
                                            </div>
                                        </div>

                                        <div className="flex items-center gap-1.5 text-accent-500 shrink-0">
                                            <Clock className="w-3.5 h-3.5" />
                                            <span className="text-xs font-heading font-bold">
                                                {t("scouting.daysLeft", { days: assignment.days_remaining })}
                                            </span>
                                        </div>
                                    </div>

                                    <div className="mt-3 flex flex-col gap-2 lg:flex-row lg:items-center lg:justify-between">
                                        <div className="flex items-center gap-2">
                                            <Button
                                                size="sm"
                                                variant="outline"
                                                icon={<XCircle className="w-4 h-4" />}
                                                onClick={() => onCancelSearch(assignment.id)}
                                            >
                                                {t("scouting.cancelSearch")}
                                            </Button>
                                        </div>

                                        <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
                                            <Select
                                                selectSize="sm"
                                                value={reassignTarget}
                                                aria-label={`${t("scouting.reassignSearch")} ${assignment.id}`}
                                                onChange={(event) =>
                                                    setReassignTargets((current) => ({
                                                        ...current,
                                                        [assignment.id]: event.target.value,
                                                    }))
                                                }
                                                disabled={reassignChoices.length === 0}
                                            >
                                                {reassignChoices.length === 0 ? (
                                                    <option value="">{t("scouting.noAlternateScout")}</option>
                                                ) : (
                                                    reassignChoices.map((candidate) => (
                                                        <option key={candidate.id} value={candidate.id}>
                                                            {candidate.first_name} {candidate.last_name}
                                                        </option>
                                                    ))
                                                )}
                                            </Select>
                                            <Button
                                                size="sm"
                                                variant="outline"
                                                icon={<RefreshCcw className="w-4 h-4" />}
                                                disabled={reassignChoices.length === 0 || reassignTarget.length === 0}
                                                onClick={() => onReassignSearch(assignment.id, reassignTarget)}
                                            >
                                                {t("scouting.reassignSearch")}
                                            </Button>
                                        </div>
                                    </div>
                                </div>
                            );
                        })}
                    </div>
                )}
            </CardBody>
        </Card>
    );
}