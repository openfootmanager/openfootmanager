import i18n from '../i18n';
import type {
  MessageActionOption,
  MessageData,
  MessageAction,
  NewsArticle,
  BoardObjective,
} from '../store/gameStore';
import {
  inferLegacyDelegatedRenewalsParams,
  normalizeNewsParams,
  resolveLegacyTakeoverContractReviewMessage,
  resolveLegacyDelegatedRenewalsMessage,
} from './backendI18n.legacy';
import {
  inferPlayerEventActionLabelKey,
  inferPlayerEventOptionBaseKey,
} from './backendI18nPlayerEvents.ts';

/**
 * Resolve a backend i18n key with params, falling back to the raw string.
 */
function resolve(key: string | undefined, fallback: string, params?: Record<string, string>): string {
  if (!key) return fallback;
  const resolved = i18n.t(key, params ?? {});
  // i18next returns the key itself if not found — fall back to raw string
  if (resolved === key) return fallback;
  return resolved;
}

function isTranslationKey(value: string): boolean {
  return value.includes('.') && i18n.t(value) !== value;
}

function extractErrorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;

  if (
    error &&
    typeof error === "object" &&
    "message" in error &&
    typeof (error as { message?: unknown }).message === "string"
  ) {
    return (error as { message: string }).message;
  }

  return "";
}

export function resolveBackendText(
  key: string | undefined,
  fallback: string,
  params?: Record<string, string>,
): string {
  return resolve(key, fallback, params);
}

export function resolveBackendError(error: unknown): string {
  const message = extractErrorMessage(error).trim();
  if (!message) return "";
  return resolve(isTranslationKey(message) ? message : undefined, message || "");
}

function boardObjectiveFallback(objective: BoardObjective): string {
  switch (objective.objective_type) {
    case 'LeaguePosition':
      return `Finish in the top ${objective.target}`;
    case 'Wins':
      return `Win at least ${objective.target} matches`;
    case 'GoalsScored':
      return `Score at least ${objective.target} goals`;
    case 'FinancialStability':
      return `Keep wage spending at or below ${objective.target}% of budget`;
    default:
      return objective.description;
  }
}

/**
 * Pre-resolve any param values that are themselves i18n keys (e.g. "common.moods.excellent").
 * A value is treated as a key if it contains a dot and i18next resolves it to something different.
 */
function resolveParamValues(params?: Record<string, string>): Record<string, string> | undefined {
  if (!params) return params;
  const resolved = { ...params };
  for (const [key, value] of Object.entries(resolved)) {
    if (value.includes('.')) {
      const attempted = i18n.t(value);
      if (attempted !== value) {
        resolved[key] = attempted;
      }
    }
  }
  return resolved;
}

type TransferRoundupDealParam = {
  player: string;
  fromTeam: string;
  toTeam: string;
  fee: string;
};

type MatchReportScorerParam = {
  player: string;
  minute: string | number;
  team: string;
};

type StandingsEntryParam = {
  rank: number;
  team: string;
  points: string | number;
  goal_difference?: string;
  goalDifference?: string;
};

type RoundupResultParam = {
  home: string;
  home_goals?: string | number;
  homeGoals?: string | number;
  away: string;
  away_goals?: string | number;
  awayGoals?: string | number;
};

