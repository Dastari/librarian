import { useApolloClient, useMutation } from "@apollo/client/react";
import { IconBookmark } from "glass-ux/icons";
import { useCallback } from "react";

import { EntityChapterUpdateManyDocument, EntityTrackUpdateManyDocument } from "@/graphql/generated/graphql";
import { activeContentStatusQueries } from "@/lib/graphql/content-status";
import { assertSuccess } from "@/lib/graphql/errors";

export type WantMissingKind = "chapters" | "tracks";

/**
 * Want (or stop wanting) every chapter or track a book or album has no file for, across one or
 * more of them: the whole-title version of a row's Want. Rows with files are never touched.
 */
export function useWantMissing() {
  const client = useApolloClient();
  const refetchQueries = () => activeContentStatusQueries(client);
  const [updateChapters] = useMutation(EntityChapterUpdateManyDocument, { refetchQueries });
  const [updateTracks] = useMutation(EntityTrackUpdateManyDocument, { refetchQueries });
  return useCallback(async (kind: WantMissingKind, ownerIds: readonly string[], wanted: boolean) => {
    const ids = { inList: [...ownerIds] };
    if (kind === "chapters") {
      const { data } = await updateChapters({ variables: { where: { audiobookId: ids, mediaFileId: { isNull: true } }, input: { wanted } } });
      return assertSuccess(data?.updateChapters, "Could not update the chapters").affectedCount;
    }
    const { data } = await updateTracks({ variables: { where: { albumId: ids, mediaFileId: { isNull: true } }, input: { wanted } } });
    return assertSuccess(data?.updateTracks, "Could not update the tracks").affectedCount;
  }, [updateChapters, updateTracks]);
}

/** What a row says about its wanted missing parts: `wantedMissing` is a count read with the row (see the list and detail queries). */
export interface WantedMissing {
  wantedMissing?: { pageInfo: { totalCount?: number | null } } | null;
}

/** How many of a title's missing parts are wanted, or undefined where the row didn't read it. */
const wantedCount = (row: WantedMissing) => row.wantedMissing?.pageInfo.totalCount ?? undefined;

/**
 * The Want / Stop wanting pair for a book's or album's missing chapters or tracks. Admins only.
 * Stop wanting is only offered where something is wanted; a row that never read the count keeps it.
 */
export function wantMissingActions<T extends { id: string } & WantedMissing>(
  kind: WantMissingKind,
  wantMissing: ReturnType<typeof useWantMissing>,
  onChanged?: () => void,
) {
  const noun = kind;
  const run = (wanted: boolean) => async (rows: readonly T[]) => {
    await wantMissing(kind, rows.map((row) => row.id), wanted);
    onChanged?.();
  };
  return [
    {
      id: "want-missing", label: `Want missing ${noun}`, icon: IconBookmark, group: "manage" as const, priority: 2, bulk: true,
      run: run(true),
      successMessage: () => `Missing ${noun} wanted`,
    },
    {
      id: "unwant-missing", label: `Stop wanting missing ${noun}`, icon: IconBookmark, group: "manage" as const, priority: 2, bulk: true,
      visible: (rows: readonly T[]) => rows.some((row) => (wantedCount(row) ?? 1) > 0),
      run: run(false),
      successMessage: () => `Missing ${noun} no longer wanted`,
    },
  ];
}
