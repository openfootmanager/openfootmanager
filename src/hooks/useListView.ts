import { type ListView, useSettingsStore } from "../store/settingsStore";

/**
 * The Table/Cards choice every list shares: the global `list_view` setting and the way to change
 * it. A list binds its `ViewSwitch` to this, so a pick on one list is how every list opens.
 */
export function useListView(): [ListView, (view: ListView) => void] {
  const view = useSettingsStore((state) => state.settings.list_view);
  const updateSettings = useSettingsStore((state) => state.updateSettings);
  return [view, (next) => void updateSettings({ list_view: next })];
}
