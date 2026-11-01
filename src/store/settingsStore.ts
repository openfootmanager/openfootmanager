import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface AppSettings {
  theme: "dark" | "light" | "system";
  language: string;
  currency: "EUR" | "GBP" | "USD";
  default_match_mode: "live" | "spectator" | "delegate";
  auto_save: boolean;
  match_speed: "slow" | "normal" | "fast";
  show_match_commentary: boolean;
  confirm_advance: boolean;
  continue_to_next_event: boolean;
  ui_scale: "small" | "normal" | "large" | "xlarge";
  high_contrast: boolean;
  /** The open sidebar's width. */
  sidebar_width: "compact" | "standard" | "wide";
  /** How the sidebar opens: as today, on hover, on a click, or never (icons only). */
  sidebar_behaviour: "normal" | "hover" | "click" | "icons";
  /** Whether the player folded the sidebar to its icon rail. */
  sidebar_collapsed: boolean;
  /** Whether a shootout starts in the automatic penalty order, without the order picker. */
  skip_shootout_picker: boolean;
  /** How every list opens: one choice for the whole game, not one per page. */
  list_view: ListView;
}

export type ListView = "table" | "cards";

export interface CurrencyDefinition {
  code: AppSettings["currency"];
  symbol: string;
  exchange_rate: number;
}

export interface SettingsResponse {
  settings: Partial<AppSettings>;
  currency: CurrencyDefinition;
  supported_currencies: CurrencyDefinition[];
}

const DEFAULT_SETTINGS: AppSettings = {
  theme: "dark",
  language: "en",
  currency: "EUR",
  default_match_mode: "live",
  auto_save: true,
  match_speed: "normal",
  show_match_commentary: true,
  confirm_advance: false,
  continue_to_next_event: false,
  ui_scale: "normal",
  high_contrast: false,
  sidebar_width: "standard",
  sidebar_behaviour: "normal",
  sidebar_collapsed: false,
  skip_shootout_picker: false,
  list_view: "table",
};

const DEFAULT_CURRENCY: CurrencyDefinition = {
  code: "EUR",
  symbol: "€",
  exchange_rate: 1,
};

function mergeWithDefaultSettings(settings: Partial<AppSettings> = {}): AppSettings {
  return { ...DEFAULT_SETTINGS, ...settings };
}

async function persistSettings(settings: AppSettings) {
  await invoke("save_settings", { settings });
}

function indexSupportedCurrencies(
  currencies: CurrencyDefinition[] = [],
): Record<string, CurrencyDefinition> {
  const index = currencies.reduce<Record<string, CurrencyDefinition>>((acc, currency) => {
    acc[currency.code] = currency;
    return acc;
  }, {});

  if (!index[DEFAULT_CURRENCY.code]) {
    index[DEFAULT_CURRENCY.code] = DEFAULT_CURRENCY;
  }

  return index;
}

function resolveCurrency(
  code: AppSettings["currency"],
  supportedCurrencies: Record<string, CurrencyDefinition>,
): CurrencyDefinition {
  return (
    supportedCurrencies[code] ?? supportedCurrencies[DEFAULT_SETTINGS.currency] ?? DEFAULT_CURRENCY
  );
}

interface SettingsStore {
  settings: AppSettings;
  currency: CurrencyDefinition;
  supportedCurrencies: Record<string, CurrencyDefinition>;
  loaded: boolean;
  loadSettings: () => Promise<void>;
  updateSettings: (partial: Partial<AppSettings>) => Promise<void>;
}

/**
 * What `updateSettings` changed while `get_settings` was still in flight, or null when no load is
 * pending. The load's answer is older than these, so it must not overwrite them: a view the
 * player picked a moment after the screen opened would otherwise flip back.
 */
let changedDuringLoad: Partial<AppSettings> | null = null;

export const useSettingsStore = create<SettingsStore>((set, get) => ({
  settings: mergeWithDefaultSettings(),
  currency: DEFAULT_CURRENCY,
  supportedCurrencies: indexSupportedCurrencies(),
  loaded: false,

  loadSettings: async () => {
    changedDuringLoad = {};
    try {
      const response = await invoke<SettingsResponse>("get_settings");
      const settings = mergeWithDefaultSettings({ ...response.settings, ...changedDuringLoad });
      const supportedCurrencies = indexSupportedCurrencies(response.supported_currencies);

      set({
        settings,
        currency: resolveCurrency(settings.currency, supportedCurrencies),
        supportedCurrencies,
        loaded: true,
      });
    } catch {
      set({
        settings: mergeWithDefaultSettings(changedDuringLoad ?? {}),
        currency: DEFAULT_CURRENCY,
        supportedCurrencies: indexSupportedCurrencies(),
        loaded: true,
      });
    } finally {
      changedDuringLoad = null;
    }
  },

  updateSettings: async (partial) => {
    if (changedDuringLoad) Object.assign(changedDuringLoad, partial);
    const previousState = get();
    const previousSettings = previousState.settings;
    const merged = mergeWithDefaultSettings({ ...previousSettings, ...partial });
    const currency = resolveCurrency(merged.currency, previousState.supportedCurrencies);

    set({ settings: merged, currency });
    try {
      await persistSettings(merged);
    } catch (err) {
      set({
        settings: previousSettings,
        currency: previousState.currency,
        supportedCurrencies: previousState.supportedCurrencies,
      });
      console.error("Failed to save settings:", err);
    }
  },
}));
