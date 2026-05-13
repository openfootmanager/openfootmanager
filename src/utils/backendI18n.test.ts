import { describe, it, expect, beforeAll } from "vitest";
import i18n from "../i18n";
import {
  resolveBackendText,
  resolveBackendError,
  resolveMessage,
  resolveAction,
  resolveNewsArticle,
  resolveBoardObjective,
} from "./backendI18n";
import type {
  MessageData,
  MessageAction,
  NewsArticle,
  BoardObjective,
} from "../store/gameStore";

// ---------------------------------------------------------------------------
// Bootstrap i18n with a test key so we can verify resolution works
// ---------------------------------------------------------------------------

beforeAll(async () => {
  // Ensure i18n is initialized (it auto-inits on import) then add test keys
  await i18n.init; // no-op if already initialised
  i18n.addResourceBundle("en", "translation", {
    "test.subject": "Resolved Subject",
    "test.body": "Hello {{name}}, welcome!",
    "test.sender": "The Board",
    "test.senderRole": "Board of Directors",
    "test.actionLabel": "Accept Offer",
    "test.optionLabel": "Encourage them",
    "test.optionDescription": "Show empathy and keep them motivated.",
    "test.headline": "Breaking: {{team}} wins!",
    "test.newsBody": "Match report for {{team}}.",
    "test.source": "OFM Sports",
    "boardObjectives.objective.LeaguePosition": "Finish in the top {{target}}",
    "boardObjectives.objective.Wins": "Win at least {{target}} matches",
    "boardObjectives.objective.GoalsScored": "Score at least {{target}} goals",
    "boardObjectives.objective.FinancialStability": "Keep wage spending at or below {{target}}% of budget",
  }, true, true);
});

// ---------------------------------------------------------------------------
// Helpers to build minimal test data
// ---------------------------------------------------------------------------

const makeAction = (overrides: Partial<MessageAction> = {}): MessageAction => ({
  id: "act_1",
  label: "raw label",
  action_type: "Acknowledge",
  resolved: false,
  ...overrides,
});

const makeMessage = (overrides: Partial<MessageData> = {}): MessageData => ({
  id: "msg_1",
  subject: "raw subject",
  body: "raw body",
  sender: "raw sender",
  sender_role: "raw role",
  date: "2026-08-01",
  read: false,
  category: "general",
  priority: "normal",
  actions: [],
  context: { team_id: null, player_id: null, fixture_id: null, match_result: null },
  ...overrides,
});

const makeNewsArticle = (overrides: Partial<NewsArticle> = {}): NewsArticle => ({
  id: "news_1",
  headline: "raw headline",
  body: "raw body",
  source: "raw source",
  date: "2026-08-01",
  category: "match",
  team_ids: [],
  player_ids: [],
  match_score: null,
  read: false,
  ...overrides,
});

const makeBoardObjective = (
  overrides: Partial<BoardObjective> = {},
): BoardObjective => ({
  id: "obj_1",
  description: "raw objective",
  target: 4,
  objective_type: "LeaguePosition",
  met: false,
  ...overrides,
});

// ---------------------------------------------------------------------------
// resolveAction
// ---------------------------------------------------------------------------

