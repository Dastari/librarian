import { defineActions } from "glass-ux/actions";
import { IconInfo } from "glass-ux/icons";
import { useCallback, useMemo, useState } from "react";
import { useApolloClient } from "@apollo/client/react";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { Link } from "@tanstack/react-router";

import { Button, EmptyState, ErrorState, MetaLine, PosterCard, Section, SkeletonHero } from "@/components/ui";
import { useArtworkAction } from "@/features/artwork/useArtworkAction";
import { ArtistDetailDocument, type ArtistDetailQuery } from "@/graphql/generated/graphql";
import { useIsAdmin } from "@/lib/auth/useSession";
import { albumCover, artistImage } from "@/lib/artwork";
import { formatRuntime, formatYear, pluralize } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { ReleaseSearchDialog } from "@/features/downloads/ReleaseSearchDialog";
import { FindReleasesButton } from "@/features/recommendations/FindReleasesButton";
import { useRecommended } from "@/features/recommendations/useRecommended";

import { DetailShell } from "./DetailShell";
import { MediaActions } from "./MediaActions";
import { useAudioPages } from "./useAudioPages";

type Artist = NonNullable<ArtistDetailQuery["artist"]>;

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
  const isAdmin = useIsAdmin();
  // An artist's own actions: its picture is the one thing to change here.
  const artwork = useArtworkAction<Artist>({ targetOf: (row) => ({ entityType: "ARTIST", entityId: row.id, title: row.name }), allowed: () => isAdmin });
  const actions = useMemo(() => defineActions<Artist>("artist", [artwork.action]), [artwork.action]);
  const [finding, setFinding] = useState(false);
  // The artist's coverage plan: discography packs, albums, and the smallest release for a lone track.
  // What is missing is the server's to work out; it answers with no plan when nothing is.
  const recommended = useRecommended({ artistId }, artist ? 1 : 0, isAdmin && Boolean(artist));
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
        actions: isAdmin ? <><MediaActions target={artist} label={artist.name} actions={actions} /><FindReleasesButton variant="primary" count={recommended.forTarget} onPress={() => setFinding(true)} /></> : undefined,
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
      {artwork.dialog}
      {finding ? <ReleaseSearchDialog isOpen onOpenChange={(open) => { if (!open) setFinding(false); }} query={artist.name} libraryId={artist.library?.id ?? null} artist={artist.name} target={{ artistId: artist.id }} /> : null}
    </DetailShell>
  );
}
