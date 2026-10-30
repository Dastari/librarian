import { IconInfo } from "glass-ux/icons";
import { useApolloClient, useMutation } from "@apollo/client/react";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { toast } from "glass-ux/toast";
import { Link, useNavigate } from "@tanstack/react-router";
import { IconInfoCircle, IconPhoto } from "@tabler/icons-react";
import { useState } from "react";

import { Artwork, ConfirmDialog, EmptyState, ErrorState, HeroFacts, KeyValueList, MediaRow, MetaLine, Panel, PosterCard, Section, SkeletonHero, StatusChip } from "@/components/ui";
import { AcquisitionChip } from "@/features/acquisition/AcquisitionChip";
import { AcquisitionDialog } from "@/features/acquisition/AcquisitionDialog";
import { movieAcquisitionMeta } from "@/features/acquisition/mode";
import { usePlayer } from "@/features/player/usePlayer";
import { ReleaseSearchDialog } from "@/features/downloads/ReleaseSearchDialog";
import {
  EntityMovieDeleteDocument,
  EntityMovieUpdateDocument,
  MovieCastCreditsDocument,
  MovieCollectionDetailsDocument,
  MovieDetailDocument,
  PlaybackProgressForFileDocument,
  RecacheMovieArtworkDocument,
  RefreshMovieDocument,
} from "@/graphql/generated/graphql";
import { useContentStatuses } from "@/hooks/useContentStatuses";
import { movieBackdrop, moviePoster, personProfile } from "@/lib/artwork";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { formatClock, formatDate, formatRuntime } from "@/lib/format";
import { activeContentStatusQueries } from "@/lib/graphql/content-status";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPES } from "@/lib/library-types";
import { statusMeta } from "@/lib/status";

import { MediaActions } from "./MediaActions";
import { DetailShell } from "./DetailShell";
import { FileDetails } from "./FileDetails";