describe("resolveAction", () => {
  it("returns action with resolved label when label_key exists", () => {
    const action = makeAction({ label: "fallback", label_key: "test.actionLabel" });
    const result = resolveAction(action);
    expect(result.label).toBe("Accept Offer");
  });

  it("resolves choose-option labels and descriptions when keys exist", () => {
    const action = makeAction({
      action_type: {
        ChooseOption: {
          options: [
            {
              id: "encourage",
              label: "fallback option",
              description: "fallback description",
              label_key: "test.optionLabel",
              description_key: "test.optionDescription",
            },
          ],
        },
      },
    });

    const result = resolveAction(action);

    if (typeof result.action_type !== "object" || !("ChooseOption" in result.action_type)) {
      throw new Error("Expected ChooseOption action type");
    }

    expect(result.action_type.ChooseOption.options[0].label).toBe("Encourage them");
    expect(result.action_type.ChooseOption.options[0].description).toBe(
      "Show empathy and keep them motivated.",
    );
  });

  it("resolves explicit random-event option keys with message interpolation params", () => {
    i18n.addResourceBundle("en", "translation", {
      be: {
        msg: {
          sponsor: {
            options: {
              accept: {
                label: "Accept the deal",
                description: "Receive €{{amount}} in sponsorship income.",
              },
            },
          },
        },
      },
    }, true, true);

    const result = resolveMessage(makeMessage({
      id: "sponsor_2026-08-01",
      i18n_params: { amount: "250000" },
      actions: [
        makeAction({
          id: "respond",
          label: "Respond",
          action_type: {
            ChooseOption: {
              options: [
                {
                  id: "accept",
                  label: "fallback option",
                  description: "fallback description",
                  label_key: "be.msg.sponsor.options.accept.label",
                  description_key: "be.msg.sponsor.options.accept.description",
                },
              ],
            },
          },
        }),
      ],
    }));

    const actionType = result.actions[0].action_type;

    if (typeof actionType !== "object" || !("ChooseOption" in actionType)) {
      throw new Error("Expected ChooseOption action type");
    }

    expect(actionType.ChooseOption.options[0].label).toBe("Accept the deal");
    expect(actionType.ChooseOption.options[0].description).toBe(
      "Receive €250000 in sponsorship income.",
    );
  });

  it("keeps raw label when label_key is absent", () => {
    const action = makeAction({ label: "Keep Me" });
    const result = resolveAction(action);
    expect(result.label).toBe("Keep Me");
  });

  it("falls back to raw label when key is not found in translations", () => {
    const action = makeAction({ label: "fallback", label_key: "nonexistent.key" });
    const result = resolveAction(action);
    expect(result.label).toBe("fallback");
  });

  it("infers player-event action and option keys for legacy saved messages", () => {
    i18n.addResourceBundle("en", "translation", {
      be: {
        msg: {
          playerEvent: {
            respond: "Custom Respond",
            options: {
              happyPlayer: {
                praiseBack: {
                  label: "Custom Praise Back",
                  description: "Custom praise description.",
                },
              },
            },
          },
        },
      },
    }, true, true);

    const action = makeAction({
      id: "respond",
      label: "Legacy respond",
      action_type: {
        ChooseOption: {
          options: [
            {
              id: "praise_back",
              label: "Legacy praise",
              description: "Legacy description",
            },
          ],
        },
      },
    });

    const result = resolveAction(action, "happy_player_p_fwd0");

    if (typeof result.action_type !== "object" || !("ChooseOption" in result.action_type)) {
      throw new Error("Expected ChooseOption action type");
    }

    expect(result.label).toBe("Custom Respond");
    expect(result.action_type.ChooseOption.options[0].label).toBe("Custom Praise Back");
    expect(result.action_type.ChooseOption.options[0].description).toBe("Custom praise description.");
  });
});

// ---------------------------------------------------------------------------
// resolveMessage
// ---------------------------------------------------------------------------

