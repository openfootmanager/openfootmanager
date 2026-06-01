import { useEffect, useState } from "react";
import { useNavigate, useLocation } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { useSettingsStore, AppSettings } from "../store/settingsStore";
import { useTheme } from "../context/ThemeContext";
import { ThemeToggle, Select } from "../components/ui";
import { SUPPORTED_LANGUAGES, changeAppLanguage } from "../i18n";
import {
  ArrowLeft,
  Monitor,
  Moon,
  Sun,
  Gamepad2,
  Save,
  Zap,
  Trash2,
  Download,
  Globe,
  Type,
  Maximize,
  Minimize,
} from "lucide-react";

const CURRENCY_OPTIONS = [
  { value: "EUR", labelKey: "settings.currencyOptions.eur", symbol: "€" },
  { value: "GBP", labelKey: "settings.currencyOptions.gbp", symbol: "£" },
  { value: "USD", labelKey: "settings.currencyOptions.usd", symbol: "$" },
] as const;

const THEME_OPTION_KEYS = ["light", "dark", "system"] as const;
const MATCH_MODE_KEYS = ["live", "spectator", "delegate"] as const;
const MATCH_SPEED_KEYS = ["slow", "normal", "fast"] as const;
const UI_SCALE_KEYS = ["small", "normal", "large", "xlarge"] as const;

