import { Suspense, lazy, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { useGameStore, GameStateData } from "../store/gameStore";
import { ThemeToggle } from "../components/ui/ThemeToggle";
import type {
  CareerStartPhase,
  CreateManagerFormData,
} from "../components/menu/CreateManagerForm";
import type { ManagerProfile } from "../components/menu/types";
import type { WorldDatabaseInfo } from "../components/menu/WorldSelect";
import { resolveBackendError } from "../utils/backendI18n";
import {
  FolderOpen,
  Settings,
  PlusCircle,
  ChevronRight,
  Power,
} from "lucide-react";

const DISCORD_INVITE_URL = "https://discord.gg/2CXaesaukT";
const GITHUB_REPO_URL = "https://github.com/openfootmanager/openfootmanager";

function DiscordIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" className={className} fill="currentColor" aria-hidden="true">
      <path d="M20.317 4.369a19.79 19.79 0 0 0-4.885-1.515.075.075 0 0 0-.079.038c-.21.375-.444.864-.608 1.249a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037 19.736 19.736 0 0 0-4.885 1.515.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994a.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128c.126-.094.252-.192.372-.291a.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.061 0a.074.074 0 0 1 .078.009c.12.099.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.891.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.84 19.84 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.029ZM8.02 15.331c-1.182 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418Zm7.974 0c-1.182 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418Z" />
    </svg>
  );
}

function GithubIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" className={className} fill="currentColor" aria-hidden="true">
      <path d="M12 .5C5.65.5.5 5.65.5 12c0 5.085 3.292 9.387 7.86 10.91.575.107.785-.25.785-.555 0-.275-.01-1-.015-1.965-3.2.695-3.875-1.54-3.875-1.54-.525-1.33-1.28-1.685-1.28-1.685-1.045-.715.08-.7.08-.7 1.155.08 1.765 1.185 1.765 1.185 1.025 1.755 2.69 1.25 3.345.955.1-.745.4-1.25.725-1.54-2.555-.29-5.245-1.275-5.245-5.685 0-1.255.45-2.28 1.18-3.085-.12-.29-.515-1.46.11-3.05 0 0 .965-.31 3.165 1.18a10.95 10.95 0 0 1 2.88-.39c.98.005 1.965.135 2.885.39 2.2-1.49 3.16-1.18 3.16-1.18.63 1.59.235 2.76.115 3.05.735.805 1.18 1.83 1.18 3.085 0 4.42-2.695 5.39-5.265 5.675.41.355.78 1.055.78 2.125 0 1.535-.015 2.77-.015 3.15 0 .305.205.665.79.55C20.215 21.385 23.5 17.085 23.5 12 23.5 5.65 18.35.5 12 .5Z" />
    </svg>
  );
}

const CreateManagerForm = lazy(
  () => import("../components/menu/CreateManagerForm"),
);
const ProfileSaveConfirm = lazy(
  () => import("../components/menu/ProfileSaveConfirm"),
);
const SavesList = lazy(() => import("../components/menu/SavesList"));
const WorldSelect = lazy(() => import("../components/menu/WorldSelect"));

interface SaveEntry {
  id: string;
  name: string;
  manager_name: string;
  team_name: string;
  db_filename: string;
  checksum: string;
  created_at: string;
  last_played_at: string;
}

/**
 * Minimum manager age (years) on create.
 */
const MANAGER_MINIMUM_AGE = 30;
const MIN_CAREER_START_YEAR = 2020;
const DEFAULT_GENERATED_HISTORY_DEPTH_YEARS = 12;
const MAX_GENERATED_HISTORY_DEPTH_YEARS = 24;
const GENERATED_HISTORY_DEPTH_STORAGE_KEY = "ofm-generated-history-depth-years";

type StartupOptionsPayload = {
  startYear: number;
  startPhase: CareerStartPhase;
  historyDepthYears: number;
};

function historyModeFromMetadata(
  metadata: unknown,
): WorldDatabaseInfo["history_mode"] {
  const kind =
    metadata && typeof metadata === "object" && "kind" in metadata
      ? (metadata as { kind?: unknown }).kind
      : undefined;

  if (kind === "historicalSnapshot") return "reference";
  if (kind === "rosterBaseline") return "hybrid";
  return undefined;
}

function defaultCareerStartYear(): string {
  return String(new Date().getFullYear());
}

