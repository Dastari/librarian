import { useMutation } from "@apollo/client/react";
import { useNavigate } from "@tanstack/react-router";
import { defineActions, type ActionSet } from "glass-ux/actions";
import { IconBookmark, IconDownload, IconExternalLink, IconPlay, IconRefresh, IconSettings, IconTrash } from "glass-ux/icons";
import { useMemo, useState, type ReactNode } from "react";

import { ReleaseSearchDialog } from "@/features/downloads/ReleaseSearchDialog";
import { SaveToListDialog } from "@/features/personal/SaveToListDialog";
import { RefreshShowDocument } from "@/graphql/generated/graphql";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { assertSuccess } from "@/lib/graphql/errors";

import type { ShowEntity } from "./ShowIdentity";

export const SHOW_SETTINGS_TABS = ["general", "downloads", "files", "info", "remove"] as const;
export type ShowSettingsTab = (typeof SHOW_SETTINGS_TABS)[number];

/**
 * Everything that can be done to a show, wherever it's shown. `play` is the next episode, which
 * only its page knows; `season` is the one a release search starts from.
 */
export function useShowActions<T extends ShowEntity>({ settings, open = true, play, season, onRefreshed }: {
  settings: (show: T, tab?: ShowSettingsTab) => void;
  open?: boolean;
  play?: { label: string; run: () => void };
  season?: number;
  onRefreshed?: () => void;
}): { actions: ActionSet<T>; dialogs: ReactNode } {
  const navigate = useNavigate();
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  const [refresh, { loading: refreshing }] = useMutation(RefreshShowDocument);
  const [saving, setSaving] = useState<T | null>(null);
  const [finding, setFinding] = useState<T | null>(null);

  const actions = useMemo(() => {
    const one = (rows: readonly T[]) => rows.length === 1;
    return defineActions<T>("show", [
      ...(play ? [{
        id: "play", label: play.label, icon: IconPlay, group: "open" as const, priority: 0, surfaces: ["menu" as const, "context" as const, "palette" as const],
        visible: one,
        run: () => play.run(),
      }] : []),
      ...(open ? [{
        id: "open", label: "Open show", icon: IconExternalLink, group: "open" as const, priority: 1,
        visible: one,
        run: (rows: readonly T[]) => void navigate({ to: "/shows/$showId", params: { showId: rows[0]!.id } }),
      }] : []),
      {
        id: "save", label: "Save to list", icon: IconBookmark, group: "open", priority: 2,
        visible: (rows) => one(rows) && rows[0]!.userId === user?.id,
        run: (rows) => setSaving(rows[0]!),
      },
      ...(isAdmin ? [
        {
          id: "find", label: "Find releases", icon: IconDownload, group: "manage" as const, priority: 2, primary: !play,
          visible: one,
          run: (rows: readonly T[]) => setFinding(rows[0]!),
        },
        {
          id: "refresh", label: "Refresh metadata", icon: IconRefresh, group: "manage" as const, priority: 3,
          visible: one,
          disabledReason: () => (refreshing ? "Already refreshing" : false),
          run: async (rows: readonly T[]) => {
            assertSuccess((await refresh({ variables: { id: rows[0]!.id } })).data?.refreshShow, "Could not refresh metadata");
            onRefreshed?.();
          },
          successMessage: () => "Metadata refreshed",
        },
      ] : []),
      {
        id: "settings", label: isAdmin ? "Show settings" : "Details", icon: IconSettings, group: "manage", priority: 4,
        visible: one,
        run: (rows) => settings(rows[0]!),
      },
      ...(isAdmin ? [{
        id: "remove", label: "Remove from library…", icon: IconTrash, group: "danger" as const, priority: 9,
        visible: one,
        run: (rows: readonly T[]) => settings(rows[0]!, "remove"),
      }] : []),
    ]);
  }, [isAdmin, navigate, onRefreshed, open, play, refresh, refreshing, settings, user?.id]);

  const dialogs = (
    <>
      {saving ? <SaveToListDialog open onOpenChange={(next) => { if (!next) setSaving(null); }} target={{ kind: "show", id: saving.id }} title={saving.name} /> : null}
      {finding ? <ReleaseSearchDialog isOpen onOpenChange={(next) => { if (!next) setFinding(null); }} query={finding.name} imdbId={finding.imdbId} season={season} libraryId={finding.libraryId} target={{ showId: finding.id }} /> : null}
    </>
  );
  return { actions, dialogs };
}
