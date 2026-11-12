import { useCallback, useState } from "react";

import { useIsAdmin } from "@/lib/auth/useSession";

import type { AudiobookEntity } from "./AudiobookIdentity";
import { AudiobookSettingsDialog } from "./AudiobookSettingsDialog";
import type { ProgressReset } from "./progressReset";
import { useAudiobookActions, type AudiobookSettingsTab } from "./useAudiobookActions";

/** A book's actions, with its settings dialog ready to open from them. */
export function useAudiobookEntity<T extends AudiobookEntity>(options: { open?: boolean; play?: { label: string; run: () => void; disabledReason?: string }; onChanged?: () => void; reset?: ProgressReset<T> } = {}) {
  const isAdmin = useIsAdmin();
  const [settings, setSettings] = useState<{ id: string; tab: AudiobookSettingsTab } | null>(null);
  const openSettings = useCallback((book: T, tab?: AudiobookSettingsTab) => setSettings({ id: book.id, tab: tab ?? (isAdmin ? "general" : "info") }), [isAdmin]);
  const { actions, dialogs } = useAudiobookActions<T>({ ...options, settings: openSettings });
  return {
    actions,
    openSettings,
    dialogs: (
      <>
        {dialogs}
        {settings ? <AudiobookSettingsDialog audiobookId={settings.id} open tab={settings.tab} onTabChange={(tab) => setSettings({ id: settings.id, tab })} onOpenChange={(next) => { if (!next) setSettings(null); }} /> : null}
      </>
    ),
  };
}