function parseCareerStartYear(rawValue: string): number | null {
  const trimmed = rawValue.trim();
  if (!/^\d+$/.test(trimmed)) return null;

  const parsed = Number(trimmed);
  if (!Number.isInteger(parsed)) return null;
  return parsed;
}

function isCareerStartPhase(value: string): value is CareerStartPhase {
  return value === "seasonStart" || value === "midSeason";
}

function normalizeHistoryDepthYears(value: number): number | null {
  if (!Number.isInteger(value)) return null;
  if (value < 0 || value > MAX_GENERATED_HISTORY_DEPTH_YEARS) return null;
  return value;
}

function initialHistoryDepthYears(): number {
  if (typeof window === "undefined") {
    return DEFAULT_GENERATED_HISTORY_DEPTH_YEARS;
  }

  const storedValue = window.localStorage.getItem(
    GENERATED_HISTORY_DEPTH_STORAGE_KEY,
  );
  if (storedValue === null) {
    return DEFAULT_GENERATED_HISTORY_DEPTH_YEARS;
  }

  const parsedValue = Number(storedValue);
  return (
    normalizeHistoryDepthYears(parsedValue) ??
    DEFAULT_GENERATED_HISTORY_DEPTH_YEARS
  );
}

function buildStartupOptions(
  formData: CreateManagerFormData,
  historyDepthYears: number,
): StartupOptionsPayload | null {
  const startYear = parseCareerStartYear(formData.startYear);
  if (startYear === null || startYear < MIN_CAREER_START_YEAR) {
    return null;
  }
  if (!isCareerStartPhase(formData.startPhase)) {
    return null;
  }
  const normalizedHistoryDepthYears = normalizeHistoryDepthYears(
    historyDepthYears,
  );
  if (normalizedHistoryDepthYears === null) {
    return null;
  }

  return {
    startYear,
    startPhase: formData.startPhase,
    historyDepthYears: normalizedHistoryDepthYears,
  };
}

type IsoDateParts = {
  year: number;
  month: number;
  day: number;
};

function parseIsoDateParts(isoDob: string): IsoDateParts | null {
  if (!isoDob) return null;

  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(isoDob);
  if (!match) return null;

  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const birthDate = new Date(Date.UTC(year, month - 1, day));

  if (
    Number.isNaN(birthDate.getTime()) ||
    birthDate.getUTCFullYear() !== year ||
    birthDate.getUTCMonth() !== month - 1 ||
    birthDate.getUTCDate() !== day
  ) {
    return null;
  }

  return { year, month, day };
}

function careerStartReferenceDate(
  startYear: number,
  startPhase: CareerStartPhase,
): Date {
  const referenceDate = new Date(Date.UTC(startYear, 6, 1));
  if (startPhase === "midSeason") {
    referenceDate.setUTCDate(referenceDate.getUTCDate() + 120);
  }
  return referenceDate;
}

function flooredAgeFromIsoDate(
  isoDob: string,
  referenceDate: Date,
): number | null {
  const parts = parseIsoDateParts(isoDob);
  if (!parts) return null;

  let age = referenceDate.getUTCFullYear() - parts.year;
  const hasHadBirthdayThisYear =
    referenceDate.getUTCMonth() > parts.month - 1 ||
    (referenceDate.getUTCMonth() === parts.month - 1 &&
      referenceDate.getUTCDate() >= parts.day);

  if (!hasHadBirthdayThisYear) {
    age -= 1;
  }
  return Number.isNaN(age) ? null : age;
}

function dobValidationMessage(
  formData: CreateManagerFormData,
  historyDepthYears: number,
  t: (key: string, options?: Record<string, unknown>) => string,
): string | null {
  if (!formData.dob) return null;

  if (parseIsoDateParts(formData.dob) === null) {
    return t("validation.invalidDate");
  }

  const startupOptions = buildStartupOptions(formData, historyDepthYears);
  if (!startupOptions) return null;

  const age = flooredAgeFromIsoDate(
    formData.dob,
    careerStartReferenceDate(startupOptions.startYear, startupOptions.startPhase),
  );
  if (age === null) return t("validation.invalidDate");
  if (age < MANAGER_MINIMUM_AGE) {
    return t("validation.minAge", { min: MANAGER_MINIMUM_AGE });
  }
  if (age > 99) return t("validation.invalidDob");
  return null;
}

const CREATE_MANAGER_FIELD_ORDER = [
  "firstName",
  "lastName",
  "dob",
  "startYear",
  "startPhase",
  "nationality",
] as const satisfies ReadonlyArray<keyof CreateManagerFormData>;

