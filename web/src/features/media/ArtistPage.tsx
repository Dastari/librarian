import { useQuery } from "@apollo/client/react";
import { Link } from "@tanstack/react-router";

import { ErrorState, MetaLine, PosterCard, Section, SkeletonHero } from "@/components/ui";
import { ArtistDetailDocument } from "@/graphql/generated/graphql";
import { albumCover, artistImage } from "@/lib/artwork";
import { formatRuntime, formatYear, pluralize } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { DetailShell } from "./DetailShell";

export function ArtistPage({ artistId }: { artistId: string }) {
  const { data, previousData, loading, error, refetch } = useQuery(ArtistDetailDocument, { variables: { id: artistId } });
  const artist = (data ?? previousData)?.artist ?? null;
  if (error && !artist) return <ErrorState error={error} onRetry={() => void refetch()} className="m-8" />;
  if (!artist) return loading ? <SkeletonHero /> : null;
  const albums = artist.albums.edges.map((edge) => edge.node);

  return (
    <DetailShell
      hero={{
        backdrop: artistImage(artist),
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
