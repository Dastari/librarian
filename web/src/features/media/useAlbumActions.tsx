import { useNavigate } from "@tanstack/react-router";
import { defineActions, type ActionSet } from "glass-ux/actions";
import { IconBookmark, IconDownload, IconExternalLink, IconPlay, IconSettings, IconTrash } from "glass-ux/icons";
import { useMemo, useState, type ReactNode } from "react";

import { ReleaseSearchDialog } from "@/features/downloads/ReleaseSearchDialog";
import { useQueueActions } from "@/features/player/queueActions";
import { SaveToListDialog } from "@/features/personal/SaveToListDialog";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";

import type { AlbumEntity } from "./AlbumIdentity";
import { resetProgressAction, useResetProgress, type ProgressReset } from "./progressReset";
import { useWantMissing, wantMissingActions } from "./wantMissing";

export const ALBUM_SETTINGS_TABS = ["general", "downloads", "files", "info", "remove"] as const;
export type AlbumSettingsTab = (typeof ALBUM_SETTINGS_TABS)[number];

/** Everything that can be done to an album, wherever it's shown. `play` needs its page's tracks. */
export function useAlbumActions<T extends AlbumEntity>({ settings, open = true, play, artist, reset, onChanged }: {
  settings: (album: T, tab?: AlbumSettingsTab) => void;
  open?: boolean;
  play?: { label: string; run: () => void; disabledReason?: string };
  /** Who a release search looks for, where it's known. */
  artist?: string | null;
  /** Where this person's progress is known (its page), which adds "Mark as unplayed". */
  reset?: ProgressReset<T>;
  /** After a change its page should reload for, such as wanting its missing tracks. */
  onChanged?: () => void;
}): { actions: ActionSet<T>; dialogs: ReactNode } {
  const navigate = useNavigate();
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  const resetProgress = useResetProgress();
  const wantMissing = useWantMissing();
  // Play next / Add to queue (the play queue): every track of each selected album, in order.
  const queueActions = useQueueActions<T>((rows) => rows.map((album) => ({ kind: "album" as const, id: album.id })));
  const [saving, setSaving] = useState<T | null>(null);
  const [finding, setFinding] = useState<T | null>(null);

  const actions = useMemo(() => {
    const one = (rows: readonly T[]) => rows.length === 1;
    return defineActions<T>("album", [
      ...(play ? [{
        id: "play", label: play.label, icon: IconPlay, group: "open" as const, priority: 0, surfaces: ["menu" as const, "context" as const, "palette" as const],
        visible: one,
        disabledReason: () => play.disabledReason ?? false,
        run: () => play.run(),
      }] : []),
      ...queueActions,
      ...(open ? [{
        id: "open", label: "Open album", icon: IconExternalLink, group: "open" as const, priority: 1,
        visible: one,
        run: (rows: readonly T[]) => void navigate({ to: "/albums/$albumId", params: { albumId: rows[0]!.id } }),
      }] : []),
      {
        id: "save", label: "Save to list", icon: IconBookmark, group: "open", priority: 2,
        visible: (rows) => one(rows) && rows[0]!.userId === user?.id,
        run: (rows) => setSaving(rows[0]!),
      },
      ...resetProgressAction("audio", reset, resetProgress),
      ...(isAdmin ? [{
        id: "find", label: "Find releases", icon: IconDownload, group: "manage" as const, priority: 2, primary: !play,
        visible: one,
        run: (rows: readonly T[]) => setFinding(rows[0]!),
      }, ...wantMissingActions<T>("tracks", wantMissing, onChanged)] : []),
      {
        id: "settings", label: isAdmin ? "Album settings" : "Details", icon: IconSettings, group: "manage", priority: 4,
        visible: one,
        run: (rows) => settings(rows[0]!),
      },
      ...(isAdmin ? [{
        id: "remove", label: "Remove from library…", icon: IconTrash, group: "danger" as const, priority: 9,
        visible: one,
        run: (rows: readonly T[]) => settings(rows[0]!, "remove"),
      }] : []),
    ]);
  }, [isAdmin, navigate, onChanged, open, play, queueActions, reset, resetProgress, settings, user?.id, wantMissing]);

  const dialogs = (
    <>
      {saving ? <SaveToListDialog open onOpenChange={(next) => { if (!next) setSaving(null); }} target={{ kind: "album", id: saving.id }} title={saving.name} /> : null}
      {finding ? <ReleaseSearchDialog isOpen onOpenChange={(next) => { if (!next) setFinding(null); }} query={[artist, finding.name].filter(Boolean).join(" ")} year={finding.year} libraryId={finding.libraryId} artist={artist} album={finding.name} target={{ albumId: finding.id }} /> : null}
    </>
  );
  return { actions, dialogs };
}
