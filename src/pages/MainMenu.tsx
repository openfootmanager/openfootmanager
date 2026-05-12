import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { useGameStore, GameStateData } from "../store/gameStore";
import { Button, ThemeToggle, DatePicker, CountryFlag } from "../components/ui";
import SavesList from "../components/menu/SavesList";
import WorldSelect, { WorldDatabaseInfo } from "../components/menu/WorldSelect";
import { resolveBackendError } from "../utils/backendI18n";
import {
  FolderOpen,
  Settings,
  X,
  PlusCircle,
  ChevronRight,
  AlertCircle,
  ChevronDown,
  Check,
  Power,
} from "lucide-react";
import { countryName, allNationalities } from "../lib/countries";

interface SaveEntry {
  id: string;
  name: string;
  manager_name: string;
  db_filename: string;
  checksum: string;
  created_at: string;
  last_played_at: string;
}

function normaliseSearchText(value: string): string {
  return value
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase();
}

/**
 * Minimum manager age (years) on create — historical rule, unchanged here on purpose.
 * Opinion (retraca, git 1631b76): this floor should probably be removed or lowered (~18);
 * leaving as-is until product agrees.
 */
const MANAGER_MINIMUM_AGE = 30;

function flooredAgeFromIsoDate(isoDob: string): number | null {
  if (!isoDob) return null;

  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(isoDob);
  if (!match) return null;

  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const birthDate = new Date(year, month - 1, day);

  if (
    Number.isNaN(birthDate.getTime()) ||
    birthDate.getFullYear() !== year ||
    birthDate.getMonth() !== month - 1 ||
    birthDate.getDate() !== day
  ) {
    return null;
  }

  const today = new Date();
  let age = today.getFullYear() - year;
  const hasHadBirthdayThisYear =
    today.getMonth() > month - 1 ||
    (today.getMonth() === month - 1 && today.getDate() >= day);

  if (!hasHadBirthdayThisYear) {
    age -= 1;
  }
  return Number.isNaN(age) ? null : age;
}

const CREATE_MANAGER_FIELD_ORDER = [
  "firstName",
  "lastName",
  "dob",
  "nationality",
] as const;