describe("resolveMessage", () => {
  it("resolves all translatable fields when keys exist", () => {
    const msg = makeMessage({
      subject: "raw", subject_key: "test.subject",
      body: "raw", body_key: "test.body",
      sender: "raw", sender_key: "test.sender",
      sender_role: "raw", sender_role_key: "test.senderRole",
      i18n_params: { name: "Coach" },
      actions: [makeAction({ label: "raw", label_key: "test.actionLabel" })],
    });
    const result = resolveMessage(msg);
    expect(result.subject).toBe("Resolved Subject");
    expect(result.body).toBe("Hello Coach, welcome!");
    expect(result.sender).toBe("The Board");
    expect(result.sender_role).toBe("Board of Directors");
    expect(result.actions[0].label).toBe("Accept Offer");
  });

  it("keeps raw values when no keys are provided", () => {
    const msg = makeMessage({
      subject: "My Subject",
      body: "My Body",
      sender: "Someone",
      sender_role: "Staff",
    });
    const result = resolveMessage(msg);
    expect(result.subject).toBe("My Subject");
    expect(result.body).toBe("My Body");
    expect(result.sender).toBe("Someone");
    expect(result.sender_role).toBe("Staff");
  });

  it("localizes legacy delegated renewal messages without persisted i18n keys", async () => {
    const previousLanguage = i18n.language;
    await i18n.changeLanguage("pt-BR");

    try {
      const msg = makeMessage({
        id: "delegated_renewals_2026-07-01_0",
        subject: "Assistant Report — Contract Renewals",
        body:
          "Boss, I went through our renewal list at Lisbon Sporting. 4 completed, 2 still pending, 1 failed.\n\nCompleted: Claes agreed to 1 year(s) on €5000/wk.\nStill difficult: Vieira — Their camp want around €25000/wk for 3 years, which is beyond the delegation limits.\nFailed: Fernandes — You told me not to reopen contract talks yet.",
        sender: "Assistant Manager",
        sender_role: "Assistant Manager",
      });

      const result = resolveMessage(msg);

      expect(result.subject).toBe(
        "Relatório do assistente — Renovações contratuais",
      );
      expect(result.sender).toBe("Auxiliar Técnico");
      expect(result.sender_role).toBe("Auxiliar Técnico");
      expect(result.body).toContain(
        "Chefe, revisei nossa lista de renovações no Lisbon Sporting. 4 concluídas, 2 ainda pendentes e 1 falhas.",
      );
      expect(result.body).toContain(
        "Concluída: Claes aceitou 1 ano(s) por €5000/semana.",
      );
      expect(result.body).toContain(
        "Continua difícil: Vieira — O estafe deles quer cerca de €25000/semana por 3 anos, acima dos limites da delegação.",
      );
      expect(result.body).toContain(
        "Falhou: Fernandes — Você me disse para ainda não reabrir as conversas contratuais.",
      );
    } finally {
      await i18n.changeLanguage(previousLanguage);
    }
  });

  it("localizes legacy takeover contract review messages without persisted i18n keys", async () => {
    const previousLanguage = i18n.language;
    await i18n.changeLanguage("pt-BR");

    try {
      const msg = makeMessage({
        id: "contract_review_takeover_team-1",
        subject: "Assistant Manager - Contract Review",
        body:
          "Your assistant has reviewed the squad contracts after your arrival. 2 player(s) are due to come up for renewal this season, but none require an immediate decision today.\n\nStart mapping out who you want to keep so the situation stays under control.",
        sender: "Assistant Manager",
        sender_role: "Assistant Manager",
        actions: [
          makeAction({
            id: "view_squad",
            label: "Review Squad Contracts",
            action_type: {
              NavigateTo: {
                route: "/dashboard?tab=Squad",
              },
            },
          }),
          makeAction({
            id: "ack",
            label: "Acknowledge",
            action_type: "Acknowledge",
          }),
        ],
      });

      const result = resolveMessage(msg);

      expect(result.subject).toBe("Assistente Técnico - Revisão de Contratos");
      expect(result.sender).toBe("Auxiliar Técnico");
      expect(result.sender_role).toBe("Auxiliar Técnico");
      expect(result.body).toBe(
        "Seu assistente revisou os contratos do elenco após a sua chegada. 2 jogador(es) devem entrar em negociação de renovação nesta temporada, mas nenhum exige uma decisão imediata hoje.\n\nComece a mapear quem você quer manter para que a situação fique sob controle.",
      );
      expect(result.actions[0].label).toBe("Revisar Contratos do Elenco");
      expect(result.actions[1].label).toBe("Entendido");
    } finally {
      await i18n.changeLanguage(previousLanguage);
    }
  });

  it("preserves non-translatable fields", () => {
    const msg = makeMessage({ id: "msg_99", read: true, category: "transfer" });
    const result = resolveMessage(msg);
    expect(result.id).toBe("msg_99");
    expect(result.read).toBe(true);
    expect(result.category).toBe("transfer");
  });
});

// ---------------------------------------------------------------------------
// resolveNewsArticle
// ---------------------------------------------------------------------------

