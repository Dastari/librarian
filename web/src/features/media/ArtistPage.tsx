import { IconInfo } from "glass-ux/icons";
import { useCallback } from "react";
import { useApolloClient } from "@apollo/client/react";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { Link } from "@tanstack/react-router";

import { Button, EmptyState, ErrorState, MetaLine, PosterCard, Section, SkeletonHero } from "@/components/ui";
import { ArtistDetailDocument } from "@/graphql/generated/graphql";
import { albumCover, artistImage } from "@/lib/artwork";
import { formatRuntime, formatYear, pluralize } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { DetailShell } from "./DetailShell";
import { useAudioPages } from "./useAudioPages";

export function ArtistPage({ artistId }: { artistId: string }) {
  const client = useApolloClient();
  const { data, previousData, loading, error, refetch } = useQuery(ArtistDetailDocument, { variables: { id: artistId }, context: { liveManaged: true } });
  const candidate = (data ?? previousData)?.artist;
  const artist = candidate?.id === artistId ? candidate : null;
  const loadPage = useCallback(async (offset: number) => {
    const result = await client.query({ query: ArtistDetailDocument, variables: { id: artistId, page: { limit: 100, offset } }, fetchPolicy: "no-cache" });
    if (!result.data?.artist) throw new Error("Could not load the remaining albums");
    return result.data.artist.albums;
  }, [client, artistId]);
  const pages = useAudioPages(artistId, artist?.albums, loadPage, { document: ArtistDetailDocument, variables: { id: artistId }, refreshHead: refetch });
  if (error && !artist) return <ErrorState error={error} onRetry={() => void refetch()} className="m-8" />;
  if (!artist) return loading ? <SkeletonHero /> : <EmptyState icon={IconInfo} title="Artist not found" description="This item may have been removed from the library." className="page-gutter my-8" />;
  const albums = pages.rows;

  return (
    <DetailShell
      backdrop={null}
      hero={{
        poster: artistImage(artist),
        posterAspect: "square",
        tint: LIBRARY_TYPES.music.tintVar,
        eyebrow: artist.library ? (
          <Link to="/libraries/$libraryId/artists" params={{ libraryId: artist.library.id }} className="nav-focus rounded hover:underline">
            {artist.library.name}
          </Link>
        ) : null,
        title: artist.name,
        meta: <MetaLine items={[pluralize(artist.albumCount ?? albums.length, "album"), pluralize(artist.trackCount ?? 0, "track"), formatRuntime(artist.totalDurationSecs), artist.disambiguation]} />,
        description: artist.bio,
      }}
    >
      {pages.error ? <p role="status">Could not load all albums. <Button size="sm" onPress={pages.retry}>Retry</Button></p> : pages.loading ? <p role="status">Loading remaining albums…</p> : null}
      <Section title="Albums">
        <div className="poster-grid">
          {albums.map((album) => (
            <PosterCard key={album.id} aspect="square" title={album.name} meta={[formatYear(album.releaseDate) || album.year, album.albumType].filter(Boolean).join(" · ")} image={albumCover(album)} tint={LIBRARY_TYPES.music.tintVar} to="/albums/$albumId" params={{ albumId: album.id }} />
          ))}
        </div>
      </Section>
    </DetailShell>
  );
}