function normalizePreseasonDigestParams(
  article: NewsArticle,
  params?: Record<string, string>,
): Record<string, string> | undefined {
  if (
    article.body_key !== 'be.news.preseasonDigest.bodyNoResults' &&
    article.body_key !== 'be.news.preseasonDigest.bodyWithResults'
  ) {
    return params;
  }

  if (!params) {
    return params;
  }

  const normalized = { ...params };

  type ListFormatConstructor = new (
    locales?: string | string[],
    options?: { style?: 'long' | 'short' | 'narrow'; type?: 'conjunction' | 'disjunction' | 'unit' },
  ) => { format(items: string[]): string };

  const formatTeamList = (teams: string[]): string => {
    const listFormat = (Intl as typeof Intl & { ListFormat?: ListFormatConstructor }).ListFormat;

    if (typeof Intl !== 'undefined' && typeof listFormat === 'function') {
      return new listFormat(i18n.resolvedLanguage || i18n.language || undefined, {
        style: 'long',
        type: 'conjunction',
      }).format(teams);
    }

    if (teams.length <= 1) {
      return teams[0] ?? '';
    }

    return `${teams.slice(0, -1).join(', ')} and ${teams[teams.length - 1]}`;
  };

  if (params.resultsData) {
    try {
      const results = JSON.parse(params.resultsData) as RoundupResultParam[];
      normalized.results = results
        .map((result) =>
          resolve(
            'be.news.preseasonDigest.resultLine',
            `  ${result.home} ${result.home_goals ?? result.homeGoals ?? ''} - ${result.away_goals ?? result.awayGoals ?? ''} ${result.away}`,
            {
              home: result.home,
              homeGoals: String(result.home_goals ?? result.homeGoals ?? ''),
              away: result.away,
              awayGoals: String(result.away_goals ?? result.awayGoals ?? ''),
            },
          ),
        )
        .join('\n');
    } catch {
      return params;
    }
  }

  if (params.unbeatenTeamsData) {
    try {
      const teams = JSON.parse(params.unbeatenTeamsData) as string[];
      const teamList = formatTeamList(teams);
      normalized.unbeatenLine = teams.length === 0
        ? ''
        : teams.length === 1
          ? resolve(
            'be.news.preseasonDigest.unbeatenLine.one',
            `\n\n${teams[0]} remain unbeaten in preseason.`,
            { team: teams[0] },
          )
          : teams.length === 2
            ? resolve(
              'be.news.preseasonDigest.unbeatenLine.two',
              `\n\n${teams[0]} and ${teams[1]} remain unbeaten in preseason.`,
              { first: teams[0], second: teams[1] },
            )
            : resolve(
              'be.news.preseasonDigest.unbeatenLine.multiple',
              `\n\n${teamList} remain unbeaten in preseason.`,
              { teams: teamList, count: String(teams.length) },
            );
    } catch {
      return params;
    }
  }

  return normalized;
}

function normalizeRoundupParams(
  article: NewsArticle,
  params?: Record<string, string>,
): Record<string, string> | undefined {
  if (article.body_key !== 'be.news.roundup.body' || !params) {
    return params;
  }

  const normalized = { ...params };

  if (params.resultsData) {
    try {
      const results = JSON.parse(params.resultsData) as RoundupResultParam[];
      normalized.results = results
        .map((result) =>
          resolve(
            'be.news.roundup.resultLine',
            `  ${result.home} ${result.home_goals ?? result.homeGoals ?? ''} - ${result.away_goals ?? result.awayGoals ?? ''} ${result.away}`,
            {
              home: result.home,
              homeGoals: String(result.home_goals ?? result.homeGoals ?? ''),
              away: result.away,
              awayGoals: String(result.away_goals ?? result.awayGoals ?? ''),
            },
          ),
        )
        .join('\n');
    } catch {
      return params;
    }
  }

  normalized.biggestWinnerLine = params.biggestWinner?.trim()
    ? resolve(
      'be.news.roundup.biggestWinnerLine',
      ` ${params.biggestWinner} recorded the biggest win of the day.`,
      { biggestWinner: params.biggestWinner },
    )
    : '';

  return normalized;
}

function normalizeStandingsParams(
  article: NewsArticle,
  params?: Record<string, string>,
): Record<string, string> | undefined {
  if (article.body_key !== 'be.news.standings.body' || !params?.standingsData) {
    return params;
  }

  try {
    const entries = JSON.parse(params.standingsData) as StandingsEntryParam[];
    return {
      ...params,
      standings: entries
        .map((entry) =>
          resolve(
            'be.news.standings.entry',
            `  ${entry.rank}. ${entry.team} — ${entry.points} pts (GD: ${entry.goal_difference ?? entry.goalDifference ?? ''})`,
            {
              rank: String(entry.rank),
              team: entry.team,
              points: String(entry.points),
              goalDifference: String(entry.goal_difference ?? entry.goalDifference ?? ''),
            },
          ),
        )
        .join('\n'),
    };
  } catch {
    return params;
  }
}

function normalizeMatchReportParams(
  article: NewsArticle,
  params?: Record<string, string>,
): Record<string, string> | undefined {
  if (
    article.body_key !== 'be.news.matchReport.reportFriendly.body' &&
    article.body_key !== 'be.news.matchReport.reportPreseason.body'
  ) {
    return params;
  }

  if (!params?.scorersData) {
    return params;
  }

  try {
    const scorers = JSON.parse(params.scorersData) as MatchReportScorerParam[];

    if (scorers.length === 0) {
      return {
        ...params,
        scorersSection: '',
      };
    }

    const scorersText = scorers
      .map((scorer) =>
        resolve('be.news.matchReport.scorer', `${scorer.player} (${scorer.minute}', ${scorer.team})`, {
          player: scorer.player,
          minute: String(scorer.minute),
          team: scorer.team,
        }),
      )
      .join(', ');

    return {
      ...params,
      scorersSection: resolve(
        'be.news.matchReport.scorersSection',
        `\n\nGoals: ${scorersText}`,
        { scorers: scorersText },
      ),
    };
  } catch {
    return params;
  }
}