export function MoviePage({ movieId }: { movieId: string }) {
  const client = useApolloClient();
  const navigate = useNavigate();
  const player = usePlayer();
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  const { data, previousData, loading, error, refetch } = useQuery(MovieDetailDocument, { variables: { id: movieId } });
  const candidate = (data ?? previousData)?.movie;
  const movie = candidate?.id === movieId ? candidate : null;
  const cast = useQuery(MovieCastCreditsDocument, { variables: { movieId } });
  const collection = useQuery(MovieCollectionDetailsDocument, { variables: { collectionId: movie?.collectionId ?? 0, libraryId: movie?.libraryId ?? "" }, skip: !movie?.collectionId || !movie.libraryId });
  const progress = useQuery(PlaybackProgressForFileDocument, { variables: { userId: user?.id ?? "", mediaFileId: movie?.mediaFileId ?? "" }, skip: !user || !movie?.mediaFileId });
  const statuses = useContentStatuses("MOVIE", movie ? [movie.id] : []);

  const [updateMovie] = useMutation(EntityMovieUpdateDocument, { refetchQueries: () => activeContentStatusQueries(client) });
  const [deleteMovie, { loading: deleting }] = useMutation(EntityMovieDeleteDocument);
  const [refreshMovie, { loading: refreshing }] = useMutation(RefreshMovieDocument);
  const [recache] = useMutation(RecacheMovieArtworkDocument);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [searching, setSearching] = useState(false);
  const [acquisition, setAcquisition] = useState(false);

  if (error && !movie) return <ErrorState error={error} onRetry={() => void refetch()} className="m-8" />;
  if (!movie) return loading ? <SkeletonHero /> : <EmptyState icon={IconInfo} title="Movie not found" description="This item may have been removed from the library." className="page-gutter my-8" />;

  const resume = progress.data?.playbackProgresses.edges[0]?.node;
  const resumeAt = resume && !resume.isWatched && resume.currentPosition > 5 ? resume.currentPosition : undefined;
  const status = statusMeta(statuses.get(movie.id));

  const play = () => {
    if (!movie.mediaFileId) return;
    player.playVideo({ mediaFileId: movie.mediaFileId, title: movie.title, subtitle: movie.year ? String(movie.year) : undefined, artwork: moviePoster(movie.id), entity: { kind: "movie", id: movie.id }, href: `/movies/${movie.id}`, startPosition: resumeAt });
    void navigate({ to: "/watch/$mediaFileId", params: { mediaFileId: movie.mediaFileId } });
  };

  const toggleWanted = async () => {
    const { data: result } = await updateMovie({ variables: { id: movie.id, input: { wanted: !movie.wanted, monitored: !movie.wanted } } });
    assertSuccess(result?.updateMovie, "Could not update movie");
  };

  const refresh = async () => {
    const { data: result } = await refreshMovie({ variables: { id: movie.id } });
    assertSuccess(result?.refreshMovie, "Could not refresh metadata");
    await recache({ variables: { movieId: movie.id } });
  };

  const remove = async () => {
    try {
      const { data: result } = await deleteMovie({ variables: { id: movie.id } });
      assertSuccess(result?.deleteMovie, "Could not remove movie");
      toast.success("Movie removed from the library");
      await navigate({ to: "/libraries/$libraryId/movies", params: { libraryId: movie.libraryId } });
    } catch (mutationError) {
      toast.danger(errorMessage(mutationError));
    }
  };

  const castList = cast.data?.movieCastCredits.edges.map((edge) => edge.node) ?? [];
  const collectionMovies = collection.data?.movieCollectionDetails.movies ?? [];

  return (
    <DetailShell
      backdrop={movieBackdrop(movie.id)}
      hero={{
        poster: moviePoster(movie.id),
        tint: LIBRARY_TYPES.movies.tintVar,
        eyebrow: movie.library ? (
          <Link to="/libraries/$libraryId/movies" params={{ libraryId: movie.library.id }} className="nav-focus rounded hover:underline">
            {movie.library.name}
          </Link>
        ) : null,
        title: movie.title,
        meta: (
          <>
            <MetaLine items={[movie.year, movie.runtime ? formatRuntime(movie.runtime, "minutes") : null, movie.certification, movie.tmdbRating ? `★ ${Number(movie.tmdbRating).toFixed(1)}` : null]} />
            <StatusChip status={status} />
            <AcquisitionChip status={movieAcquisitionMeta(movie)} onPress={isAdmin ? () => setAcquisition(true) : undefined} />
          </>
        ),
        actions: (
          <MediaActions
            personal={movie.userId === user?.id ? { kind: "movie", id: movie.id } : undefined}
            id={movie.id}
            label={movie.title}
            play={movie.mediaFileId ? { label: resumeAt ? `Resume · ${formatClock(resumeAt)}` : "Play", run: play } : undefined}
            wanted={isAdmin ? { value: movie.wanted, run: toggleWanted } : undefined}
            find={isAdmin && !movie.hasFile ? () => setSearching(true) : undefined}
            settings={isAdmin ? () => setAcquisition(true) : undefined}
            refresh={isAdmin ? { pending: refreshing, run: refresh } : undefined}
            remove={isAdmin ? () => setConfirmDelete(true) : undefined}
          />
        ),
        tagline: movie.tagline,
        description: movie.overview,
        facts: <HeroFacts items={[{ label: "Director", value: movie.director }, { label: "Genres", value: movie.genres.join(", ") }]} />,
      }}
    >
      {castList.length > 0 ? (
        <Section title="Cast" bleed>
          <MediaRow ariaLabel="Cast">
            {castList.map((credit) => (
              <Link key={credit.id} to="/search" search={{ q: credit.person?.name ?? "" }} data-focusable className="tv-lift nav-focus group w-28 shrink-0 rounded-tile p-1 text-center" aria-label={[credit.person?.name, credit.characterName].filter(Boolean).join(" as ")}>
                <Artwork src={personProfile(credit.person)} alt="" aspect="square" displayWidth={96} className="tv-lift-target mx-auto w-24 overflow-clip rounded-full" fallback={<IconPhoto size={24} />} />
                <p className="mt-2 truncate text-label text-foreground">{credit.person?.name}</p>
                <p className="truncate text-label-sm text-muted">{credit.characterName}</p>
              </Link>
            ))}
          </MediaRow>
        </Section>
      ) : null}

      {collectionMovies.length > 1 ? (
        <Section title={collection.data?.movieCollectionDetails.name ?? movie.collectionName ?? "Collection"} bleed trailing={<span>{collectionMovies.filter((item) => item.libraryMovieId).length} of {collectionMovies.length} in library</span>}>
          <MediaRow ariaLabel="Collection">
            {collectionMovies.map((item) =>
              item.libraryMovieId ? (
                <PosterCard key={item.tmdbId} width="row" title={item.title} meta={item.year ?? undefined} image={moviePoster(item.libraryMovieId)} tint={LIBRARY_TYPES.movies.tintVar} to="/movies/$movieId" params={{ movieId: item.libraryMovieId }} selected={item.libraryMovieId === movie.id} />
              ) : (
                <PosterCard key={item.tmdbId} width="row" title={item.title} meta={item.year ? `${item.year} · not in library` : "Not in library"} image={item.posterUrl} tint={LIBRARY_TYPES.movies.tintVar} className="opacity-70" />
              ),
            )}
          </MediaRow>
        </Section>
      ) : null}

      <div className="grid items-start gap-6 lg:grid-cols-[minmax(0,2fr)_minmax(0,3fr)]">
        <Panel title="Details">
          <KeyValueList
            items={[
              { label: "Released", value: formatDate(movie.releaseDate) },
              { label: "Original title", value: movie.originalTitle !== movie.title ? movie.originalTitle : undefined },
              { label: "Countries", value: movie.productionCountries.join(", ") || undefined },
              { label: "Languages", value: movie.spokenLanguages.join(", ") || undefined },
              { label: "TMDB", value: movie.tmdbId ? <a className="nav-focus rounded text-brand hover:underline" href={`https://www.themoviedb.org/movie/${movie.tmdbId}`} target="_blank" rel="noreferrer">{movie.tmdbId}</a> : undefined },
              { label: "IMDb", value: movie.imdbId ? <a className="nav-focus rounded text-brand hover:underline" href={`https://www.imdb.com/title/${movie.imdbId}`} target="_blank" rel="noreferrer">{movie.imdbId}</a> : undefined },
              { label: "Added", value: formatDate(movie.createdAt) },
            ]}
          />
        </Panel>
        {movie.mediaFile ? (
          <FileDetails file={movie.mediaFile} />
        ) : (
          <Panel title="File" tone="secondary">
            <p className="inline-flex items-center gap-2 text-body-sm text-muted">
              <IconInfoCircle size={16} /> No file is linked yet. {movie.wanted ? "Librarian is looking for a release." : "Mark it as wanted to search automatically."}
            </p>
          </Panel>
        )}
      </div>

      <ConfirmDialog isOpen={confirmDelete} onOpenChange={setConfirmDelete} title={`Remove ${movie.title}?`} description="The movie leaves the catalogue. Its file stays on disk and will show up as unmatched on the next scan." confirmLabel="Remove" destructive isPending={deleting} onConfirm={remove} />
      <AcquisitionDialog isOpen={acquisition} onOpenChange={setAcquisition} target={{ kind: "movie", id: movie.id, title: movie.title, libraryId: movie.libraryId, monitored: movie.monitored, wanted: movie.wanted, qualityProfileId: movie.qualityProfileId }} onSaved={() => void refetch()} />
      <ReleaseSearchDialog isOpen={searching} onOpenChange={setSearching} query={movie.title} year={movie.year} imdbId={movie.imdbId} libraryId={movie.libraryId} target={{ movieId: movie.id }} />
    </DetailShell>
  );
}
