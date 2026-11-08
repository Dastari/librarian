import { useMutation } from "@apollo/client/react";
import { defineActions, type ActionSet } from "glass-ux/actions";
import { IconEdit, IconExternalLink, IconPin, IconPlay, IconSettings, IconShuffle, IconTrash } from "glass-ux/icons";
import { useMemo } from "react";

import { EntityPersonalListDeleteDocument } from "@/graphql/generated/graphql";
import { assertSuccess } from "@/lib/graphql/errors";

import type { PlaylistEntity } from "./PlaylistIdentity";
import type { PlaylistSettingsTab } from "./PlaylistSettingsDialog";
import { usePlaylistPlayback } from "./usePlaylistPlayback";
import { useHomeShelves } from "./usePersonalLibrary";

/**
 * Everything that can be done to a playlist, wherever it's shown: its header, a table row's menu,
 * a card's right-click. `settings` opens its dialog at a section. `open` is left out on its page.
 * Smart collections have no entries to play, so they offer their rules instead.
 */
export function usePlaylistActions<T extends PlaylistEntity>({ settings, open = true }: {
  settings: (list: T, tab?: PlaylistSettingsTab) => void;
  open?: boolean;
}): ActionSet<T> {
  const playback = usePlaylistPlayback();
  const home = useHomeShelves();
  const [remove] = useMutation(EntityPersonalListDeleteDocument);
  return useMemo(() => {
    const one = (rows: readonly T[]) => rows.length === 1;
    const playable = (rows: readonly T[]) => one(rows) && rows[0]!.kind !== "SMART";
    const shelf = (id: string) => home.rows.find((row) => row.listId === id && row.visible);
    return defineActions<T>("playlist", [
      {
        id: "play", label: "Play", icon: IconPlay, group: "open", priority: 0, primary: true,
        visible: playable, run: (rows) => playback.playPlaylist(rows[0]!.id),
      },
      {
        id: "shuffle", label: "Shuffle", icon: IconShuffle, group: "open", priority: 1,
        visible: playable, run: (rows) => playback.shufflePlaylist(rows[0]!.id),
      },
      ...(open ? [{
        id: "open", label: "Open playlist", icon: IconExternalLink, group: "open" as const, priority: 2,
        visible: one, run: (rows: readonly T[], context: { navigate: (href: string) => void }) => context.navigate(`/lists/${rows[0]!.id}`),
      }] : []),
      {
        id: "pin", label: (rows) => (shelf(rows[0]!.id) ? "Unpin from Home" : "Pin to Home"), icon: IconPin, group: "organise", priority: 3,
        visible: one,
        disabledReason: () => (home.complete ? false : "Loading Home"),
        run: (rows) => {
          const current = shelf(rows[0]!.id);
          return home.save(`list:${rows[0]!.id}`, { position: current?.position ?? 1000 + home.rows.length * 10, visible: !current, listId: rows[0]!.id });
        },
        successMessage: (rows) => (shelf(rows[0]!.id) ? "Unpinned from Home" : "Pinned to Home"),
      },
      {
        id: "rename", label: "Rename…", icon: IconEdit, group: "manage", priority: 4,
        visible: (rows) => one(rows) && rows[0]!.kind === "MANUAL", run: (rows) => settings(rows[0]!),
      },
      {
        id: "rules", label: "Edit rules…", icon: IconEdit, group: "manage", priority: 4,
        visible: (rows) => one(rows) && rows[0]!.kind === "SMART", run: (rows) => settings(rows[0]!, "rules"),
      },
      {
        id: "settings", label: "Playlist settings", icon: IconSettings, group: "manage", priority: 5,
        visible: one, run: (rows) => settings(rows[0]!),
      },
      {
        id: "remove", label: (rows) => (rows.length === 1 ? "Delete…" : `Delete ${rows.length} playlists…`), icon: IconTrash, group: "danger", priority: 9, bulk: true,
        confirm: (rows) => rows.length > 1 ? { title: `Delete ${rows.length} playlists?`, body: "Only the playlists go. Media and download settings stay as they are.", confirmLabel: "Delete playlists", tone: "danger" } : null,
        run: async (rows, context) => {
          if (rows.length === 1) { settings(rows[0]!, "remove"); return; }
          for (const row of rows) assertSuccess((await remove({ variables: { id: row.id } })).data?.deletePersonalList, `Could not delete ${row.name}`);
          context.navigate("/lists");
        },
      },
    ]);
  }, [home.complete, home.rows, home.save, open, playback, remove, settings]);
}