export default function Settings() {
  const navigate = useNavigate();
  const location = useLocation();
  const { t, i18n } = useTranslation();
  const { settings, loaded, loadSettings, updateSettings } = useSettingsStore();
  const { theme, toggleTheme } = useTheme();
  const [confirmClear, setConfirmClear] = useState(false);
  const [clearSuccess, setClearSuccess] = useState(false);
  const [exportPath, setExportPath] = useState<string | null>(null);
  const [isFullscreen, setIsFullscreen] = useState(
    !!document.fullscreenElement,
  );

  // Where to go back to
  const returnTo = (location.state as { from?: string })?.from || "/";

  useEffect(() => {
    if (!loaded) loadSettings();
  }, [loaded, loadSettings]);

  // Track fullscreen state
  useEffect(() => {
    const handler = () => setIsFullscreen(!!document.fullscreenElement);
    document.addEventListener("fullscreenchange", handler);
    return () => document.removeEventListener("fullscreenchange", handler);
  }, []);

  const toggleFullscreen = async () => {
    if (document.fullscreenElement) {
      await document.exitFullscreen();
    } else {
      await document.documentElement.requestFullscreen();
    }
  };

  // Sync language with i18n when settings are loaded
  useEffect(() => {
    if (loaded && settings.language && settings.language !== i18n.language) {
      void changeAppLanguage(settings.language);
    }
  }, [loaded, settings.language, i18n]);

  const handleUpdate = (partial: Partial<AppSettings>) => {
    updateSettings(partial);

    // Sync theme with ThemeContext
    if (partial.theme) {
      const desired =
        partial.theme === "system"
          ? window.matchMedia("(prefers-color-scheme: dark)").matches
            ? "dark"
            : "light"
          : partial.theme;
      if (desired !== theme) toggleTheme();
    }

    // Sync language with i18n
    if (partial.language) {
      void changeAppLanguage(partial.language);
    }
  };

  const handleClearSaves = async () => {
    try {
      await invoke("clear_all_saves");
      setClearSuccess(true);
      setConfirmClear(false);
      setTimeout(() => setClearSuccess(false), 3000);
    } catch (err) {
      console.error("Failed to clear saves:", err);
    }
  };

  const handleExportWorld = async () => {
    try {
      // Simple export to app data dir
      const path = await invoke<string>("export_world_database", {
        exportPath: "exported_world.json",
      });
      setExportPath(path);
      setTimeout(() => setExportPath(null), 5000);
    } catch (err) {
      console.error("Failed to export world:", err);
    }
  };

  if (!loaded) {
    return (
      <div className="min-h-screen bg-gray-100 dark:bg-navy-900 flex items-center justify-center transition-colors">
        <div className="w-8 h-8 border-4 border-primary-500 border-t-transparent rounded-full animate-spin" />
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-100 dark:bg-navy-900 transition-colors duration-300">
      {/* Header */}
      <header className="bg-white dark:bg-navy-800 border-b border-gray-200 dark:border-navy-700 shadow-sm">
        <div className="max-w-3xl mx-auto px-6 py-4 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <button
              onClick={() => navigate(returnTo)}
              className="p-2 rounded-lg text-gray-400 hover:text-gray-700 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-navy-700 transition-colors"
            >
              <ArrowLeft className="w-5 h-5" />
            </button>
            <h1 className="text-xl font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white">
              {t("settings.title")}
            </h1>
          </div>
          <ThemeToggle />
        </div>
      </header>

      {/* Content */}
      <div className="max-w-3xl mx-auto px-6 py-8 flex flex-col gap-8">
        {/* ─── Display ─── */}
        <Section
          title={t("settings.display")}
          icon={<Monitor className="w-5 h-5" />}
        >
          <SettingRow
            label={t("settings.theme")}
            description={t("settings.themeDesc")}
          >
            <SegmentedControl
              options={THEME_OPTION_KEYS.map((key) => ({
                value: key,
                label: t(`settings.themeOptions.${key}`),
                icon:
                  key === "light" ? (
                    <Sun className="w-4 h-4" />
                  ) : key === "dark" ? (
                    <Moon className="w-4 h-4" />
                  ) : (
                    <Monitor className="w-4 h-4" />
                  ),
              }))}
              value={settings.theme}
              onChange={(v) =>
                handleUpdate({ theme: v as AppSettings["theme"] })
              }
            />
          </SettingRow>

          <SettingRow
            label={t("settings.language")}
            description={t("settings.languageDesc")}
          >
            <Select
              value={settings.language}
              onChange={(e) => handleUpdate({ language: e.target.value })}
              icon={<Globe className="w-4 h-4" />}
              className="min-w-48"
            >
              {SUPPORTED_LANGUAGES.map((lang) => (
                <option key={lang.code} value={lang.code}>
                  {t(lang.labelKey)}
                </option>
              ))}
            </Select>
          </SettingRow>

          <SettingRow
            label={t("settings.currency")}
            description={t("settings.currencyDesc")}
          >
            <Select
              value={settings.currency}
              onChange={(e) =>
                handleUpdate({
                  currency: e.target.value as AppSettings["currency"],
                })
              }
              className="min-w-48"
            >
              {CURRENCY_OPTIONS.map((c) => (
                <option key={c.value} value={c.value}>
                  {c.symbol} {t(c.labelKey)}
                </option>
              ))}
            </Select>
          </SettingRow>

          <SettingRow
            label={t("settings.uiScale")}
            description={t("settings.uiScaleDesc")}
          >
            <div className="flex items-center gap-2">
              <Type className="w-4 h-4 text-gray-400" />
              <SegmentedControl
                options={UI_SCALE_KEYS.map((key) => ({
                  value: key,
                  label: t(`settings.uiScaleOptions.${key}`),
                }))}
                value={settings.ui_scale}
                onChange={(v) =>
                  handleUpdate({ ui_scale: v as AppSettings["ui_scale"] })
                }
              />
            </div>
          </SettingRow>

          <SettingRow
            label={t("settings.highContrast")}
            description={t("settings.highContrastDesc")}
          >
            <Toggle
              checked={settings.high_contrast}
              onChange={(v) => handleUpdate({ high_contrast: v })}
            />
          </SettingRow>

          <SettingRow
            label={t("settings.fullscreen")}
            description={t("settings.fullscreenDesc")}
          >
            <button
              onClick={toggleFullscreen}
              className="flex items-center gap-2 px-4 py-2 rounded-lg bg-gray-100 dark:bg-navy-700 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-navy-600 text-sm font-heading font-bold uppercase tracking-wider transition-colors"
            >
              {isFullscreen ? (
                <Minimize className="w-4 h-4" />
              ) : (
                <Maximize className="w-4 h-4" />
              )}
              {isFullscreen
                ? t("settings.exitFullscreen")
                : t("settings.enterFullscreen")}
            </button>
          </SettingRow>
        </Section>

        {/* ─── Gameplay ─── */}
        <Section
          title={t("settings.gameplay")}
          icon={<Gamepad2 className="w-5 h-5" />}
        >
          <SettingRow
            label={t("settings.defaultMatchMode")}
            description={t("settings.defaultMatchModeDesc")}
          >
            <Select
              value={settings.default_match_mode}
              onChange={(e) =>
                handleUpdate({
                  default_match_mode: e.target
                    .value as AppSettings["default_match_mode"],
                })
              }
              className="min-w-48"
            >
              {MATCH_MODE_KEYS.map((k) => (
                <option key={k} value={k}>
                  {t(`settings.matchModes.${k}`)}
                </option>
              ))}
            </Select>
          </SettingRow>

          <SettingRow
            label={t("settings.matchSpeed")}
            description={t("settings.matchSpeedDesc")}
          >
            <SegmentedControl
              options={MATCH_SPEED_KEYS.map((k) => ({
                value: k,
                label: t(`settings.speeds.${k}`),
              }))}
              value={settings.match_speed}
              onChange={(v) =>
                handleUpdate({ match_speed: v as AppSettings["match_speed"] })
              }
            />
          </SettingRow>

          <SettingRow
            label={t("settings.matchCommentary")}
            description={t("settings.matchCommentaryDesc")}
          >
            <Toggle
              checked={settings.show_match_commentary}
              onChange={(v) => handleUpdate({ show_match_commentary: v })}
            />
          </SettingRow>

          <SettingRow
            label={t("settings.confirmAdvance")}
            description={t("settings.confirmAdvanceDesc")}
          >
            <Toggle
              checked={settings.confirm_advance}
              onChange={(v) => handleUpdate({ confirm_advance: v })}
            />
          </SettingRow>
        </Section>

        {/* ─── Saves & Data ─── */}
        <Section
          title={t("settings.savesData")}
          icon={<Save className="w-5 h-5" />}
        >
          <SettingRow
            label={t("settings.autoSave")}
            description={t("settings.autoSaveDesc")}
          >
            <Toggle
              checked={settings.auto_save}
              onChange={(v) => handleUpdate({ auto_save: v })}
            />
          </SettingRow>

          <SettingRow
            label={t("settings.exportWorld")}
            description={t("settings.exportWorldDesc")}
          >
            <button
              onClick={handleExportWorld}
              className="flex items-center gap-2 px-4 py-2 rounded-lg bg-primary-500/10 text-primary-600 dark:text-primary-400 hover:bg-primary-500/20 text-sm font-heading font-bold uppercase tracking-wider transition-colors"
            >
              <Download className="w-4 h-4" />
              {t("settings.export")}
            </button>
          </SettingRow>
          {exportPath && (
            <p className="text-xs text-primary-500 -mt-2 ml-1">
              {t("settings.exportedTo", { path: exportPath })}
            </p>
          )}

          <div className="border-t border-gray-200 dark:border-navy-600 pt-4 mt-2">
            <SettingRow
              label={t("settings.clearSaves")}
              description={t("settings.clearSavesDesc")}
              danger
            >
              {confirmClear ? (
                <div className="flex items-center gap-2">
                  <button
                    onClick={handleClearSaves}
                    className="px-4 py-2 rounded-lg bg-red-500 text-white text-sm font-heading font-bold uppercase tracking-wider hover:bg-red-600 transition-colors"
                  >
                    {t("common.confirm")}
                  </button>
                  <button
                    onClick={() => setConfirmClear(false)}
                    className="px-4 py-2 rounded-lg bg-gray-200 dark:bg-navy-600 text-gray-700 dark:text-gray-300 text-sm font-heading font-bold uppercase tracking-wider hover:bg-gray-300 dark:hover:bg-navy-500 transition-colors"
                  >
                    {t("common.cancel")}
                  </button>
                </div>
              ) : clearSuccess ? (
                <span className="text-sm text-primary-500 font-heading font-bold uppercase tracking-wider">
                  {t("settings.savesCleared")}
                </span>
              ) : (
                <button
                  onClick={() => setConfirmClear(true)}
                  className="flex items-center gap-2 px-4 py-2 rounded-lg bg-red-500/10 text-red-500 hover:bg-red-500/20 text-sm font-heading font-bold uppercase tracking-wider transition-colors"
                >
                  <Trash2 className="w-4 h-4" />
                  {t("settings.clear")}
                </button>
              )}
            </SettingRow>
          </div>
        </Section>

        {/* ─── About ─── */}
        <Section title={t("settings.about")} icon={<Zap className="w-5 h-5" />}>
          <div className="flex justify-between items-center">
            <div>
              <p className="text-sm font-medium text-gray-800 dark:text-gray-200">
                {t("app.name")}
              </p>
              <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                {t("app.version")}
              </p>
            </div>
            <span className="text-[10px] font-heading uppercase tracking-widest text-gray-400 dark:text-gray-600">
              {t("app.publisher")}
            </span>
          </div>
        </Section>
      </div>
    </div>
  );
}

// ── Reusable sub-components ──

function Section({
  title,
  icon,
  children,
}: {
  title: string;
  icon: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div className="bg-white dark:bg-navy-800 rounded-2xl border border-gray-200 dark:border-navy-700 shadow-sm overflow-hidden">
      <div className="flex items-center gap-2 px-6 py-4 border-b border-gray-100 dark:border-navy-700">
        <span className="text-primary-500">{icon}</span>
        <h2 className="text-sm font-heading font-bold uppercase tracking-wider text-gray-800 dark:text-gray-200">
          {title}
        </h2>
      </div>
      <div className="px-6 py-4 flex flex-col gap-5">{children}</div>
    </div>
  );
}

function SettingRow({
  label,
  description,
  danger,
  children,
}: {
  label: string;
  description: string;
  danger?: boolean;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-4">
      <div className="flex-1 min-w-0">
        <p
          className={`text-sm font-medium ${danger ? "text-red-500" : "text-gray-800 dark:text-gray-200"}`}
        >
          {label}
        </p>
        <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
          {description}
        </p>
      </div>
      <div className="flex-shrink-0">{children}</div>
    </div>
  );
}

function Toggle({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <button
      onClick={() => onChange(!checked)}
      className={`relative w-11 h-6 rounded-full transition-colors duration-200 ${checked ? "bg-primary-500" : "bg-gray-300 dark:bg-navy-600"
        }`}
    >
      <div
        className={`absolute top-0.5 w-5 h-5 bg-white rounded-full shadow-sm transition-transform duration-200 ${checked ? "translate-x-[22px]" : "translate-x-0.5"
          }`}
      />
    </button>
  );
}

function SegmentedControl({
  options,
  value,
  onChange,
}: {
  options: Array<{ value: string; label?: string; icon?: React.ReactNode }>;
  value: string;
  onChange: (v: string) => void;
}) {
  return (
    <div className="flex rounded-lg bg-gray-100 dark:bg-navy-700 p-0.5 border border-gray-200 dark:border-navy-600">
      {options.map((opt) => (
        <button
          key={opt.value}
          onClick={() => onChange(opt.value)}
          className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-heading font-bold uppercase tracking-wider transition-all ${value === opt.value
            ? "bg-white dark:bg-navy-500 text-primary-600 dark:text-primary-400 shadow-sm"
            : "text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300"
            }`}
        >
          {opt.icon}
          {opt.label || opt.value}
        </button>
      ))}
    </div>
  );
}