function normalizeTransferRoundupParams(
  article: NewsArticle,
  params?: Record<string, string>,
): Record<string, string> | undefined {
  if (article.body_key !== 'be.news.transferRoundup.body' || !params?.dealsData) {
    return params;
  }

  try {
    const deals = JSON.parse(params.dealsData) as TransferRoundupDealParam[];
    return {
      ...params,
      deals: deals
        .map((deal) =>
          resolve(
            'be.news.transferRoundup.dealLine',
            `  ${deal.player}: ${deal.fromTeam} -> ${deal.toTeam} (${deal.fee})`,
            deal,
          ),
        )
        .join('\n'),
    };
  } catch {
    return params;
  }
}

/**
 * Resolve all translatable fields on a message, returning a copy with resolved strings.
 */
export function resolveMessage(msg: MessageData): MessageData {
  const inferredParams = msg.i18n_params ?? inferLegacyDelegatedRenewalsParams(msg);
  const p = resolveParamValues(inferredParams);
  const resolved = {
    ...msg,
    i18n_params: inferredParams,
    subject: resolve(msg.subject_key, msg.subject, p),
    body: resolve(msg.body_key, msg.body, p),
    sender: resolve(msg.sender_key, msg.sender, p),
    sender_role: resolve(msg.sender_role_key, msg.sender_role, p),
    actions: msg.actions.map((action) => resolveAction(action, msg.id, p)),
  };

  return resolveLegacyTakeoverContractReviewMessage(
    resolveLegacyDelegatedRenewalsMessage(resolved, resolve, p),
    resolve,
  );
}

/**
 * Resolve the label on a message action.
 */
export function resolveAction(
  action: MessageAction,
  messageId?: string,
  params?: Record<string, string>,
): MessageAction {
  const labelKey = action.label_key ?? inferPlayerEventActionLabelKey(messageId ?? '', action.id);

  if (typeof action.action_type === 'object' && 'ChooseOption' in action.action_type) {
    return {
      ...action,
      label: resolve(labelKey, action.label, params),
      action_type: {
        ChooseOption: {
          options: action.action_type.ChooseOption.options.map((option) =>
            resolveActionOption(option, messageId, params),
          ),
        },
      },
    };
  }

  return {
    ...action,
    label: resolve(labelKey, action.label, params),
  };
}

function resolveActionOption(
  option: MessageActionOption,
  messageId?: string,
  params?: Record<string, string>,
): MessageActionOption {
  const baseKey = inferPlayerEventOptionBaseKey(messageId ?? '', option.id);
  const labelKey = option.label_key ?? (baseKey ? `${baseKey}.label` : undefined);
  const descriptionKey = option.description_key ?? (baseKey ? `${baseKey}.description` : undefined);

  return {
    ...option,
    label: resolve(labelKey, option.label, params),
    description: resolve(descriptionKey, option.description, params),
  };
}

/**
 * Resolve all translatable fields on a news article, returning a copy with resolved strings.
 */
export function resolveNewsArticle(article: NewsArticle): NewsArticle {
  const p = normalizeTransferRoundupParams(
    article,
    normalizeMatchReportParams(
      article,
      normalizeStandingsParams(
        article,
        normalizeRoundupParams(
          article,
          normalizePreseasonDigestParams(article, normalizeNewsParams(article)),
        ),
      ),
    ),
  );
  return {
    ...article,
    i18n_params: p,
    headline: resolve(article.headline_key, article.headline, p),
    body: resolve(article.body_key, article.body, p),
    source: resolve(article.source_key, article.source, p),
  };
}

/**
 * Resolve a board objective description from its structured type and target.
 */
export function resolveBoardObjective(objective: BoardObjective): BoardObjective {
  const descriptionKey = `boardObjectives.objective.${objective.objective_type}`;
  const params = { target: String(objective.target) };

  return {
    ...objective,
    description: resolve(descriptionKey, boardObjectiveFallback(objective), params),
  };
}
