import { useApolloClient, useMutation } from "@apollo/client/react";
import type { ApolloClient } from "@apollo/client";
import type { ActionDefinition } from "glass-ux/actions";
import { IconRotateCcw } from "glass-ux/icons";
import { useCallback } from "react";

import { EntityPlaybackProgressDeleteManyDocument, type PlaybackProgressFieldsFragment } from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { assertSuccess } from "@/lib/graphql/errors";

/** `PlaybackProgressForFiles` reads at most 100 rows, and a short `inList` keeps each delete small. */
export const RESET_BATCH = 100;

/** What reads of a person's saved progress look like, whichever query supplied them. */
export type SavedProgress = Pick<PlaybackProgressFieldsFragment, "isWatched" | "currentPosition">;

/** Some progress is a finished mark (also what "mark as watched" leaves) or a saved position. */
export function hasSavedProgress(saved: SavedProgress | null | undefined): boolean {
  return Boolean(saved && (saved.isWatched || saved.currentPosition > 0));
}

/** The files, once each and in order, where this person has progress worth resetting. */
export function filesWithProgress(fileIds: Iterable<string | null | undefined>, progressByFile: ReadonlyMap<string | null | undefined, SavedProgress>): string[] {
  const found = new Set<string>();
  for (const id of fileIds) if (id && hasSavedProgress(progressByFile.get(id))) found.add(id);
  return [...found];
}

/**
 * Where a title's page or row tells an action set which files hold this person's progress. It is
 * only given where progress is loaded, so a poster that never read it doesn't offer the action.
 */
export interface ProgressReset<T> {
  /** Files holding saved progress for these rows; none means there is nothing to reset. */
  files: (rows: readonly T[]) => readonly string[];
  /** Why it can't run yet: the list of files, or the progress itself, is still loading. */
  disabledReason?: (rows: readonly T[]) => string | false | null | undefined;
}

/** The filter for one batch. It always names this person, so an administrator's reset can't touch anyone else's rows. */
export function resetWhere(userId: string, mediaFileIds: readonly string[]) {
  return { userId: { eq: userId }, mediaFileId: { inList: [...mediaFileIds] } };
}

const PROGRESS_READERS = new Set(["PlaybackProgressForFile", "PlaybackProgressForFiles", "PlaybackProgressOwners", "ResumeShelf", "ContentStatuses"]);

/** Refetch every reader of progress that is on screen: the per-file queries, the home shelf and computed statuses. */
export function refetchProgressReaders(client: ApolloClient): Promise<unknown> {
  const active = [...client.getObservableQueries("active").values()].filter((query) => query.queryName && PROGRESS_READERS.has(query.queryName));
  return Promise.allSettled(active.map((query) => query.refetch()));
}

/**
 * Forgets this person's progress on files: deletes their `PlaybackProgress` rows through the
 * generated bulk mutation. The server only lets a member remove their own rows; the filter names
 * them anyway, because an administrator could otherwise remove everyone's.
 */
export function useResetProgress(): (mediaFileIds: readonly string[]) => Promise<void> {
  const client = useApolloClient();
  const { user } = useSession();
  const userId = user?.id;
  const [deleteMany] = useMutation(EntityPlaybackProgressDeleteManyDocument);
  return useCallback(async (mediaFileIds) => {
    if (!userId) throw new Error("Sign in to change your watch history");
    const files = [...new Set(mediaFileIds)];
    if (files.length === 0) return;
    try {
      for (let at = 0; at < files.length; at += RESET_BATCH) {
        const { data } = await deleteMany({ variables: { where: resetWhere(userId, files.slice(at, at + RESET_BATCH)) } });
        assertSuccess(data?.deletePlaybackProgresses, "Could not reset progress");
      }
    } finally {
      // Live updates follow the deletes; this makes the screen right as soon as the call returns,
      // and after a partial failure shows what is really left.
      await refetchProgressReaders(client);
    }
  }, [client, deleteMany, userId]);
}

export type ProgressMedia = "video" | "audio";

/** Wording follows the medium: films and episodes are watched, music and books are played. */
export const resetLabel = (media: ProgressMedia) => (media === "video" ? "Mark as unwatched" : "Mark as unplayed");

/**
 * One "Mark as unwatched" / "Mark as unplayed" action for an action set. It is hidden until the
 * rows have progress, and disabled while the page is still loading what it would reset. It sits
 * in the menu (no header button), beside the other things you can do to the title.
 */
export function resetProgressAction<T>(media: ProgressMedia, reset: ProgressReset<T> | undefined, run: (mediaFileIds: readonly string[]) => Promise<void>, options: { id?: string; label?: string } = {}): ActionDefinition<T>[] {
  if (!reset) return [];
  return [{
    id: options.id ?? "reset-progress",
    label: options.label ?? resetLabel(media),
    icon: IconRotateCcw,
    group: "manage",
    bulk: true,
    visible: (rows) => reset.files(rows).length > 0,
    disabledReason: (rows) => reset.disabledReason?.(rows) ?? false,
    run: (rows) => run(reset.files(rows)),
    successMessage: () => (media === "video" ? "Marked as unwatched" : "Marked as unplayed"),
  }];
}
