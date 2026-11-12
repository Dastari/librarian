import { useLiveRoute } from "@/lib/graphql/live/hooks";
import { libraryTrail } from "@/components/shell/Breadcrumbs";
import { createFileRoute } from "@tanstack/react-router";

import { ShowPage } from "@/features/media/ShowPage";
import { ShowDetailDocument } from "@/graphql/generated/graphql";

/** A link can open a show at one of its episodes. */
interface ShowSearch {
  season?: number;
  episode?: string;
}

export const Route = createFileRoute("/_app/shows/$showId")({
  validateSearch: (search: Record<string, unknown>): ShowSearch => ({
    season: typeof search.season === "number" ? search.season : undefined,
    episode: typeof search.episode === "string" ? search.episode : undefined,
  }),
    loader: async ({ context, params }) => {
    const { data } = await context.apollo.query({ query: ShowDetailDocument, variables: { id: params.showId }, fetchPolicy: "cache-first" });
    return { crumb: data?.show?.name ?? "Show", trail: libraryTrail(data?.show?.library, "shows") };
  },
  component: () => {
    const { showId } = Route.useParams();
    const { season, episode } = Route.useSearch();
    useLiveRoute(ShowDetailDocument, { id: showId });
    return <ShowPage showId={showId} initialSeason={season} focusEpisodeId={episode} />;
  },
});
