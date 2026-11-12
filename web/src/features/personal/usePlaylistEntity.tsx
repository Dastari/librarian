import { useCallback, useState } from "react";

import type { PlaylistEntity } from "./PlaylistIdentity";
import { PlaylistSettingsDialog, type PlaylistSettingsTab } from "./PlaylistSettingsDialog";
import { usePlaylistActions } from "./usePlaylistActions";

/** A playlist's actions, with its settings dialog ready to open from them. */
export function usePlaylistEntity<T extends PlaylistEntity>(options: { open?: boolean } = {}) {
  const [settings, setSettings] = useState<{ id: string; tab: PlaylistSettingsTab } | null>(null);
  const openSettings = useCallback((list: T, tab?: PlaylistSettingsTab) => setSettings({ id: list.id, tab: tab ?? "general" }), []);
  const actions = usePlaylistActions<T>({ ...options, settings: openSettings });
  return {
    actions,
    openSettings,
    dialogs: settings ? <PlaylistSettingsDialog listId={settings.id} open tab={settings.tab} onTabChange={(tab) => setSettings({ id: settings.id, tab })} onOpenChange={(next) => { if (!next) setSettings(null); }} /> : null,
  };
}