describe("resolveNewsArticle", () => {
  it("resolves all translatable fields with params", () => {
    const article = makeNewsArticle({
      headline: "raw", headline_key: "test.headline",
      body: "raw", body_key: "test.newsBody",
      source: "raw", source_key: "test.source",
      i18n_params: { team: "Test FC" },
    });
    const result = resolveNewsArticle(article);
    expect(result.headline).toBe("Breaking: Test FC wins!");
    expect(result.body).toBe("Match report for Test FC.");
    expect(result.source).toBe("OFM Sports");
  });

  it("keeps raw values when no keys are provided", () => {
    const article = makeNewsArticle({
      headline: "Big News",
      body: "Details here",
      source: "Press",
    });
    const result = resolveNewsArticle(article);
    expect(result.headline).toBe("Big News");
    expect(result.body).toBe("Details here");
    expect(result.source).toBe("Press");
  });

  it("localizes legacy weekly digest headlines that still carry an English weekLabel param", async () => {
    const previousLanguage = i18n.language;
    await i18n.changeLanguage("pt-BR");

    try {
      const article = makeNewsArticle({
        headline: "Weekly Digest — Week of 2026-07-27",
        headline_key: "be.news.weeklyDigest.headline",
        i18n_params: { weekLabel: "Week of 2026-07-27" },
      });

      const result = resolveNewsArticle(article);

      expect(result.headline).toBe("Resumo Semanal — Semana de 2026-07-27");
    } finally {
      await i18n.changeLanguage(previousLanguage);
    }
  });

  it("localizes transfer roundup articles through backend keys", async () => {
    const previousLanguage = i18n.language;
    await i18n.changeLanguage("pt-BR");

    try {
      const article = makeNewsArticle({
        headline: "Transfer Roundup — Week of 2026-07-27",
        headline_key: "be.news.transferRoundup.headline",
        body: "The transfer market stayed busy this week.",
        body_key: "be.news.transferRoundup.body",
        source: "Transfer Intelligence",
        source_key: "be.source.transferIntelligence",
        i18n_params: {
          weekStart: "2026-07-27",
          transferCount: "2",
          dealsData: JSON.stringify([
            {
              player: "Adam Smith",
              fromTeam: "Alpha FC",
              toTeam: "Beta FC",
              fee: "€1.8M",
            },
          ]),
        },
      });

      const result = resolveNewsArticle(article);

      expect(result.headline).toBe("Resumo de Transferências — Semana de 2026-07-27");
      expect(result.body).toContain("2 transferência(s) concluída(s)");
      expect(result.body).toContain("Adam Smith: de Alpha FC para Beta FC (€1.8M)");
      expect(result.source).toBe("Inteligência de Transferências");
    } finally {
      await i18n.changeLanguage(previousLanguage);
    }
  });

  it("localizes friendly match report scorer sections through backend keys", async () => {
    const previousLanguage = i18n.language;
    await i18n.changeLanguage("pt-BR");

    try {
      const article = makeNewsArticle({
        headline: "Alpha FC 2 - 1 Beta FC: friendly report",
        headline_key: "be.news.matchReport.reportFriendly.title",
        body: "In friendly action, Alpha FC 2-1 Beta FC. Both sides used the fixture to build sharpness before the competitive campaign.\n\nGoals: Alice (10', Alpha FC)",
        body_key: "be.news.matchReport.reportFriendly.body",
        source: "Sports Gazette",
        source_key: "be.source.sportsGazette",
        i18n_params: {
          home: "Alpha FC",
          away: "Beta FC",
          homeGoals: "2",
          awayGoals: "1",
          scorersSection: "\n\nGoals: Alice (10', Alpha FC)",
          scorersData: JSON.stringify([
            {
              player: "Alice",
              minute: 10,
              team: "Alpha FC",
            },
          ]),
        },
      });

      const result = resolveNewsArticle(article);

      expect(result.headline).toBe("Alpha FC 2 - 1 Beta FC: resumo do amistoso");
      expect(result.body).toContain("No amistoso, Alpha FC 2 - 1 Beta FC.");
      expect(result.body).toContain("Gols: Alice (10', Alpha FC)");
      expect(result.source).toBe("Gazeta Esportiva");
    } finally {
      await i18n.changeLanguage(previousLanguage);
    }
  });

  it("localizes standings entries through backend keys", async () => {
    const previousLanguage = i18n.language;
    await i18n.changeLanguage("pt-BR");

    try {
      const article = makeNewsArticle({
        headline: "Table update",
        headline_key: "be.news.standings.headline2",
        body: "After Matchday 4, Alpha FC sit at the top of the Premier Division table.\n\nStandings:\n  1. Alpha FC — 12 pts (GD: +5)",
        body_key: "be.news.standings.body",
        source: "League Wire",
        source_key: "be.source.leagueWire",
        i18n_params: {
          matchday: "4",
          leader: "Alpha FC",
          standings: "  1. Alpha FC — 12 pts (GD: +5)",
          standingsData: JSON.stringify([
            {
              rank: 1,
              team: "Alpha FC",
              points: 12,
              goal_difference: "+5",
            },
          ]),
        },
      });

      const result = resolveNewsArticle(article);

      expect(result.headline).toBe("Atualização da Classificação — Rodada 4");
      expect(result.body).toContain("Após a Rodada 4, o Alpha FC está no topo da tabela da Primeira Divisão.");
      expect(result.body).toContain("1. Alpha FC — 12 pts (SG: +5)");
      expect(result.source).toBe("Notícias da Liga");
    } finally {
      await i18n.changeLanguage(previousLanguage);
    }
  });

  it("localizes roundup result lines and biggest winner copy through backend keys", async () => {
    const previousLanguage = i18n.language;
    await i18n.changeLanguage("pt-BR");

    try {
      const article = makeNewsArticle({
        headline: "Premier Division Matchday 4: All the Results",
        headline_key: "be.news.roundup.headline1",
        body: "Matchday 4 is in the books. Here are the full results:\n\n  Alpha FC 3 - 0 Beta FC\n  Gamma FC 1 - 1 Delta FC\n\n5 goals scored across 2 matches. Alpha FC recorded the biggest win of the day.",
        body_key: "be.news.roundup.body",
        source: "League Wire",
        source_key: "be.source.leagueWire",
        i18n_params: {
          matchday: "4",
          totalGoals: "5",
          matchCount: "2",
          results: "  Alpha FC 3 - 0 Beta FC\n  Gamma FC 1 - 1 Delta FC",
          resultsData: JSON.stringify([
            {
              home: "Alpha FC",
              home_goals: 3,
              away: "Beta FC",
              away_goals: 0,
            },
            {
              home: "Gamma FC",
              home_goals: 1,
              away: "Delta FC",
              away_goals: 1,
            },
          ]),
          biggestWinner: "Alpha FC",
        },
      });

      const result = resolveNewsArticle(article);

      expect(result.headline).toBe("Primeira Divisão Rodada 4: Todos os Resultados");
      expect(result.body).toContain("A Rodada 4 está encerrada. Confira os resultados completos:");
      expect(result.body).toContain("Alpha FC 3 - 0 Beta FC");
      expect(result.body).toContain("Gamma FC 1 - 1 Delta FC");
      expect(result.body).toContain("5 gols marcados em 2 jogos.");
      expect(result.body).toContain("O Alpha FC registrou a maior vitória do dia.");
      expect(result.source).toBe("Notícias da Liga");
    } finally {
      await i18n.changeLanguage(previousLanguage);
    }
  });

  it("localizes preseason digest result lines and unbeaten copy through backend keys", async () => {
    const previousLanguage = i18n.language;
    await i18n.changeLanguage("pt-BR");

    try {
      const article = makeNewsArticle({
        headline: "Preseason Digest — Week of 2025-08-11",
        headline_key: "be.news.preseasonDigest.headline",
        body: "The latest preseason digest is here. 2 friendly result(s) were played across the division this week, producing 3 goal(s).\n\nResults:\n  Alpha FC 2 - 1 Beta FC\n  Gamma FC 0 - 0 Delta FC\n\nAlpha FC and Gamma FC remain unbeaten in preseason.",
        body_key: "be.news.preseasonDigest.bodyWithResults",
        source: "League Chronicle",
        source_key: "be.source.leagueChronicle",
        i18n_params: {
          weekStart: "2025-08-11",
          resultCount: "2",
          totalGoals: "3",
          results: "  Alpha FC 2 - 1 Beta FC\n  Gamma FC 0 - 0 Delta FC",
          resultsData: JSON.stringify([
            { home: "Alpha FC", home_goals: 2, away: "Beta FC", away_goals: 1 },
            { home: "Gamma FC", home_goals: 0, away: "Delta FC", away_goals: 0 },
          ]),
          unbeatenLine: "\n\nAlpha FC and Gamma FC remain unbeaten in preseason.",
          unbeatenTeamsData: JSON.stringify(["Alpha FC", "Gamma FC"]),
        },
      });

      const result = resolveNewsArticle(article);

      expect(result.headline).toBe("Resumo da pré-temporada — Semana de 2025-08-11");
      expect(result.body).toContain("2 amistoso(s) foram disputados pela divisão nesta semana, produzindo 3 gol(s).");
      expect(result.body).toContain("Alpha FC 2 - 1 Beta FC");
      expect(result.body).toContain("Gamma FC 0 - 0 Delta FC");
      expect(result.body).toContain("Alpha FC e Gamma FC seguem invictos na pré-temporada.");
      expect(result.source).toBe("Crônica da Liga");
    } finally {
      await i18n.changeLanguage(previousLanguage);
    }
  });

  it("includes every unbeaten team when more than two clubs remain unbeaten", async () => {
    const previousLanguage = i18n.language;
    await i18n.changeLanguage("pt-BR");

    try {
      const article = makeNewsArticle({
        headline: "Preseason Digest — Week of 2025-08-11",
        headline_key: "be.news.preseasonDigest.headline",
        body: "The latest preseason digest is here. Training camps, selection decisions, and transfer business continue across the division as clubs prepare for opening day.\n\nAlpha FC, Gamma FC, and Delta FC remain unbeaten in preseason.",
        body_key: "be.news.preseasonDigest.bodyNoResults",
        source: "League Chronicle",
        source_key: "be.source.leagueChronicle",
        i18n_params: {
          weekStart: "2025-08-11",
          unbeatenLine: "\n\nAlpha FC, Gamma FC, and Delta FC remain unbeaten in preseason.",
          unbeatenTeamsData: JSON.stringify(["Alpha FC", "Gamma FC", "Delta FC"]),
        },
      });

      const result = resolveNewsArticle(article);

      expect(result.body).toContain("Alpha FC, Gamma FC e Delta FC seguem invictos na pré-temporada.");
    } finally {
      await i18n.changeLanguage(previousLanguage);
    }
  });

  it("preserves non-translatable fields", () => {
    const article = makeNewsArticle({ id: "n_5", category: "transfer", read: true });
    const result = resolveNewsArticle(article);
    expect(result.id).toBe("n_5");
    expect(result.category).toBe("transfer");
    expect(result.read).toBe(true);
  });
});