function prefersReducedMotion(): boolean {
  if (typeof window === "undefined") return false;
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function focusFirstCreateManagerError(errors: Record<string, string>): void {
  const first = CREATE_MANAGER_FIELD_ORDER.find((k) => errors[k]);
  if (!first) return;
  const root = document.getElementById(`create-manager-field-${first}`);
  root?.scrollIntoView?.({
    behavior: prefersReducedMotion() ? "auto" : "smooth",
    block: "center",
  });
  const focusable = root?.querySelector<HTMLElement>(
    "input:not([type=hidden]), button:not([disabled]), select, textarea",
  );
  focusable?.focus({ preventScroll: true });
}

function logNationalityDebug(
  message: string,
  details?: Record<string, unknown>,
): void {
  console.debug("[MainMenu nationality]", {
    message,
    ...(details ?? {}),
  });
}

export default function MainMenu() {
  const navigate = useNavigate();
  const setGameActive = useGameStore((state) => state.setGameActive);
  const setGameState = useGameStore((state) => state.setGameState);
  const { t, i18n } = useTranslation();

  const [menuState, setMenuState] = useState<
    "main" | "create" | "world" | "load"
  >("main");
  const [saves, setSaves] = useState<SaveEntry[]>([]);
  const [isLoadingSaves, setIsLoadingSaves] = useState(false);
  const [loadingSaveId, setLoadingSaveId] = useState<string | null>(null);
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const [isStarting, setIsStarting] = useState(false);

  const [formData, setFormData] = useState({
    firstName: "",
    lastName: "",
    dob: "",
    nationality: "",
  });
  const [formErrors, setFormErrors] = useState<Record<string, string>>({});
  const [nationalityOpen, setNationalityOpen] = useState(false);
  const [nationalitySearch, setNationalitySearch] = useState("");
  const nationalityRef = useRef<HTMLDivElement>(null);

  // World database state
  const [worldDatabases, setWorldDatabases] = useState<WorldDatabaseInfo[]>([]);
  const [selectedWorldId, setSelectedWorldId] = useState<string>("random");
  const [isLoadingWorlds, setIsLoadingWorlds] = useState(false);

  const countriesList = allNationalities(i18n.language);
  const normalisedNationalitySearch = normaliseSearchText(nationalitySearch);

  /** Same messages as `validateForm` for DOB, so the age rule surfaces as the user edits. */
  const dobLiveRuleMessage = (() => {
    if (!formData.dob) return null;
    const age = flooredAgeFromIsoDate(formData.dob);
    if (age === null) return t("validation.invalidDate");
    if (age < MANAGER_MINIMUM_AGE)
      return t("validation.minAge", { min: MANAGER_MINIMUM_AGE });
    if (age > 99) return t("validation.invalidDob");
    return null;
  })();
  const dobDisplayedError = formErrors.dob || dobLiveRuleMessage;

  const filteredNationalities = countriesList.filter((nationality) => {
    const normalisedName = normaliseSearchText(nationality.name);
    const normalisedCode = normaliseSearchText(nationality.code);

    return (
      normalisedName.includes(normalisedNationalitySearch) ||
      normalisedCode.includes(normalisedNationalitySearch)
    );
  });

  const toggleNationalityDropdown = () => {
    setNationalityOpen((open) => {
      const nextOpen = !open;
      logNationalityDebug("toggle button", { nextOpen });
      return nextOpen;
    });
    setNationalitySearch("");
  };

  const validateForm = (): {
    ok: boolean;
    errors: Record<string, string>;
  } => {
    const errors: Record<string, string> = {};
    if (!formData.firstName.trim()) {
      errors.firstName = t("validation.required", {
        field: t("createManager.firstName"),
      });
    } else if (formData.firstName.length > 30) {
      errors.firstName = t("validation.maxLength", {
        field: t("createManager.firstName"),
        max: 30,
      });
    }

    if (!formData.lastName.trim()) {
      errors.lastName = t("validation.required", {
        field: t("createManager.lastName"),
      });
    } else if (formData.lastName.length > 30) {
      errors.lastName = t("validation.maxLength", {
        field: t("createManager.lastName"),
        max: 30,
      });
    }

    if (!formData.dob) {
      errors.dob = t("validation.required", { field: t("createManager.dob") });
    } else {
      const age = flooredAgeFromIsoDate(formData.dob);
      if (age === null) {
        errors.dob = t("validation.invalidDate");
      } else if (age < MANAGER_MINIMUM_AGE) {
        errors.dob = t("validation.minAge", { min: MANAGER_MINIMUM_AGE });
      } else if (age > 99) {
        errors.dob = t("validation.invalidDob");
      }
    }
    if (!formData.nationality)
      errors.nationality = t("validation.required", {
        field: t("createManager.countryOfOrigin"),
      });
    setFormErrors(errors);
    return {
      ok: Object.keys(errors).length === 0,
      errors,
    };
  };

  const handleGoToWorldSelect = (e: React.FormEvent) => {
    e.preventDefault();
    const validation = validateForm();
    if (!validation.ok) {
      requestAnimationFrame(() =>
        focusFirstCreateManagerError(validation.errors),
      );
      return;
    }
    setMenuState("world");
    loadWorldDatabases();
  };

  // Close nationality dropdown on outside click
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (!nationalityOpen || !nationalityRef.current) {
        return;
      }

      const targetNode = e.target instanceof Node ? e.target : null;
      const eventPath =
        typeof e.composedPath === "function" ? e.composedPath() : [];
      const clickedInside =
        eventPath.includes(nationalityRef.current) ||
        (targetNode ? nationalityRef.current.contains(targetNode) : false);
      const targetElement = e.target instanceof HTMLElement ? e.target : null;

      logNationalityDebug("document mousedown", {
        clickedInside,
        targetTag: targetElement?.tagName.toLowerCase(),
        targetClass: targetElement?.className ?? "",
        targetText: targetElement?.textContent?.trim().slice(0, 60) ?? "",
      });

      if (!clickedInside) {
        logNationalityDebug("closing from outside click");
        setNationalityOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [nationalityOpen]);

  const loadWorldDatabases = async () => {
    setIsLoadingWorlds(true);
    try {
      const dbs = await invoke<WorldDatabaseInfo[]>("list_world_databases");
      setWorldDatabases(dbs);
    } catch (error) {
      console.error("Failed to load world databases:", error);
      // Always have random available even if scan fails
      setWorldDatabases([
        {
          id: "random",
          name: t("worldSelect.randomWorld"),
          description: t("worldSelect.randomDescription"),
          team_count: 8,
          player_count: 160,
          source: "builtin",
          path: "",
        },
      ]);
    } finally {
      setIsLoadingWorlds(false);
    }
  };

  const handleImportFile = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = async () => {
      try {
        const json = reader.result as string;
        const parsed = JSON.parse(json);
        const info: WorldDatabaseInfo = {
          id: `file:${file.name}`,
          name: parsed.name || file.name.replace(".json", ""),
          description: parsed.description || t("menu.importedDescription"),
          team_count: parsed.teams?.length ?? 0,
          player_count: parsed.players?.length ?? 0,
          source: "imported",
          path: "", // will use the parsed data directly
        };
        // Store the raw JSON in sessionStorage so we can write it to a temp path
        sessionStorage.setItem("imported_world_json", json);
        setWorldDatabases((prev) => {
          const filtered = prev.filter((d) => d.source !== "imported");
          return [...filtered, info];
        });
        setSelectedWorldId(info.id);
      } catch (err) {
        alert(t("menu.invalidWorldDb", { error: String(err) }));
      }
    };
    reader.readAsText(file);
    // Reset input so the same file can be re-selected
    e.target.value = "";
  };

  const handleStartGame = async () => {
    setIsStarting(true);
    try {
      // Determine world source
      let worldSource: string | undefined = selectedWorldId;
      if (selectedWorldId === "random") {
        worldSource = undefined;
      } else if (
        selectedWorldId.startsWith("file:") &&
        sessionStorage.getItem("imported_world_json")
      ) {
        // For imported files, write to a temp location first
        const json = sessionStorage.getItem("imported_world_json")!;
        // Write it via a temp file approach — just pass "random" and override
        // Actually, better to write the file to user databases dir first
        const path = await invoke<string>("write_temp_database", {
          json,
        }).catch(() => null);
        if (path) {
          worldSource = `file:${path}`;
        } else {
          // Fallback: pass the imported data inline — won't work with current backend
          // So fall back to random
          worldSource = undefined;
          console.warn(
            "Could not write imported database, falling back to random",
          );
        }
      }

      const game = await invoke<GameStateData>("start_new_game", {
        firstName: formData.firstName,
        lastName: formData.lastName,
        dob: formData.dob,
        nationality: formData.nationality,
        worldSource,
      });
      sessionStorage.removeItem("imported_world_json");
      setGameState(game);
      navigate("/select-team");
    } catch (error) {
      console.error("Failed to start game:", error);
      alert(
        t("menu.failedStartGame", {
          error: resolveBackendError(error),
        }),
      );
    } finally {
      setIsStarting(false);
    }
  };

  const handleOpenLoadMenu = async () => {
    setMenuState("load");
    setIsLoadingSaves(true);
    try {
      const dbSaves = await invoke<SaveEntry[]>("get_saves");
      setSaves(dbSaves);
    } catch (error) {
      console.error("Failed to load saves:", error);
    } finally {
      setIsLoadingSaves(false);
    }
  };

  const handleLoadGame = async (saveId: string) => {
    setLoadingSaveId(saveId);
    try {
      const managerName = await invoke<string>("load_game", { saveId });
      setGameActive(true, managerName);
      navigate("/dashboard");
    } catch (error) {
      console.error("Failed to load game:", error);
      setLoadingSaveId(null);
    }
  };

  const handleDeleteSave = async (saveId: string) => {
    try {
      await invoke<boolean>("delete_save", { saveId });
      setSaves((prev) => prev.filter((s) => s.id !== saveId));
      setConfirmDeleteId(null);
    } catch (error) {
      console.error("Failed to delete save:", error);
    }
  };

  const handleExitApp = async (): Promise<void> => {
    try {
      if (document.fullscreenElement) {
        await document.exitFullscreen();
      }
      await getCurrentWindow().destroy();
    } catch (error) {
      console.error("Failed to exit app:", error);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-100 dark:bg-navy-900 transition-colors duration-500 relative overflow-x-hidden">
      {/* Background gradient accents */}
      <div className="absolute inset-0 overflow-hidden pointer-events-none">
        <div className="absolute -top-40 -right-40 w-96 h-96 bg-primary-500/10 dark:bg-primary-500/5 rounded-full blur-3xl" />
        <div className="absolute -bottom-40 -left-40 w-96 h-96 bg-accent-400/10 dark:bg-accent-400/5 rounded-full blur-3xl" />
      </div>

      {/* Theme Toggle */}
      <ThemeToggle className="absolute top-6 right-6 z-20" />

      {/* Main Card */}
      <div className="relative z-10 w-full max-w-md">
        {/* Top accent bar */}
        <div className="h-1.5 bg-gradient-to-r from-primary-500 via-accent-400 to-primary-500 rounded-t-2xl" />

        <div className="bg-white dark:bg-navy-800 p-8 rounded-b-2xl shadow-xl dark:shadow-2xl border border-gray-200 dark:border-navy-600 border-t-0 transition-all duration-500">
          {/* Logo */}
          <img
            src="/openfootlogo.svg"
            alt="OpenFootball"
            className="text-center w-full h-full object-cover"
          />

          <div className="border-t border-gray-200 dark:border-navy-600 my-8 transition-colors duration-500" />

          {/* Main Menu */}
          {menuState === "main" && (
            <div className="flex flex-col gap-3">
              <button
                onClick={() => setMenuState("create")}
                className="group flex items-center justify-between w-full p-4 bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-600 hover:to-primary-700 text-white rounded-xl transition-all duration-300 shadow-md hover:shadow-lg hover:shadow-primary-500/20"
              >
                <div className="flex items-center gap-3">
                  <PlusCircle className="w-6 h-6" />
                  <span className="font-heading font-bold text-lg uppercase tracking-wide">
                    {t("menu.newGame")}
                  </span>
                </div>
                <ChevronRight className="w-5 h-5 opacity-70 group-hover:opacity-100 group-hover:translate-x-0.5 transition-all" />
              </button>

              <button
                onClick={handleOpenLoadMenu}
                className="group flex items-center justify-between w-full p-4 bg-white dark:bg-navy-700 hover:bg-gray-50 dark:hover:bg-navy-600 text-gray-800 dark:text-gray-200 rounded-xl transition-all duration-300 border border-gray-200 dark:border-navy-600 hover:border-accent-400 dark:hover:border-accent-400 shadow-sm"
              >
                <div className="flex items-center gap-3">
                  <FolderOpen className="w-6 h-6 text-accent-500 dark:text-accent-400" />
                  <span className="font-heading font-bold text-lg uppercase tracking-wide">
                    {t("menu.loadGame")}
                  </span>
                </div>
                <ChevronRight className="w-5 h-5 opacity-0 group-hover:opacity-70 group-hover:translate-x-0.5 transition-all text-accent-500" />
              </button>

              <button
                onClick={() => navigate("/settings", { state: { from: "/" } })}
                className="group flex items-center justify-between w-full p-4 bg-white dark:bg-navy-700 hover:bg-gray-50 dark:hover:bg-navy-600 text-gray-800 dark:text-gray-200 rounded-xl transition-all duration-300 border border-gray-200 dark:border-navy-600 hover:border-gray-300 dark:hover:border-navy-600 shadow-sm"
              >
                <div className="flex items-center gap-3">
                  <Settings className="w-6 h-6 text-gray-400 dark:text-gray-500" />
                  <span className="font-heading font-bold text-lg uppercase tracking-wide">
                    {t("menu.settings")}
                  </span>
                </div>
                <ChevronRight className="w-5 h-5 opacity-0 group-hover:opacity-70 group-hover:translate-x-0.5 transition-all text-gray-400" />
              </button>

              <button
                onClick={() => {
                  void handleExitApp();
                }}
                className="group flex items-center justify-between w-full p-4 bg-white dark:bg-navy-700 hover:bg-red-50 dark:hover:bg-red-500/10 text-gray-800 dark:text-gray-200 rounded-xl transition-all duration-300 border border-gray-200 dark:border-navy-600 hover:border-red-200 dark:hover:border-red-500/30 shadow-sm"
              >
                <div className="flex items-center gap-3">
                  <Power className="w-6 h-6 text-red-500 dark:text-red-400" />
                  <span className="font-heading font-bold text-lg uppercase tracking-wide">
                    {t("menu.exitGame")}
                  </span>
                </div>
              </button>
            </div>
          )}

          {/* Step 1: Create Manager Form */}
          {menuState === "create" && (
            <form
              onSubmit={handleGoToWorldSelect}
              className="flex flex-col gap-4"
            >
              <div className="flex justify-between items-center mb-2">
                <h2 className="text-xl font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white transition-colors">
                  {t("createManager.title")}
                </h2>
                <button
                  type="button"
                  onClick={() => {
                    setMenuState("main");
                    setFormErrors({});
                  }}
                  className="text-gray-400 hover:text-gray-700 dark:hover:text-white transition-colors p-1 rounded-lg hover:bg-gray-100 dark:hover:bg-navy-600"
                >
                  <X className="w-5 h-5" />
                </button>
              </div>

              {/* Step indicator */}
              <div className="flex items-center gap-2 mb-1">
                <div className="flex items-center justify-center w-6 h-6 rounded-full bg-primary-500 text-white text-xs font-bold">
                  1
                </div>
                <div className="h-0.5 flex-1 bg-gray-200 dark:bg-navy-600" />
                <div className="flex items-center justify-center w-6 h-6 rounded-full bg-gray-200 dark:bg-navy-600 text-gray-400 dark:text-gray-500 text-xs font-bold">
                  2
                </div>
              </div>

              {/* Name fields with labels */}
              <div className="flex gap-3">
                <div className="flex-1" id="create-manager-field-firstName">
                  <label className="block text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1.5">
                    {t("createManager.firstName")}
                  </label>
                  <input
                    maxLength={30}
                    className={`w-full bg-gray-50 dark:bg-navy-900 border text-gray-900 dark:text-white rounded-lg p-3 outline-none focus:ring-2 transition-all placeholder:text-gray-400 dark:placeholder:text-gray-500 ${formErrors.firstName
                        ? "border-red-400 dark:border-red-500 focus:border-red-500 focus:ring-red-500/20"
                        : "border-gray-300 dark:border-navy-600 focus:border-primary-500 focus:ring-primary-500/20"
                      }`}
                    placeholder={t("createManager.placeholderFirst")}
                    value={formData.firstName}
                    onChange={(e) => {
                      setFormData((prev) => ({
                        ...prev,
                        firstName: e.target.value,
                      }));
                      setFormErrors((prev) => ({ ...prev, firstName: "" }));
                    }}
                  />
                  {formErrors.firstName && (
                    <p className="flex items-center gap-1 text-xs text-red-500 mt-1">
                      <AlertCircle className="w-3 h-3" />
                      {formErrors.firstName}
                    </p>
                  )}
                </div>
                <div className="flex-1" id="create-manager-field-lastName">
                  <label className="block text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1.5">
                    {t("createManager.lastName")}
                  </label>
                  <input
                    maxLength={30}
                    className={`w-full bg-gray-50 dark:bg-navy-900 border text-gray-900 dark:text-white rounded-lg p-3 outline-none focus:ring-2 transition-all placeholder:text-gray-400 dark:placeholder:text-gray-500 ${formErrors.lastName
                        ? "border-red-400 dark:border-red-500 focus:border-red-500 focus:ring-red-500/20"
                        : "border-gray-300 dark:border-navy-600 focus:border-primary-500 focus:ring-primary-500/20"
                      }`}
                    placeholder={t("createManager.placeholderLast")}
                    value={formData.lastName}
                    onChange={(e) => {
                      setFormData((prev) => ({
                        ...prev,
                        lastName: e.target.value,
                      }));
                      setFormErrors((prev) => ({ ...prev, lastName: "" }));
                    }}
                  />
                  {formErrors.lastName && (
                    <p className="flex items-center gap-1 text-xs text-red-500 mt-1">
                      <AlertCircle className="w-3 h-3" />
                      {formErrors.lastName}
                    </p>
                  )}
                </div>
              </div>

              {/* Date of Birth with label */}
              <div id="create-manager-field-dob">
                <label className="block text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1.5">
                  {t("createManager.dob")}
                </label>
                <DatePicker
                  value={formData.dob}
                  onChange={(date) => {
                    setFormData((prev) => ({
                      ...prev,
                      dob: date,
                    }));
                    setFormErrors((prev) => ({ ...prev, dob: "" }));
                  }}
                  error={!!dobDisplayedError}
                />
                {dobDisplayedError && (
                  <p className="flex items-center gap-1 text-xs text-red-500 mt-1">
                    <AlertCircle className="w-3 h-3 shrink-0" />
                    {dobDisplayedError}
                  </p>
                )}
              </div>

              {/* Country/Region combobox — elevate stacking when open so the menu paints above the submit button */}
              <div
                id="create-manager-field-nationality"
                ref={nationalityRef}
                className={nationalityOpen ? "relative z-50" : undefined}
              >
                <label className="block text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1.5">
                  {t("createManager.countryOfOrigin")}
                </label>
                <div className="relative">
                  <button
                    type="button"
                    onMouseDown={(event) => {
                      event.preventDefault();
                      event.stopPropagation();
                      toggleNationalityDropdown();
                    }}
                    onClick={(event) => {
                      if (event.detail === 0) {
                        toggleNationalityDropdown();
                      }
                    }}
                    className={`w-full flex items-center justify-between bg-gray-50 dark:bg-navy-900 border text-left rounded-lg p-3 outline-none transition-all ${formErrors.nationality
                        ? "border-red-400 dark:border-red-500"
                        : nationalityOpen
                          ? "border-primary-500 ring-2 ring-primary-500/20"
                          : "border-gray-300 dark:border-navy-600"
                      }`}
                  >
                    <span
                      className={
                        formData.nationality
                          ? "text-gray-900 dark:text-white"
                          : "text-gray-400 dark:text-gray-500"
                      }
                    >
                      {formData.nationality ? (
                        <span className="flex items-center gap-2">
                          <CountryFlag
                            code={formData.nationality}
                            locale={i18n.language}
                            className="text-lg leading-none"
                          />
                          <span>
                            {countryName(formData.nationality, i18n.language) ||
                              formData.nationality}
                          </span>
                        </span>
                      ) : (
                        t("createManager.selectCountry")
                      )}
                    </span>
                    <ChevronDown
                      className={`w-4 h-4 text-gray-400 transition-transform ${nationalityOpen ? "rotate-180" : ""}`}
                    />
                  </button>

                  {nationalityOpen && (
                    <div
                      className="absolute z-50 bottom-full mb-1 left-0 right-0 bg-white dark:bg-navy-700 rounded-lg shadow-xl border border-gray-200 dark:border-navy-600 overflow-hidden"
                      onMouseDown={(event) => {
                        event.stopPropagation();
                        logNationalityDebug("dropdown panel mousedown");
                      }}
                    >
                      <div className="p-2 border-b border-gray-100 dark:border-navy-600">
                        <input
                          type="text"
                          autoFocus
                          placeholder={t("createManager.searchNationalities")}
                          value={nationalitySearch}
                          onChange={(e) => setNationalitySearch(e.target.value)}
                          className="w-full bg-gray-50 dark:bg-navy-800 border border-gray-200 dark:border-navy-600 text-gray-900 dark:text-white rounded-md px-3 py-2 text-sm outline-none focus:border-primary-500 transition-colors placeholder:text-gray-400 dark:placeholder:text-gray-500"
                        />
                      </div>
                      <div className="max-h-[min(20rem,calc(100vh-9rem))] overflow-y-auto overscroll-contain">
                        {filteredNationalities.length === 0 ? (
                          <p className="px-3 py-2 text-xs text-gray-400 dark:text-gray-500">
                            {t("menu.noResults")}
                          </p>
                        ) : (
                          filteredNationalities.map((nat) => (
                            <button
                              key={nat.code}
                              type="button"
                              onMouseDown={(event) => {
                                event.preventDefault();
                                event.stopPropagation();
                                logNationalityDebug("option selected", {
                                  code: nat.code,
                                  name: nat.name,
                                });
                                setFormData((prev) => ({
                                  ...prev,
                                  nationality: nat.code,
                                }));
                                setNationalityOpen(false);
                                setNationalitySearch("");
                                setFormErrors((prev) => ({
                                  ...prev,
                                  nationality: "",
                                }));
                              }}
                              className={`w-full text-left px-3 py-2 text-sm flex items-center justify-between transition-colors ${formData.nationality === nat.code
                                  ? "bg-primary-50 dark:bg-primary-500/10 text-primary-600 dark:text-primary-400"
                                  : "text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-navy-600"
                                }`}
                            >
                              <div className="flex items-center gap-2">
                                <CountryFlag
                                  code={nat.code}
                                  locale={i18n.language}
                                  className="text-lg leading-none"
                                />
                                <span>{nat.name}</span>
                              </div>
                              {formData.nationality === nat.code && (
                                <Check className="w-4 h-4 text-primary-500" />
                              )}
                            </button>
                          ))
                        )}
                      </div>
                    </div>
                  )}
                </div>
                {formErrors.nationality && (
                  <p className="flex items-center gap-1 text-xs text-red-500 mt-1">
                    <AlertCircle className="w-3 h-3" />
                    {formErrors.nationality}
                  </p>
                )}
              </div>

              <Button
                type="submit"
                variant="primary"
                size="lg"
                className="mt-2 w-full"
                iconRight={<ChevronRight />}
              >
                {t("createManager.chooseWorld")}
              </Button>
            </form>
          )}

          {/* Step 2: World Database Selection */}
          {menuState === "world" && (
            <WorldSelect
              worldDatabases={worldDatabases}
              selectedWorldId={selectedWorldId}
              isLoadingWorlds={isLoadingWorlds}
              isStarting={isStarting}
              onSelectWorld={setSelectedWorldId}
              onImportFile={handleImportFile}
              onStart={handleStartGame}
              onBack={() => setMenuState("create")}
              onClose={() => setMenuState("main")}
            />
          )}

          {/* Load Game List */}
          {menuState === "load" && (
            <SavesList
              loadingSaveId={loadingSaveId}
              saves={saves}
              isLoading={isLoadingSaves}
              confirmDeleteId={confirmDeleteId}
              onLoad={handleLoadGame}
              onDelete={handleDeleteSave}
              onConfirmDelete={setConfirmDeleteId}
              onClose={() => setMenuState("main")}
            />
          )}
        </div>
      </div>

      {/* Version */}
      <div className="absolute bottom-4 right-4 text-gray-400 dark:text-gray-600 text-xs font-heading uppercase tracking-widest transition-colors">
        {t("app.version")}
      </div>
    </div>
  );
}
