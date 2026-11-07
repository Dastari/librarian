import { useCallback, useState } from "react";

import { useIsAdmin } from "@/lib/auth/useSession";

import type { MovieEntity } from "./MovieIdentity";
import { MovieSettingsDialog } from "./MovieSettingsDialog";
import { useMovieActions, type MovieSettingsTab } from "./useMovieActions";

/** A movie's actions, with its settings dialog ready to open from them. */
export function useMovieEntity<T extends MovieEntity>(options: { open?: boolean; play?: (movie: T) => void; playLabel?: string } = {}) {
  const isAdmin = useIsAdmin();
  const [settings, setSettings] = useState<{ id: string; tab: MovieSettingsTab } | null>(null);
  const openSettings = useCallback((movie: T, tab?: MovieSettingsTab) => setSettings({ id: movie.id, tab: tab ?? (isAdmin ? "general" : "info") }), [isAdmin]);
  const { actions, dialogs } = useMovieActions<T>({ ...options, settings: openSettings });
  return {
    actions,
    openSettings,
    dialogs: (
      <>
        {dialogs}
        {settings ? <MovieSettingsDialog movieId={settings.id} open tab={settings.tab} onTabChange={(tab) => setSettings({ id: settings.id, tab })} onOpenChange={(next) => { if (!next) setSettings(null); }} /> : null}
      </>
    ),
  };
}
