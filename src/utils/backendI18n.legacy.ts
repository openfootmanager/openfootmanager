import type { MessageAction, MessageData, NewsArticle } from "../store/gameStore";

const LEGACY_DELEGATED_RENEWALS_PREFIX = "delegated_renewals_";
const LEGACY_TAKEOVER_CONTRACT_REVIEW_PREFIX = "contract_review_takeover_";
const LEGACY_TAKEOVER_CONTRACT_REVIEW_SUBJECT = "Assistant Manager - Contract Review";
const LEGACY_DELEGATED_RENEWALS_SUMMARY_RE =
    /^Boss, I went through our renewal list at (?<team>.+)\. (?<successes>\d+) completed, (?<stalled>\d+) still pending, (?<failures>\d+) failed\.$/;
const LEGACY_DELEGATED_RENEWALS_SUCCESS_RE =
    /^Completed: (?<player>.+) agreed to (?<years>\d+) year\(s\) on €(?<wage>\d+)\/wk\.$/;
const LEGACY_DELEGATED_RENEWALS_STATUS_RE =
    /^(?<status>Still difficult|Failed): (?<player>.+) — (?<detail>.+)$/;
const LEGACY_DELEGATED_RENEWALS_BEYOND_LIMITS_RE =
    /^Their camp want around €(?<wage>\d+)\/wk for (?<years>\d+) years, which is beyond the delegation limits\.$/;
const LEGACY_DELEGATED_RENEWALS_PREFERS_MANAGER_RE =
    /^They would listen, but they still want about €(?<wage>\d+)\/wk for (?<years>\d+) years and prefer to hear from you directly\.$/;
const LEGACY_DELEGATED_RENEWALS_MANAGER_BLOCKED_RE =
    /^You told me not to reopen contract talks yet\.$/;
const LEGACY_DELEGATED_RENEWALS_RELATIONSHIP_BLOCKED_RE =
    /^They are not willing to commit through me under the current relationship and contract situation\.$/;
const LEGACY_WEEKLY_DIGEST_WEEK_LABEL_RE =
    /^Week of (?<weekStart>\d{4}-\d{2}-\d{2})$/;
const LEGACY_WEEKLY_DIGEST_HEADLINE_RE =
    /^Weekly Digest — Week of (?<weekStart>\d{4}-\d{2}-\d{2})$/;
const LEGACY_TAKEOVER_CONTRACT_REVIEW_NONE_URGENT_RE =
    /^Your assistant has reviewed the squad contracts after your arrival\. (?<totalExpiring>\d+) player\(s\) are due to come up for renewal this season, but none require an immediate decision today\.(?:\r?\n){2}Start mapping out who you want to keep so the situation stays under control\.$/;
const LEGACY_TAKEOVER_CONTRACT_REVIEW_URGENT_RE =
    /^Your assistant has reviewed the squad contracts after your arrival\. (?<totalExpiring>\d+) player\(s\) are due to come up for renewal this season, and (?<urgentContracts>\d+) already need attention inside the next six months\.(?:\r?\n){2}You do not need to solve everything at once, but these cases should be near the top of your early workload\.$/;
const LEGACY_TAKEOVER_CONTRACT_REVIEW_FINAL_MONTH_RE =
    /^Your assistant has reviewed the squad contracts after your arrival\. (?<totalExpiring>\d+) player\(s\) are due to come up for renewal this season, (?<urgentContracts>\d+) need attention inside the next six months, and (?<finalMonthContracts>\d+) are already in the final month\.(?:\r?\n){2}Stabilize the urgent cases first, then work through the wider renewal list in stages\.$/;

export type BackendTextResolver = (
    key: string | undefined,
    fallback: string,
    params?: Record<string, string>,
) => string;

export function inferLegacyDelegatedRenewalsParams(
    message: MessageData,
): Record<string, string> | undefined {
    if (!message.id.startsWith(LEGACY_DELEGATED_RENEWALS_PREFIX)) {
        return undefined;
    }

    const summaryLine = message.body
        .split("\n")
        .map((line) => line.trim())
        .find((line) => line.length > 0);

    const match = summaryLine?.match(LEGACY_DELEGATED_RENEWALS_SUMMARY_RE);
    if (!match?.groups) {
        return undefined;
    }

    return {
        team: match.groups.team,
        successes: match.groups.successes,
        stalled: match.groups.stalled,
        failures: match.groups.failures,
    };
}