describe("resolveBoardObjective", () => {
  it("resolves objective text from objective_type and target", () => {
    const objective = makeBoardObjective({
      description: "boardObjectives.objective.LeaguePosition",
      target: 6,
      objective_type: "LeaguePosition",
    });

    const result = resolveBoardObjective(objective);

    expect(result.description).toBe("Finish in the top 6");
  });

  it("falls back to raw description for unknown objective types", () => {
    const objective = makeBoardObjective({
      description: "Custom target",
      objective_type: "CustomObjective",
    });

    const result = resolveBoardObjective(objective);

    expect(result.description).toBe("Custom target");
  });
});

describe("resolveBackendText", () => {
  it("resolves backend effect keys with params", () => {
    i18n.addResourceBundle("en", "translation", {
      "test.effect": "Morale {{delta}}",
    }, true, true);

    const result = resolveBackendText("test.effect", "fallback", { delta: "+3" });

    expect(result).toBe("Morale +3");
  });
});

describe("resolveBackendError", () => {
  it("resolves backend error keys", () => {
    i18n.addResourceBundle("en", "translation", {
      "be.error.noActiveGameSession": "No active game session",
    }, true, true);

    expect(resolveBackendError("be.error.noActiveGameSession")).toBe(
      "No active game session",
    );
  });

  it("keeps raw backend errors when no translation key exists", () => {
    expect(resolveBackendError(new Error("Something raw happened"))).toBe(
      "Something raw happened",
    );
  });
});
