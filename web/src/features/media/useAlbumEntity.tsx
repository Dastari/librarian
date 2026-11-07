import { useCallback, useState } from "react";

import { useIsAdmin } from "@/lib/auth/useSession";

import type { AlbumEntity } from "./AlbumIdentity";
import { AlbumSettingsDialog } from "./AlbumSettingsDialog";
import { useAlbumActions, type AlbumSettingsTab } from "./useAlbumActions";

/** An album's actions, with its settings dialog ready to open from them. */
export function useAlbumEntity<T extends AlbumEntity>(options: { open?: boolean; play?: { label: string; run: () => void; disabledReason?: string }; enqueue?: () => void; artist?: string | null } = {}) {
  const isAdmin = useIsAdmin();
  const [settings, setSettings] = useState<{ id: string; tab: AlbumSettingsTab } | null>(null);
  const openSettings = useCallback((album: T, tab?: AlbumSettingsTab) => setSettings({ id: album.id, tab: tab ?? (isAdmin ? "general" : "info") }), [isAdmin]);
  const { actions, dialogs } = useAlbumActions<T>({ ...options, settings: openSettings });
  return {
    actions,
    openSettings,
    dialogs: (
      <>
        {dialogs}
        {settings ? <AlbumSettingsDialog albumId={settings.id} open tab={settings.tab} onTabChange={(tab) => setSettings({ id: settings.id, tab })} onOpenChange={(next) => { if (!next) setSettings(null); }} /> : null}
      </>
    ),
  };
}