function resolveLegacyDelegatedRenewalsDetail(
    detail: string,
    resolve: BackendTextResolver,
): string {
    const beyondLimits = detail.match(LEGACY_DELEGATED_RENEWALS_BEYOND_LIMITS_RE);
    if (beyondLimits?.groups) {
        return resolve("be.msg.delegatedRenewals.notes.beyondLimits", detail, {
            wage: beyondLimits.groups.wage,
            years: beyondLimits.groups.years,
        });
    }

    const prefersManager = detail.match(LEGACY_DELEGATED_RENEWALS_PREFERS_MANAGER_RE);
    if (prefersManager?.groups) {
        return resolve("be.msg.delegatedRenewals.notes.prefersManager", detail, {
            wage: prefersManager.groups.wage,
            years: prefersManager.groups.years,
        });
    }

    if (LEGACY_DELEGATED_RENEWALS_MANAGER_BLOCKED_RE.test(detail)) {
        return resolve("be.msg.delegatedRenewals.notes.managerBlocked", detail);
    }

    if (LEGACY_DELEGATED_RENEWALS_RELATIONSHIP_BLOCKED_RE.test(detail)) {
        return resolve("be.msg.delegatedRenewals.notes.relationshipBlocked", detail);
    }

    return detail;
}

function resolveLegacyDelegatedRenewalsBody(
    body: string,
    resolve: BackendTextResolver,
): string {
    return body
        .split("\n")
        .map((line) => {
            const trimmed = line.trim();

            if (trimmed.length === 0) {
                return line;
            }

            const summary = trimmed.match(LEGACY_DELEGATED_RENEWALS_SUMMARY_RE);
            if (summary?.groups) {
                return resolve("be.msg.delegatedRenewals.body", trimmed, {
                    team: summary.groups.team,
                    successes: summary.groups.successes,
                    stalled: summary.groups.stalled,
                    failures: summary.groups.failures,
                });
            }

            const success = trimmed.match(LEGACY_DELEGATED_RENEWALS_SUCCESS_RE);
            if (success?.groups) {
                return resolve("be.msg.delegatedRenewals.case.successful", trimmed, {
                    player: success.groups.player,
                    years: success.groups.years,
                    wage: success.groups.wage,
                });
            }

            const status = trimmed.match(LEGACY_DELEGATED_RENEWALS_STATUS_RE);
            if (status?.groups) {
                const detail = resolveLegacyDelegatedRenewalsDetail(
                    status.groups.detail,
                    resolve,
                );
                const key =
                    status.groups.status === "Still difficult"
                        ? "be.msg.delegatedRenewals.case.stalled"
                        : "be.msg.delegatedRenewals.case.failed";

                return resolve(key, trimmed, {
                    player: status.groups.player,
                    detail,
                });
            }

            return line;
        })
        .join("\n");
}

export function resolveLegacyDelegatedRenewalsMessage(
    message: MessageData,
    resolve: BackendTextResolver,
    params?: Record<string, string>,
): MessageData {
    if (!message.id.startsWith(LEGACY_DELEGATED_RENEWALS_PREFIX)) {
        return message;
    }

    if (
        message.subject_key ||
        message.body_key ||
        message.sender_key ||
        message.sender_role_key
    ) {
        return message;
    }

    if (message.context?.delegated_renewal_report?.cases?.length) {
        return {
            ...message,
            subject: resolve("be.msg.delegatedRenewals.subject", message.subject, params),
            body: resolve("be.msg.delegatedRenewals.body", message.body, params),
            sender: resolve("be.sender.assistantManager", message.sender),
            sender_role: resolve("be.role.assistantManager", message.sender_role),
        };
    }

    return {
        ...message,
        subject: resolve("be.msg.delegatedRenewals.subject", message.subject, params),
        body: resolveLegacyDelegatedRenewalsBody(message.body, resolve),
        sender: resolve("be.sender.assistantManager", message.sender),
        sender_role: resolve("be.role.assistantManager", message.sender_role),
    };
}

