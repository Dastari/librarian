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

/** The Want / Stop wanting pair for a book's or album's missing chapters or tracks. Admins only. */
export function wantMissingActions<T extends { id: string }>(
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
      run: run(false),
      successMessage: () => `Missing ${noun} no longer wanted`,
    },
  ];
}
