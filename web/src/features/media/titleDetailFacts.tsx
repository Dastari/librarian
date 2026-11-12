import { useMemo, type ReactNode } from "react";

import { TitleDetailsForDocument, type TitleDetailWhereInput } from "@/graphql/generated/graphql";
import { useLiveQuery } from "@/lib/graphql/live/hooks";
import { contentRatings, mainTrailer, preferredRating, videos } from "@/lib/titleDetails";

const link = "nav-focus rounded text-brand hover:underline";

/**
 * Hero facts from a title's details (Q130): studios or networks, its rating in the viewer's
 * country and a link to its trailer. Empty until the details have been read.
 */
export function useTitleDetailFacts(where: TitleDetailWhereInput, { skip, networks = false }: { skip?: boolean; networks?: boolean } = {}) {
  const query = useLiveQuery(TitleDetailsForDocument, { variables: { where }, skip });
  const detail = query.data?.titleDetails.edges[0]?.node;
  return useMemo(() => {
    if (!detail) return { facts: [] as Array<{ label: string; value: ReactNode }>, rating: null as string | null, detail };
    const rating = preferredRating(contentRatings(detail.contentRatings));
    const trailer = mainTrailer(videos(detail.videos));
    const facts: Array<{ label: string; value: ReactNode }> = [
      { label: networks ? "Network" : "Studio", value: (networks && detail.networks.length ? detail.networks : detail.studios).slice(0, 3).join(", ") || null },
      ...(networks && detail.networks.length && detail.studios.length ? [{ label: "Studio", value: detail.studios.slice(0, 3).join(", ") }] : []),
      { label: "Rated", value: rating ? `${rating.rating} (${rating.country})` : null },
      { label: "Trailer", value: trailer?.url ? <a className={link} href={trailer.url} target="_blank" rel="noreferrer">{trailer.name || "Watch"} · {trailer.site}</a> : null },
    ];
    return { facts, rating: rating?.rating ?? null, detail };
  }, [detail, networks]);
}