function inferLegacyTakeoverContractReviewVariant(
    message: MessageData,
): { bodyKey: string; params: Record<string, string> } | undefined {
    if (
        !message.id.startsWith(LEGACY_TAKEOVER_CONTRACT_REVIEW_PREFIX) &&
        message.subject !== LEGACY_TAKEOVER_CONTRACT_REVIEW_SUBJECT
    ) {
        return undefined;
    }

    const noneUrgent = message.body.match(LEGACY_TAKEOVER_CONTRACT_REVIEW_NONE_URGENT_RE);
    if (noneUrgent?.groups) {
        return {
            bodyKey: "be.msg.takeoverContractReview.bodyNoneUrgent",
            params: {
                totalExpiring: noneUrgent.groups.totalExpiring,
                urgentContracts: "0",
                finalMonthContracts: "0",
            },
        };
    }

    const urgent = message.body.match(LEGACY_TAKEOVER_CONTRACT_REVIEW_URGENT_RE);
    if (urgent?.groups) {
        return {
            bodyKey: "be.msg.takeoverContractReview.bodyUrgent",
            params: {
                totalExpiring: urgent.groups.totalExpiring,
                urgentContracts: urgent.groups.urgentContracts,
                finalMonthContracts: "0",
            },
        };
    }

    const finalMonth = message.body.match(LEGACY_TAKEOVER_CONTRACT_REVIEW_FINAL_MONTH_RE);
    if (finalMonth?.groups) {
        return {
            bodyKey: "be.msg.takeoverContractReview.bodyFinalMonth",
            params: {
                totalExpiring: finalMonth.groups.totalExpiring,
                urgentContracts: finalMonth.groups.urgentContracts,
                finalMonthContracts: finalMonth.groups.finalMonthContracts,
            },
        };
    }

    return undefined;
}

function isLegacyTakeoverReviewAction(action: MessageAction): boolean {
    if (action.id === "view_squad" || action.id === "review_squad") {
        return true;
    }

    if (typeof action.action_type !== "object" || !("NavigateTo" in action.action_type)) {
        return false;
    }

    return action.action_type.NavigateTo.route === "/dashboard?tab=Squad";
}

function isLegacyTakeoverAcknowledgeAction(action: MessageAction): boolean {
    return action.id === "ack" || action.action_type === "Acknowledge";
}

export function resolveLegacyTakeoverContractReviewMessage(
    message: MessageData,
    resolve: BackendTextResolver,
): MessageData {
    if (
        message.subject_key ||
        message.body_key ||
        message.sender_key ||
        message.sender_role_key
    ) {
        return message;
    }

    const variant = inferLegacyTakeoverContractReviewVariant(message);
    if (!variant) {
        return message;
    }

    return {
        ...message,
        subject: resolve("be.msg.takeoverContractReview.subject", message.subject, variant.params),
        body: resolve(variant.bodyKey, message.body, variant.params),
        sender: resolve("be.sender.assistantManager", message.sender),
        sender_role: resolve("be.role.assistantManager", message.sender_role),
        actions: message.actions.map((action) => {
            if (action.label_key) {
                return action;
            }

            if (isLegacyTakeoverReviewAction(action)) {
                return {
                    ...action,
                    label: resolve(
                        "be.msg.takeoverContractReview.actionReview",
                        action.label,
                        variant.params,
                    ),
                };
            }

            if (isLegacyTakeoverAcknowledgeAction(action)) {
                return {
                    ...action,
                    label: resolve("be.msg.event.ack", action.label, variant.params),
                };
            }

            return action;
        }),
    };
}

export function normalizeNewsParams(
    article: NewsArticle,
): Record<string, string> | undefined {
    const params = article.i18n_params ? { ...article.i18n_params } : {};

    if (article.headline_key !== "be.news.weeklyDigest.headline") {
        return Object.keys(params).length > 0 ? params : article.i18n_params;
    }

    if (!params.weekStart && params.weekLabel) {
        const weekLabelMatch = params.weekLabel.match(LEGACY_WEEKLY_DIGEST_WEEK_LABEL_RE);

        if (weekLabelMatch?.groups?.weekStart) {
            params.weekStart = weekLabelMatch.groups.weekStart;
        }
    }

    if (!params.weekStart) {
        const headlineMatch = article.headline.match(LEGACY_WEEKLY_DIGEST_HEADLINE_RE);

        if (headlineMatch?.groups?.weekStart) {
            params.weekStart = headlineMatch.groups.weekStart;
        }
    }

    delete params.weekLabel;

    return Object.keys(params).length > 0 ? params : undefined;
}
