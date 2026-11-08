import { useCallback, useState } from "react";

import { useIsAdmin } from "@/lib/auth/useSession";

import { ShowSettingsDialog } from "./ShowSettingsDialog";
import type { ShowEntity } from "./ShowIdentity";
import type { ProgressReset } from "./progressReset";
import { useShowActions, type ShowSettingsTab } from "./useShowActions";

/** A show's actions, with its settings dialog ready to open from them. */
export function useShowEntity<T extends ShowEntity>(options: { open?: boolean; play?: { label: string; run: () => void }; season?: number; onRefreshed?: () => void; reset?: ProgressReset<T> } = {}) {
  const isAdmin = useIsAdmin();
  const [settings, setSettings] = useState<{ id: string; tab: ShowSettingsTab } | null>(null);
  const openSettings = useCallback((show: T, tab?: ShowSettingsTab) => setSettings({ id: show.id, tab: tab ?? (isAdmin ? "general" : "info") }), [isAdmin]);
  const { actions, dialogs } = useShowActions<T>({ ...options, settings: openSettings });
  return {
    actions,
    openSettings,
    dialogs: (
      <>
        {dialogs}
        {settings ? <ShowSettingsDialog showId={settings.id} open tab={settings.tab} onTabChange={(tab) => setSettings({ id: settings.id, tab })} onOpenChange={(next) => { if (!next) setSettings(null); }} /> : null}
      </>
    ),
  };
}