function prefersReducedMotion(): boolean {
  if (typeof window === "undefined") return false;
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function deferFocusToNextPaint(callback: () => void): void {
  requestAnimationFrame(() => {
    requestAnimationFrame(callback);
  });
}

function focusFirstCreateManagerError(
  errors: Partial<Record<keyof CreateManagerFormData, string>>,
): void {
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

function MenuPanelFallback() {
  return (
    <div className="flex min-h-64 items-center justify-center">
      <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary-500 border-t-transparent" />
    </div>
  );
}

export default function MainMenu() {
  const navigate = useNavigate();
  const setGameActive = useGameStore((state) => state.setGameActive);
  const setGameState = useGameStore((state) => state.setGameState);
  const { t } = useTranslation();

  const [menuState, setMenuState] = useState<
    "main" | "create" | "world" | "load"
  >("main");
  const [showProfileConfirm, setShowProfileConfirm] = useState(false);
  const [saves, setSaves] = useState<SaveEntry[]>([]);
  const [isLoadingSaves, setIsLoadingSaves] = useState(false);
  const [loadingSaveId, setLoadingSaveId] = useState<string | null>(null);
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const [isStarting, setIsStarting] = useState(false);

  const [profiles, setProfiles] = useState<ManagerProfile[]>([]);
  const [loadedProfile, setLoadedProfile] = useState<ManagerProfile | null>(null);

  const [formData, setFormData] = useState<CreateManagerFormData>({
    firstName: "",
    lastName: "",
    dob: "",
    startYear: defaultCareerStartYear(),
    startPhase: "seasonStart",
    nationality: "",
  });
  const [formErrors, setFormErrors] = useState<
    Partial<Record<keyof CreateManagerFormData, string>>
  >({});

  // World database state
  const [worldDatabases, setWorldDatabases] = useState<WorldDatabaseInfo[]>([]);
  const [selectedWorldId, setSelectedWorldId] = useState<string>("random");
  const [isLoadingWorlds, setIsLoadingWorlds] = useState(false);
  const [historyDepthYears, setHistoryDepthYears] = useState(
    initialHistoryDepthYears,
  );

  useEffect(() => {
    window.localStorage.setItem(
      GENERATED_HISTORY_DEPTH_STORAGE_KEY,
      String(historyDepthYears),
    );
  }, [historyDepthYears]);

  useEffect(() => {
    invoke<ManagerProfile[]>("get_manager_profiles")
      .then((p) => setProfiles(p ?? []))
      .catch((error) => console.error("Failed to load manager profiles:", error));
  }, []);

  // Check if a game is already active (e.g. loaded by MCP --mcp-auto-start before frontend mounted)
  useEffect(() => {
    invoke<GameStateData>("get_active_game")
      .then((state) => {
        const mgrName = `${state.manager.first_name} ${state.manager.last_name}`;
        setGameState(state);
        setGameActive(true, mgrName);
        navigate("/dashboard");
      })
      .catch(() => {
        // No active game — stay on menu
      });
  }, [setGameState, setGameActive, navigate]);

  // Listen for game loaded by MCP auto-start (event may arrive after mount)
  useEffect(() => {
    const unlisten = listen("game-state-changed", async () => {
      try {
        const state = await invoke<GameStateData>("get_active_game");
        const mgrName = `${state.manager.first_name} ${state.manager.last_name}`;
        setGameState(state);
        setGameActive(true, mgrName);
        navigate("/dashboard");
      } catch {
        // Game not actually active — ignore
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [setGameState, setGameActive, navigate]);

  /** Same messages as `validateForm` for DOB, so the age rule surfaces as the user edits. */
  const dobLiveRuleMessage = dobValidationMessage(formData, historyDepthYears, t);
  const dobDisplayedError = formErrors.dob || dobLiveRuleMessage;

  const updateFormField = (field: keyof CreateManagerFormData, value: string) => {
    setFormData((previous) => ({
      ...previous,
      [field]: value,
    }));
  };

  const clearFormError = (field: keyof CreateManagerFormData) => {
    setFormErrors((previous) => ({
      ...previous,
      [field]: "",
    }));
  };

  const validateForm = (): {
    ok: boolean;
    errors: Partial<Record<keyof CreateManagerFormData, string>>;
  } => {
    const errors: Partial<Record<keyof CreateManagerFormData, string>> = {};
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
      const dobError = dobValidationMessage(formData, historyDepthYears, t);
      if (dobError) {
        errors.dob = dobError;
      }
    }
    if (!formData.startYear.trim()) {
      errors.startYear = t("validation.required", {
        field: t("createManager.startYear"),
      });
    } else {
      const startYear = parseCareerStartYear(formData.startYear);
      if (startYear === null || startYear < MIN_CAREER_START_YEAR) {
        errors.startYear = t("validation.minStartYear", {
          min: MIN_CAREER_START_YEAR,
        });
      }
    }
    if (!isCareerStartPhase(formData.startPhase)) {
      errors.startPhase = t("validation.required", {
        field: t("createManager.startPhase"),
      });
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
      deferFocusToNextPaint(() => focusFirstCreateManagerError(validation.errors));
      return;
    }
    if (loadedProfile && formDiffersFromProfile(formData, loadedProfile)) {
      setShowProfileConfirm(true);
      return;
    }
    void autoSaveProfile();
    proceedToWorldSelect();
  };

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
          history_mode: "generated",
          base_year: null,
          snapshot_date: null,
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
        const path = await invoke<string>("write_temp_database", { json });
        const info: WorldDatabaseInfo = {
          id: `file:${file.name}`,
          name: parsed.name || file.name.replace(".json", ""),
          description: parsed.description || t("menu.importedDescription"),
          team_count: parsed.teams?.length ?? 0,
          player_count: parsed.players?.length ?? 0,
          history_mode: historyModeFromMetadata(parsed.metadata) ?? "hybrid",
          base_year:
            typeof parsed.metadata?.base_year === "number"
              ? parsed.metadata.base_year
              : null,
          snapshot_date:
            typeof parsed.metadata?.snapshot_date === "string"
              ? parsed.metadata.snapshot_date
              : null,
          source: "imported",
          path,
        };
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
    const startupOptions = buildStartupOptions(formData, historyDepthYears);
    if (!startupOptions) {
      const validation = validateForm();
      setMenuState("create");
      deferFocusToNextPaint(() =>
        focusFirstCreateManagerError(validation.errors),
      );
      return;
    }

    setIsStarting(true);
    try {
      // Determine world source
      let worldSource: string | undefined = selectedWorldId;
      if (selectedWorldId === "random") {
        worldSource = undefined;
      } else {
        const selectedDb = worldDatabases.find((db) => db.id === selectedWorldId);
        if (selectedDb?.path) {
          worldSource = `file:${selectedDb.path}`;
        }
      }

      const game = await invoke<GameStateData>("start_new_game", {
        firstName: formData.firstName,
        lastName: formData.lastName,
        dob: formData.dob,
        nationality: formData.nationality,
        startupOptions,
        worldSource,
      });
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

  const handleSelectProfile = (profile: ManagerProfile) => {
    setFormData((prev) => ({
      ...prev,
      firstName: profile.first_name,
      lastName: profile.last_name,
      dob: profile.date_of_birth,
      nationality: profile.nationality,
    }));
    setFormErrors({});
    setLoadedProfile(profile);
    void invoke("touch_manager_profile", { id: profile.id });
  };

  const formDiffersFromProfile = (form: CreateManagerFormData, profile: ManagerProfile) =>
    form.firstName !== profile.first_name ||
    form.lastName !== profile.last_name ||
    form.dob !== profile.date_of_birth ||
    form.nationality !== profile.nationality;

  const proceedToWorldSelect = () => {
    setShowProfileConfirm(false);
    setMenuState("world");
    loadWorldDatabases();
  };

  const handleUpdateProfile = async () => {
    if (!loadedProfile) return;
    try {
      const updated = await invoke<ManagerProfile | null>("update_manager_profile", {
        id: loadedProfile.id,
        firstName: formData.firstName,
        lastName: formData.lastName,
        dob: formData.dob,
        nationality: formData.nationality,
      });
      if (updated) {
        setProfiles((prev) => prev.map((p) => (p.id === updated.id ? updated : p)));
        setLoadedProfile(updated);
      }
    } catch (error) {
      console.error("Failed to update manager profile:", error);
    }
    proceedToWorldSelect();
  };

  const handleSaveAsNewProfile = () => {
    void autoSaveProfile(true);
    setLoadedProfile(null);
    proceedToWorldSelect();
  };

  const handleDeleteProfile = async (id: string) => {
    try {
      await invoke<boolean>("delete_manager_profile", { id });
      setProfiles((prev) => prev.filter((p) => p.id !== id));
      if (loadedProfile?.id === id) setLoadedProfile(null);
    } catch (error) {
      console.error("Failed to delete manager profile:", error);
    }
  };

  const autoSaveProfile = async (forceNew = false) => {
    try {
      const saved = await invoke<ManagerProfile>("save_manager_profile", {
        firstName: formData.firstName,
        lastName: formData.lastName,
        dob: formData.dob,
        nationality: formData.nationality,
        force: forceNew || undefined,
      });
      setProfiles((prev) => {
        const exists = prev.some((p) => p.id === saved.id);
        const next =
          !forceNew && exists
            ? prev.map((p) => (p.id === saved.id ? saved : p))
            : [...prev, saved];
        return next.sort((a, b) => {
          const aDate = a.last_used_at ?? a.created_at;
          const bDate = b.last_used_at ?? b.created_at;
          return bDate.localeCompare(aDate);
        });
      });
    } catch (error) {
      console.error("Failed to auto-save manager profile:", error);
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
            alt={t("app.name")}
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
            <Suspense fallback={<MenuPanelFallback />}>
              <CreateManagerForm
                formData={formData}
                formErrors={formErrors}
                dobError={dobDisplayedError}
                profiles={profiles}
                selectedProfileId={loadedProfile?.id}
                onChange={updateFormField}
                onClearError={clearFormError}
                onClose={() => {
                  setMenuState("main");
                  setFormErrors({});
                  setLoadedProfile(null);
                }}
                onSelectProfile={handleSelectProfile}
                onDeleteProfile={handleDeleteProfile}
                onSubmit={handleGoToWorldSelect}
              />
            </Suspense>
          )}

          {/* Profile save confirmation modal */}
          {showProfileConfirm && loadedProfile && (
            <Suspense fallback={null}>
              <ProfileSaveConfirm
                loadedProfile={loadedProfile}
                onUpdate={() => { void handleUpdateProfile(); }}
                onSaveNew={() => { void handleSaveAsNewProfile(); }}
                onSkip={proceedToWorldSelect}
                onClose={() => setShowProfileConfirm(false)}
              />
            </Suspense>
          )}

          {/* Step 2: World Database Selection */}
          {menuState === "world" && (
            <Suspense fallback={<MenuPanelFallback />}>
              <WorldSelect
                worldDatabases={worldDatabases}
                selectedWorldId={selectedWorldId}
                isLoadingWorlds={isLoadingWorlds}
                isStarting={isStarting}
                startYear={parseCareerStartYear(formData.startYear) ?? MIN_CAREER_START_YEAR}
                startPhase={formData.startPhase}
                historyDepthYears={historyDepthYears}
                onSelectWorld={setSelectedWorldId}
                onChangeHistoryDepthYears={setHistoryDepthYears}
                onImportFile={handleImportFile}
                onStart={handleStartGame}
                onBack={() => setMenuState("create")}
                onClose={() => setMenuState("main")}
              />
            </Suspense>
          )}

          {/* Load Game List */}
          {menuState === "load" && (
            <Suspense fallback={<MenuPanelFallback />}>
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
            </Suspense>
          )}

        </div>
      </div>

      {/* Community links */}
      <div className="absolute bottom-3 left-4 flex items-center gap-1">
        <button
          type="button"
          aria-label={t("menu.openDiscord")}
          title={t("menu.openDiscord")}
          onClick={() => { void openUrl(DISCORD_INVITE_URL); }}
          className="p-1.5 rounded-lg text-gray-400 dark:text-gray-600 hover:text-[#5865F2] dark:hover:text-[#7289DA] hover:bg-gray-100 dark:hover:bg-navy-700 transition-colors"
        >
          <DiscordIcon className="w-5 h-5" />
        </button>
        <button
          type="button"
          aria-label={t("menu.openGithub")}
          title={t("menu.openGithub")}
          onClick={() => { void openUrl(GITHUB_REPO_URL); }}
          className="p-1.5 rounded-lg text-gray-400 dark:text-gray-600 hover:text-gray-900 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-navy-700 transition-colors"
        >
          <GithubIcon className="w-5 h-5" />
        </button>
      </div>

      {/* Version */}
      <div className="absolute bottom-4 right-4 text-gray-400 dark:text-gray-600 text-xs font-heading uppercase tracking-widest transition-colors">
        {t("app.version")}
      </div>
    </div>
  );
}
