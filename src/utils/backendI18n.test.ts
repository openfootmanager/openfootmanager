import { describe, it, expect, beforeAll } from "vitest";
import i18n, { changeAppLanguage, i18nReady } from "../i18n";
import {
  resolveBackendText,
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
  await i18nReady;
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
    await changeAppLanguage("pt-BR");

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
      await changeAppLanguage(previousLanguage);
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
    await changeAppLanguage("pt-BR");

    try {
      const article = makeNewsArticle({
        headline: "Weekly Digest — Week of 2026-07-27",
        headline_key: "be.news.weeklyDigest.headline",
        i18n_params: { weekLabel: "Week of 2026-07-27" },
      });

      const result = resolveNewsArticle(article);

      expect(result.headline).toBe("Resumo Semanal — Semana de 2026-07-27");
    } finally {
      await changeAppLanguage(previousLanguage);
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
