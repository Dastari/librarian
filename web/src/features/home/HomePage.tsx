import { useState, type CSSProperties, type ReactNode } from "react";
import { useCompleteConnection } from "@/features/libraries/browser/useCompleteConnection";
import { useHomeShelves, useLibraryPreferences } from "@/features/personal/usePersonalLibrary";
import { HomeCustomization } from "@/features/personal/HomeCustomization";
import { PersonalShelf } from "@/features/personal/PersonalShelf";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { Link, useNavigate } from "@tanstack/react-router";
import { ButtonLink } from "glass-ux/button-link";
import { IconPlus } from "glass-ux/icons";
import { IconStack2 } from "@tabler/icons-react";

import { SceneBackdrop } from "@/components/three/SceneBackdrop";
import { Button, EmptyState, ErrorState, MediaRow, PosterCard, Section, SkeletonBlock } from "@/components/ui";
import { LibraryCard } from "@/features/libraries/LibraryCard";
import { usePlayer } from "@/features/player/usePlayer";
import { LibrariesOverviewDocument, RecentAlbumsDocument, RecentAudiobooksDocument, RecentMoviesDocument, RecentShowsDocument } from "@/graphql/generated/graphql";
import { albumCover, audiobookCover, movieBackdrop, moviePoster, showPoster } from "@/lib/artwork";
import { formatRuntime, formatYear } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { GuideSpotlight } from "@/features/guide/GuideSpotlight";
import { MovieReleases } from "./MovieReleases";
import { ResumeCard } from "./ResumeCard";
import { useContinueWatching } from "./useContinueWatching";

const ROW_LIMIT = 20;

