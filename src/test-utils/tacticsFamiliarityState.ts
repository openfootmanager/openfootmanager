/**
 * The players the stand-in for the backend's familiarity grades in the Tactics tab's suites, and
 * whether it has arrived (every player the harness builds is registered here). Its own module so a
 * suite's `vi.mock` factory can read it without importing the harness, which imports the tab.
 */
export const familiarityState = {
  arrived: true,
  players: new Map<string, unknown>(),
};
