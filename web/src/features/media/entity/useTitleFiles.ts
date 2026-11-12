import { useMemo } from "react";

import { TitleFileCountDocument, TitleFileOwnersDocument, type MediaFileWhereInput } from "@/graphql/generated/graphql";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

export type TitleWithParts = "show" | "album" | "audiobook";

/** A title's files are the files of its episodes, tracks or chapters: how to ask for them, and how many. */
export function useTitleFiles(kind: TitleWithParts, id: string, skip = false) {
  const owners = useQuery(TitleFileOwnersDocument, {
    skip,
    variables: {
      showId: kind === "show" ? id : "",
      albumId: kind === "album" ? id : "",
      audiobookId: kind === "audiobook" ? id : "",
      isShow: kind === "show",
      isAlbum: kind === "album",
      isBook: kind === "audiobook",
    },
  });
  const scope = useMemo<MediaFileWhereInput | null>(() => {
    if (!owners.data) return null;
    if (kind === "show") return { episodeId: { inList: (owners.data.episodes?.edges ?? []).map(({ node }) => node.id) } };
    if (kind === "album") return { trackId: { inList: (owners.data.tracks?.edges ?? []).map(({ node }) => node.id) } };
    return { chapterId: { inList: (owners.data.chapters?.edges ?? []).map(({ node }) => node.id) } };
  }, [kind, owners.data]);
  const counted = useQuery(TitleFileCountDocument, { variables: { where: scope ?? {} }, skip: !scope });
  return { scope, count: counted.data?.mediaFiles.pageInfo.totalCount ?? undefined };
}
