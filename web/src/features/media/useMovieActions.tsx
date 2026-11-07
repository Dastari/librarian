import { useApolloClient, useMutation } from "@apollo/client/react";
import { useNavigate } from "@tanstack/react-router";
import { defineActions, type ActionSet } from "glass-ux/actions";
import { IconBookmark, IconDownload, IconExternalLink, IconPlay, IconRefresh, IconSettings, IconTrash } from "glass-ux/icons";
import { useMemo, useState, type ReactNode } from "react";

import { ReleaseSearchDialog } from "@/features/downloads/ReleaseSearchDialog";
import { SaveToListDialog } from "@/features/personal/SaveToListDialog";
import { usePlayer } from "@/features/player/usePlayer";
import { EntityMovieUpdateDocument, RecacheMovieArtworkDocument, RefreshMovieDocument } from "@/graphql/generated/graphql";
import { movieBackdrop, moviePoster } from "@/lib/artwork";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { activeContentStatusQueries } from "@/lib/graphql/content-status";
import { assertSuccess } from "@/lib/graphql/errors";

import type { MovieEntity } from "./MovieIdentity";

export const MOVIE_SETTINGS_TABS = ["general", "tags", "downloads", "files", "info", "remove"] as const;
export type MovieSettingsTab = (typeof MOVIE_SETTINGS_TABS)[number];

/**
 * Everything that can be done to a movie, wherever it's shown: its page, a poster's right-click, a
 * table row's menu, its settings dialog. `settings` opens the dialog at a section. `open` is left
 * out on its page, and `play` lets the page resume where someone stopped.
 */
export function useMovieActions<T extends MovieEntity>({ settings, open = true, play, playLabel }: {
  settings: (movie: T, tab?: MovieSettingsTab) => void;
  open?: boolean;
  play?: (movie: T) => void;
  playLabel?: string;
}): { actions: ActionSet<T>; dialogs: ReactNode } {
  const client = useApolloClient();
  const navigate = useNavigate();
  const player = usePlayer();
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  const [update] = useMutation(EntityMovieUpdateDocument, { refetchQueries: () => activeContentStatusQueries(client) });
  const [refresh, { loading: refreshing }] = useMutation(RefreshMovieDocument);
  const [recache] = useMutation(RecacheMovieArtworkDocument);
  const [saving, setSaving] = useState<T | null>(null);
  const [finding, setFinding] = useState<T | null>(null);

  const actions = useMemo(() => {
    const one = (rows: readonly T[]) => rows.length === 1;
    return defineActions<T>("movie", [
      {
        id: "play", label: playLabel ?? "Play", icon: IconPlay, group: "open", priority: 0, surfaces: ["menu", "context", "palette"],
        visible: (rows) => one(rows) && Boolean(rows[0]!.mediaFileId),
        run: (rows) => {
          const movie = rows[0]!;
          if (play) return play(movie);
          player.playVideo({ mediaFileId: movie.mediaFileId!, title: movie.title, subtitle: movie.year ? String(movie.year) : undefined, artwork: moviePoster(movie.id), backdrop: movieBackdrop(movie.id), entity: { kind: "movie", id: movie.id }, href: `/movies/${movie.id}` });
          void navigate({ to: "/watch/$mediaFileId", params: { mediaFileId: movie.mediaFileId! } });
        },
      },
      ...(open ? [{
        id: "open", label: "Open movie", icon: IconExternalLink, group: "open" as const, priority: 1,
        visible: one,
        run: (rows: readonly T[]) => void navigate({ to: "/movies/$movieId", params: { movieId: rows[0]!.id } }),
      }] : []),
      {
        id: "save", label: "Save to list", icon: IconBookmark, group: "open", priority: 2,
        visible: (rows) => one(rows) && rows[0]!.userId === user?.id,
        run: (rows) => setSaving(rows[0]!),
      },
      ...(isAdmin ? [
        {
          id: "wanted", label: (rows: readonly T[]) => (rows[0]!.wanted ? "Remove from Wanted" : "Add to Wanted"), icon: IconBookmark, group: "manage" as const, priority: 1,
          visible: one,
          run: async (rows: readonly T[]) => {
            const movie = rows[0]!;
            assertSuccess((await update({ variables: { id: movie.id, input: { wanted: !movie.wanted, monitored: !movie.wanted } } })).data?.updateMovie, "Could not update the movie");
          },
        },
        {
          id: "find", label: "Find releases", icon: IconDownload, group: "manage" as const, priority: 2, primary: true,
          visible: (rows: readonly T[]) => one(rows) && !rows[0]!.hasFile,
          run: (rows: readonly T[]) => setFinding(rows[0]!),
        },
        {
          id: "refresh", label: "Refresh metadata", icon: IconRefresh, group: "manage" as const, priority: 3,
          visible: one,
          disabledReason: () => (refreshing ? "Already refreshing" : false),
          run: async (rows: readonly T[]) => {
            const movie = rows[0]!;
            assertSuccess((await refresh({ variables: { id: movie.id } })).data?.refreshMovie, "Could not refresh metadata");
            await recache({ variables: { movieId: movie.id } });
          },
          successMessage: () => "Metadata refreshed",
        },
      ] : []),
      {
        id: "settings", label: isAdmin ? "Movie settings" : "Details", icon: IconSettings, group: "manage", priority: 4,
        visible: one,
        run: (rows) => settings(rows[0]!),
      },
      ...(isAdmin ? [{
        id: "remove", label: "Remove from library…", icon: IconTrash, group: "danger" as const, priority: 9,
        visible: one,
        run: (rows: readonly T[]) => settings(rows[0]!, "remove"),
      }] : []),
    ]);
  }, [isAdmin, navigate, open, play, playLabel, player, recache, refresh, refreshing, settings, update, user?.id]);

  const dialogs = (
    <>
      {saving ? <SaveToListDialog open onOpenChange={(next) => { if (!next) setSaving(null); }} target={{ kind: "movie", id: saving.id }} title={saving.title} /> : null}
      {finding ? <ReleaseSearchDialog isOpen onOpenChange={(next) => { if (!next) setFinding(null); }} query={finding.title} year={finding.year} imdbId={finding.imdbId} libraryId={finding.libraryId} target={{ movieId: finding.id }} /> : null}
    </>
  );
  return { actions, dialogs };
}
