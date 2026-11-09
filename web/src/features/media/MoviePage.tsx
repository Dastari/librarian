import { IconImage, IconInfo } from "glass-ux/icons";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { Link, useNavigate } from "@tanstack/react-router";
import { useCallback, useMemo } from "react";

import { RecommendedRelease } from "@/features/recommendations/RecommendedRelease";
import { Artwork, EmptyState, ErrorState, HeroFacts, MediaRow, MetaLine, Panel, PosterCard, Section, SkeletonHero, StatusChip } from "@/components/ui";
import { AcquisitionChip } from "@/features/acquisition/AcquisitionChip";
import { movieAcquisitionMeta } from "@/features/acquisition/mode";
import { usePlayer } from "@/features/player/usePlayer";
import {
  MovieCastCreditsDocument,
  MovieCollectionDetailsDocument,
  MovieDetailDocument,
  PlaybackProgressForFileDocument,
} from "@/graphql/generated/graphql";
import { useContentStatuses, contentStatusMeta } from "@/hooks/useContentStatuses";
import { movieBackdrop, moviePoster, personProfile } from "@/lib/artwork";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { formatClock, formatRuntime } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { MediaActions } from "./MediaActions";
import { DetailShell } from "./DetailShell";
import { FileDetails } from "./FileDetails";
import { filesWithProgress, type ProgressReset } from "./progressReset";
import { useMovieEntity } from "./useMovieEntity";

export function MoviePage({ movieId }: { movieId: string }) {
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

  const resume = progress.data?.playbackProgresses.edges[0]?.node;
  const resumeAt = resume && !resume.isWatched && resume.currentPosition > 5 ? resume.currentPosition : undefined;
  const play = useCallback((target: { id: string; title: string; year?: number | null; mediaFileId?: string | null }) => {
    if (!target.mediaFileId) return;
    player.playVideo({ mediaFileId: target.mediaFileId, title: target.title, subtitle: target.year ? String(target.year) : undefined, artwork: moviePoster(target.id), backdrop: movieBackdrop(target.id), entity: { kind: "movie", id: target.id }, href: `/movies/${target.id}`, startPosition: resumeAt });
    void navigate({ to: "/watch/$mediaFileId", params: { mediaFileId: target.mediaFileId } });
  }, [navigate, player, resumeAt]);
  const playLabel = resumeAt ? `Resume · ${formatClock(resumeAt)}` : "Play";
  const reset = useMemo<ProgressReset<{ mediaFileId?: string | null }>>(() => ({
    files: (rows) => filesWithProgress(rows.map((row) => row.mediaFileId), new Map(resume && resume.mediaFileId ? [[resume.mediaFileId, resume]] : [])),
  }), [resume]);
  const { actions, dialogs, openSettings } = useMovieEntity<NonNullable<typeof movie>>({ open: false, play, playLabel, reset });

  if (error && !movie) return <ErrorState error={error} onRetry={() => void refetch()} className="m-8" />;
  if (!movie) return loading ? <SkeletonHero /> : <EmptyState icon={IconInfo} title="Movie not found" description="This item may have been removed from the library." className="page-gutter my-8" />;

  const status = contentStatusMeta(statuses, movie.id);

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
            <AcquisitionChip status={movieAcquisitionMeta(movie)} onPress={isAdmin ? () => openSettings(movie, "downloads") : undefined} />
          </>
        ),
        actions: (
          <MediaActions
            target={movie}
            label={movie.title}
            actions={actions}
            play={movie.mediaFileId ? { label: playLabel, run: () => play(movie) } : undefined}
          />
        ),
        tagline: movie.tagline,
        description: movie.overview,
        facts: <HeroFacts items={[
          { label: "Director", value: movie.director },
          { label: "Genres", value: movie.genres.join(", ") },
          { label: "Original title", value: movie.originalTitle !== movie.title ? movie.originalTitle : null },
          { label: "Links", value: movie.tmdbId || movie.imdbId ? (
            <span className="inline-flex gap-3">
              {movie.tmdbId ? <a className="nav-focus rounded text-brand hover:underline" href={`https://www.themoviedb.org/movie/${movie.tmdbId}`} target="_blank" rel="noreferrer">TMDB</a> : null}
              {movie.imdbId ? <a className="nav-focus rounded text-brand hover:underline" href={`https://www.imdb.com/title/${movie.imdbId}`} target="_blank" rel="noreferrer">IMDb</a> : null}
            </span>
          ) : null },
        ]} />,
      }}
    >
      {castList.length > 0 ? (
        <Section title="Cast" bleed>
          <MediaRow ariaLabel="Cast">
            {castList.map((credit) => (
              <Link key={credit.id} to="/search" search={{ q: credit.person?.name ?? "" }} data-focusable className="tv-lift nav-focus group w-28 shrink-0 rounded-tile p-1 text-center" aria-label={[credit.person?.name, credit.characterName].filter(Boolean).join(" as ")}>
                <Artwork src={personProfile(credit.person)} alt="" aspect="square" displayWidth={96} className="tv-lift-target mx-auto w-24 overflow-clip rounded-full" fallback={<IconImage size="xl" />} />
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

      <div>
        {movie.mediaFile ? (
          <FileDetails file={movie.mediaFile} />
        ) : (
          <Panel title="File" tone="secondary">
            <p className="inline-flex items-center gap-2 text-body-sm text-muted">
              <IconInfo size="md" /> No file is linked yet. {movie.wanted ? "Librarian is looking for a release." : "Mark it as wanted to search automatically."}
            </p>
            <RecommendedRelease target={{ movieId: movie.id }} missing={1} />
          </Panel>
        )}
      </div>

      {dialogs}
    </DetailShell>
  );
}