export function HomePage() {
  const navigate = useNavigate();
  const player = usePlayer();
  const [customizing, setCustomizing] = useState(false);
  const home = useHomeShelves();
  const preferences = useLibraryPreferences();
  const libraries = useCompleteConnection(LibrariesOverviewDocument, {}, (data) => data.libraries);
  const movies = useQuery(RecentMoviesDocument, { variables: { limit: ROW_LIMIT } });
  const shows = useQuery(RecentShowsDocument, { variables: { limit: ROW_LIMIT } });
  const albums = useQuery(RecentAlbumsDocument, { variables: { limit: ROW_LIMIT } });
  const audiobooks = useQuery(RecentAudiobooksDocument, { variables: { limit: ROW_LIMIT } });
  const resume = useContinueWatching();

  const favouriteIds = new Set(preferences.rows.filter((row) => row.favourite).map((row) => row.libraryId));
  const libraryList = [...libraries.rows].sort((a, b) => Number(favouriteIds.has(b.id)) - Number(favouriteIds.has(a.id)));
  const movieList = movies.data?.movies.edges.map((edge) => edge.node) ?? [];
  const showList = shows.data?.shows.edges.map((edge) => edge.node) ?? [];
  const albumList = albums.data?.albums.edges.map((edge) => edge.node) ?? [];
  const bookList = audiobooks.data?.audiobooks.edges.map((edge) => edge.node) ?? [];
  const booting = libraries.loading;

  if (libraries.error && libraryList.length === 0) return <ErrorState error={libraries.error} onRetry={() => void libraries.refetch()} className="page-gutter my-6" />;
  if (!booting && libraryList.length === 0) {
    return (
      <div className="relative flex min-h-full flex-1 flex-col items-center justify-center page-gutter">
        <SceneBackdrop density={8} />
        <EmptyState
          icon={IconStack2}
          title="Add your first library"
          description="Point Librarian at a folder of movies, shows, music or audiobooks and it will catalogue everything it finds."
          action={
            <ButtonLink variant="primary" size="lg" icon={<IconPlus />} render={<Link to="/settings/libraries" />}>
              Add library
            </ButtonLink>
          }
          className="max-w-lg"
        />
      </div>
    );
  }

  const sections: Record<string, ReactNode> = {
    continue: (resume.items.length > 0 ? (
        <Section title="Continue watching" bleed>
          <MediaRow ariaLabel="Continue watching">
            {resume.items.map((item) => (
              <ResumeCard key={item.key} item={item} />
            ))}
          </MediaRow>
        </Section>
      ) : null),
    libraries: (<Section title="Libraries" bleed trailing={<Link to="/libraries" className="nav-focus rounded hover:text-foreground" data-tv-skip>See all</Link>}>
        <MediaRow ariaLabel="Libraries">
          {booting && libraryList.length === 0
            ? [0, 1, 2].map((index) => <SkeletonBlock key={index} className="h-40 w-64 shrink-0 rounded-card" />)
            : libraryList.map((library) => <LibraryCard key={library.id} library={library} className="w-64 shrink-0" />)}
        </MediaRow>
      </Section>),
    movies: (movieList.length > 0 ? (
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
                  player.playVideo({ mediaFileId: movie.mediaFileId!, title: movie.title, subtitle: movie.year ? String(movie.year) : undefined, artwork: moviePoster(movie.id), backdrop: movieBackdrop(movie.id), entity: { kind: "movie", id: movie.id } });
                  void navigate({ to: "/watch/$mediaFileId", params: { mediaFileId: movie.mediaFileId! } });
                } : undefined}
              />
            ))}
          </MediaRow>
        </Section>
      ) : null),
    shows: (showList.length > 0 ? (
        <Section title="Recently added shows" bleed>
          <MediaRow ariaLabel="Recently added shows">
            {showList.map((show) => (
              <PosterCard key={show.id} width="row" title={show.name} meta={[show.year, show.network].filter(Boolean).join(" · ")} image={showPoster(show)} tint={LIBRARY_TYPES.tv.tintVar} to="/shows/$showId" params={{ showId: show.id }} />
            ))}
          </MediaRow>
        </Section>
      ) : null),
    albums: (albumList.length > 0 ? (
        <Section title="New music" bleed>
          <MediaRow ariaLabel="New music">
            {albumList.map((album) => (
              <PosterCard key={album.id} width="row" aspect="square" title={album.name} meta={[formatYear(album.releaseDate) || album.year, album.albumType].filter(Boolean).join(" · ")} image={albumCover(album)} tint={LIBRARY_TYPES.music.tintVar} to="/albums/$albumId" params={{ albumId: album.id }} />
            ))}
          </MediaRow>
        </Section>
      ) : null),
    audiobooks: (bookList.length > 0 ? (
        <Section title="New audiobooks" bleed>
          <MediaRow ariaLabel="New audiobooks">
            {bookList.map((book) => (
              <PosterCard key={book.id} width="row" title={book.title} meta={book.authorName ?? undefined} image={audiobookCover(book)} tint={LIBRARY_TYPES.audiobooks.tintVar} to="/audiobooks/$audiobookId" params={{ audiobookId: book.id }} />
            ))}
          </MediaRow>
        </Section>
      ) : null),
    guide: (<div className="page-gutter">
        <GuideSpotlight />
      </div>),
    releases: (<MovieReleases />),
  };
  const shelves = home.shelves.filter((shelf) => shelf.visible && (shelf.listId || sections[shelf.key]));
  // Shelves from the first line: what to watch next is the page, not a preview of one title.
  return <div className="relative isolate flex min-w-0 flex-col gap-10 overflow-x-clip pb-12 pt-2">
    {shelves.map((shelf, index) => <div key={shelf.key} data-nav-section className="media-rise min-w-0" style={{ "--rise-order": Math.min(index, 6) } as CSSProperties}>{shelf.listId ? <PersonalShelf listId={shelf.listId} /> : sections[shelf.key]}</div>)}
    <div className="page-gutter flex justify-center" data-print="hide"><Button variant="ghost" onPress={() => setCustomizing(true)}>Customize home</Button></div>
    {customizing ? <HomeCustomization open={customizing} onOpenChange={setCustomizing} /> : null}
  </div>;
}
