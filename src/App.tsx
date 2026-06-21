import { useEffect, lazy, Suspense } from "react";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { useSettingsStore } from "./store/settingsStore";
import i18n, { changeAppLanguage } from "./i18n";
import "./App.css";

const MainMenu = lazy(() => import("./pages/MainMenu"));
const TeamSelection = lazy(() => import("./pages/TeamSelection"));
const Dashboard = lazy(() => import("./pages/Dashboard"));
const MatchSimulation = lazy(() => import("./pages/MatchSimulation"));
const Settings = lazy(() => import("./pages/Settings"));

function LazyFallback() {
  return (
    <div className="min-h-screen bg-gray-100 dark:bg-navy-900 flex items-center justify-center">
      <div className="w-8 h-8 border-4 border-primary-500 border-t-transparent rounded-full animate-spin" />
    </div>
  );
}

const SCALE_MAP: Record<string, string> = {
  small: "14px",
  normal: "16px",
  large: "18px",
  xlarge: "20px",
};

function App() {
  const { settings, loaded, loadSettings } = useSettingsStore();

  useEffect(() => {
    if (!loaded) loadSettings();
  }, [loaded, loadSettings]);

  useEffect(() => {
    const size = SCALE_MAP[settings.ui_scale] || "16px";
    document.documentElement.style.fontSize = size;
  }, [settings.ui_scale]);

  useEffect(() => {
    document.documentElement.classList.toggle(
      "high-contrast",
      settings.high_contrast,
    );
  }, [settings.high_contrast]);

  // Apply saved language from settings once loaded (overrides OS detection)
  useEffect(() => {
    if (loaded && settings.language && settings.language !== i18n.language) {
      void changeAppLanguage(settings.language);
    }
  }, [loaded, settings.language]);

  return (
    <BrowserRouter>
      <Suspense fallback={<LazyFallback />}>
        <Routes>
          <Route path="/" element={<MainMenu />} />
          <Route path="/select-team" element={<TeamSelection />} />
          <Route path="/dashboard" element={<Dashboard />} />
          <Route path="/match" element={<MatchSimulation />} />
          <Route path="/settings" element={<Settings />} />
        </Routes>
      </Suspense>
    </BrowserRouter>
  );
}

export default App;
