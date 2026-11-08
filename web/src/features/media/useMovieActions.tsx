import { useApolloClient, useMutation } from "@apollo/client/react";
import { useNavigate } from "@tanstack/react-router";
import { defineActions, type ActionSet } from "glass-ux/actions";
import { IconBookmark, IconDownload, IconExternalLink, IconPlay, IconRefresh, IconSettings, IconTrash } from "glass-ux/icons";
import { useMemo, useState, type ReactNode } from "react";

import { ReleaseSearchDialog } from "@/features/downloads/ReleaseSearchDialog";
import { playlistTitle, useAddToPlaylist } from "@/features/personal/useAddToPlaylist";
import { useQueueActions } from "@/features/player/queueActions";
import { usePlayer } from "@/features/player/usePlayer";
import { EntityMovieUpdateDocument, RecacheMovieArtworkDocument, RefreshMovieDocument } from "@/graphql/generated/graphql";
import { movieBackdrop, moviePoster } from "@/lib/artwork";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { activeContentStatusQueries } from "@/lib/graphql/content-status";
import { assertSuccess } from "@/lib/graphql/errors";

import type { MovieEntity } from "./MovieIdentity";
import { resetProgressAction, useResetProgress, type ProgressReset } from "./progressReset";

export const MOVIE_SETTINGS_TABS = ["general", "tags", "downloads", "files", "info", "remove"] as const;
export type MovieSettingsTab = (typeof MOVIE_SETTINGS_TABS)[number];

/**
 * Everything that can be done to a movie, wherever it's shown: its page, a poster's right-click, a
 * table row's menu, its settings dialog. `settings` opens the dialog at a section. `open` is left
 * out on its page, and `play` lets the page resume where someone stopped.
 */
export function useMovieActions<T extends MovieEntity>({ settings, open = true, play, playLabel, reset }: {
  settings: (movie: T, tab?: MovieSettingsTab) => void;
  open?: boolean;
  play?: (movie: T) => void;
  playLabel?: string;
  /** Where this person's progress is known (its page), which adds "Mark as unwatched". */
  reset?: ProgressReset<T>;
}): { actions: ActionSet<T>; dialogs: ReactNode } {
  const client = useApolloClient();
  const navigate = useNavigate();
  const player = usePlayer();
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  const [update] = useMutation(EntityMovieUpdateDocument, { refetchQueries: () => activeContentStatusQueries(client) });
  const [refresh, { loading: refreshing }] = useMutation(RefreshMovieDocument);
  const [recache] = useMutation(RecacheMovieArtworkDocument);
  const resetProgress = useResetProgress();
  const playlist = useAddToPlaylist<T>({
    targetsOf: (rows) => rows.map((row) => ({ kind: "movie" as const, id: row.id })),
    titleOf: (rows) => playlistTitle(rows, rows[0]!.title, { one: "movie", other: "movies" }),
    allowed: (rows) => Boolean(user) && rows.every((row) => row.userId === user?.id),
    bulk: true,
    priority: 2,
  });
  // Play next / Add to queue (the play queue): one movie, or every selected movie that has a file.
  const queueActions = useQueueActions<T>((rows) => rows.filter((movie) => movie.mediaFileId).map((movie) => ({ kind: "movie" as const, id: movie.id })), { visible: (rows) => rows.some((movie) => movie.mediaFileId) });
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
      ...queueActions,
      ...(open ? [{
        id: "open", label: "Open movie", icon: IconExternalLink, group: "open" as const, priority: 1,
        visible: one,
        run: (rows: readonly T[]) => void navigate({ to: "/movies/$movieId", params: { movieId: rows[0]!.id } }),
      }] : []),
      playlist.action,
      ...resetProgressAction("video", reset, resetProgress),
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
  }, [isAdmin, navigate, open, play, playLabel, player, queueActions, recache, refresh, refreshing, reset, resetProgress, settings, update, user?.id]);

  const dialogs = (
    <>
      {playlist.dialog}
      {finding ? <ReleaseSearchDialog isOpen onOpenChange={(next) => { if (!next) setFinding(null); }} query={finding.title} year={finding.year} imdbId={finding.imdbId} libraryId={finding.libraryId} target={{ movieId: finding.id }} /> : null}
    </>
  );
  return { actions, dialogs };
}
