import { useQuery } from "@apollo/client/react";
import { Link, useNavigate } from "@tanstack/react-router";
import { IconPlus, IconStack2 } from "@tabler/icons-react";

import { SceneBackdrop } from "@/components/three/SceneBackdrop";
import { BackdropCard, Button, EmptyState, MediaRow, PosterCard, Section } from "@/components/ui";
import { LibraryCard } from "@/features/libraries/LibraryCard";
import { usePlayer } from "@/features/player/usePlayer";
import { LibrariesOverviewDocument, RecentAlbumsDocument, RecentAudiobooksDocument, RecentMoviesDocument, RecentShowsDocument } from "@/graphql/generated/graphql";
import { albumCover, audiobookCover, moviePoster, showPoster } from "@/lib/artwork";
import { formatClock, formatRuntime, formatYear } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { MovieReleases } from "./MovieReleases";
import { TvGuide } from "./TvGuide";
import { useContinueWatching } from "./useContinueWatching";

const ROW_LIMIT = 20;

export function HomePage() {
  const navigate = useNavigate();
  const player = usePlayer();
  const libraries = useQuery(LibrariesOverviewDocument);
  const movies = useQuery(RecentMoviesDocument, { variables: { limit: ROW_LIMIT } });
  const shows = useQuery(RecentShowsDocument, { variables: { limit: ROW_LIMIT } });
  const albums = useQuery(RecentAlbumsDocument, { variables: { limit: ROW_LIMIT } });
  const audiobooks = useQuery(RecentAudiobooksDocument, { variables: { limit: ROW_LIMIT } });
  const resume = useContinueWatching();

  const libraryList = libraries.data?.libraries.edges.map((edge) => edge.node) ?? [];
  const movieList = movies.data?.movies.edges.map((edge) => edge.node) ?? [];
  const showList = shows.data?.shows.edges.map((edge) => edge.node) ?? [];
  const albumList = albums.data?.albums.edges.map((edge) => edge.node) ?? [];
  const bookList = audiobooks.data?.audiobooks.edges.map((edge) => edge.node) ?? [];
  const booting = libraries.loading && !libraries.data;

  if (!booting && libraryList.length === 0) {
    return (
      <div className="relative flex min-h-svh flex-col items-center justify-center page-gutter">
        <SceneBackdrop density={8} />
        <EmptyState
          icon={IconStack2}
          title="Add your first library"
          description="Point Librarian at a folder of movies, shows, music or audiobooks and it will catalogue everything it finds."
          action={
            <Link to="/settings/libraries">
              <Button variant="primary" size="lg">
                <IconPlus size={18} /> Add library
              </Button>
            </Link>
          }
          className="max-w-lg"
        />
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-10 pb-12 pt-6">
      <div className="page-gutter">
        <TvGuide />
      </div>

      {resume.items.length > 0 ? (
        <Section title="Continue watching" bleed>
          <MediaRow ariaLabel="Continue watching">
            {resume.items.map((item) => (
              <BackdropCard
                key={item.key}
                width="row"
                title={item.title}
                subtitle={item.subtitle}
                image={item.image}
                progress={item.progress}
                corner={item.duration ? `${formatClock(Math.max(0, item.duration - item.position))} left` : undefined}
                onPress={() => {
                  if (item.kind === "movie" || item.kind === "episode") {
                    player.playVideo({ mediaFileId: item.mediaFileId, title: item.title, subtitle: item.subtitle, artwork: item.image, entity: { kind: item.kind, id: item.href.split("/").pop()! }, startPosition: item.position });
                    void navigate({ to: "/watch/$mediaFileId", params: { mediaFileId: item.mediaFileId } });
                  } else {
                    void navigate({ href: item.href });
                  }
                }}
              />
            ))}
          </MediaRow>
        </Section>
      ) : null}

      <MovieReleases />

      <Section title="Libraries" className="page-gutter" trailing={<Link to="/libraries" className="nav-focus rounded hover:text-foreground">See all</Link>}>
        <div className="library-grid">
          {libraryList.map((library) => (
            <LibraryCard key={library.id} library={library} />
          ))}
        </div>
      </Section>

      {movieList.length > 0 ? (
        <Section title="Recently added movies" bleed>
          <MediaRow ariaLabel="Recently added movies">
            {movieList.map((movie) => (
              <PosterCard
                key={movie.id}
                width="row"
                title={movie.title}
                meta={[movie.year, movie.runtime ? formatRuntime(movie.runtime, "minutes") : null].filter(Boolean).join(" · ")}
                image={moviePoster(movie.id)}
                tint={LIBRARY_TYPES.movies.tintVar}
                to="/movies/$movieId"
                params={{ movieId: movie.id }}
                onPlay={movie.mediaFileId ? () => {
                  player.playVideo({ mediaFileId: movie.mediaFileId!, title: movie.title, subtitle: movie.year ? String(movie.year) : undefined, artwork: moviePoster(movie.id), entity: { kind: "movie", id: movie.id } });
                  void navigate({ to: "/watch/$mediaFileId", params: { mediaFileId: movie.mediaFileId! } });
                } : undefined}
              />
            ))}
          </MediaRow>
        </Section>
      ) : null}

      {showList.length > 0 ? (
        <Section title="Recently added shows" bleed>
          <MediaRow ariaLabel="Recently added shows">
            {showList.map((show) => (
              <PosterCard key={show.id} width="row" title={show.name} meta={[show.year, show.network].filter(Boolean).join(" · ")} image={showPoster(show)} tint={LIBRARY_TYPES.tv.tintVar} to="/shows/$showId" params={{ showId: show.id }} />
            ))}
          </MediaRow>
        </Section>
      ) : null}

      {albumList.length > 0 ? (
        <Section title="New music" bleed>
          <MediaRow ariaLabel="New music">
            {albumList.map((album) => (
              <PosterCard key={album.id} width="row" aspect="square" title={album.name} meta={[formatYear(album.releaseDate) || album.year, album.albumType].filter(Boolean).join(" · ")} image={albumCover(album)} tint={LIBRARY_TYPES.music.tintVar} to="/albums/$albumId" params={{ albumId: album.id }} />
            ))}
          </MediaRow>
        </Section>
      ) : null}

      {bookList.length > 0 ? (
        <Section title="New audiobooks" bleed>
          <MediaRow ariaLabel="New audiobooks">
            {bookList.map((book) => (
              <PosterCard key={book.id} width="row" title={book.title} meta={book.authorName ?? undefined} image={audiobookCover(book)} tint={LIBRARY_TYPES.audiobooks.tintVar} to="/audiobooks/$audiobookId" params={{ audiobookId: book.id }} />
            ))}
          </MediaRow>
        </Section>
      ) : null}
    </div>
  );
}
