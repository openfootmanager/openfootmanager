import { useEffect } from "react";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";

import Dashboard from "../../pages/Dashboard";
import { useGameStore } from "../../store/gameStore";
import { HARNESS_PAGES, type HarnessPage } from "./harnessPage";

/** Shown when the page navigates away, which a failed load does, so a capture can't pass blank. */
function LeftTheDashboard() {
  const location = useLocation();
  return (
    <p role="alert" className="p-4 text-body text-danger">
      The page left the dashboard for {location.pathname}: a command it needs is probably
      unanswered.
    </p>
  );
}

/**
 * Mounts the real dashboard on the fixture career. The career is marked active the way loading a
 * save marks it, and every backend call goes to the mocked IPC installed before this rendered.
 */
export function PageHarness({ page, managerName }: { page: HarnessPage; managerName: string }) {
  const hasActiveGame = useGameStore((state) => state.hasActiveGame);
  useEffect(() => {
    useGameStore.getState().setGameActive(true, managerName);
  }, [managerName]);

  if (!hasActiveGame) return null;
  return (
    <MemoryRouter initialEntries={["/dashboard"]}>
      <Routes>
        <Route path="/dashboard" element={<Dashboard initialTab={HARNESS_PAGES[page]} />} />
        <Route path="*" element={<LeftTheDashboard />} />
      </Routes>
    </MemoryRouter>
  );
}
