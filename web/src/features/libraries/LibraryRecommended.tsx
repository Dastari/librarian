import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { Link, useNavigate } from "@tanstack/react-router";
import { useContext, useLayoutEffect, useMemo, useRef, type CSSProperties, type ReactNode } from "react";

import { BackdropCard, MediaRow, PosterCard, Section } from "@/components/ui";
import { HomeSpotlight, movieSpotlight, showSpotlight, spotlightItems, type SpotlightItem } from "@/features/home/HomeSpotlight";
import { useFocusPreview } from "@/features/home/useFocusPreview";
import { useContinueWatching } from "@/features/home/useContinueWatching";
import { useMediaProgress } from "@/features/media/useMediaProgress";
import { usePlayer } from "@/features/player/usePlayer";
import { LibraryCollectionsDocument, LibraryGenreShelvesDocument, LibraryMoviesDocument, LibraryShowsDocument, type LibraryGenreShelvesQuery, type LibraryMoviesQuery } from "@/graphql/generated/graphql";
import { collectionPoster, moviePoster, showPoster } from "@/lib/artwork";
import { useSession } from "@/lib/auth/useSession";
import { formatClock, formatRuntime } from "@/lib/format";
import { LIBRARY_TYPES, libraryType } from "@/lib/library-types";

import { LibraryArtContext, type LibraryDetail } from "./LibraryLayout";

type Movie = LibraryMoviesQuery["movies"]["edges"][number]["node"];
type MovieTile = Pick<Movie, "id" | "title" | "year" | "runtime" | "certification" | "genres" | "overview" | "mediaFileId">;
type GenreShelf = LibraryGenreShelvesQuery["libraryGenreShelves"][number];
/** Genres need at least this many titles to earn a shelf; one-title rows are noise. */
const GENRE_SHELF_MIN = 2;

/** The library's largest genres, each as a shelf of its newest titles. */
function useGenreShelves(libraryId: string): GenreShelf[] {
  const shelves = useQuery(LibraryGenreShelvesDocument, { variables: { libraryId, maxGenres: 6, perGenre: SHELF } });
  return (shelves.data?.libraryGenreShelves ?? []).filter((shelf) => shelf.total >= GENRE_SHELF_MIN);
}
const SHELF = 24;

/**
 * A library's opening view, the way media servers present one: a spotlight over the title's
 * art, then shelves of what to watch next. The complete, filterable wall stays in Browse,
 * where its filters, sorting and letter jump keep their place.
 */
export function LibraryRecommended({ library }: { library: LibraryDetail }) {
  const type = libraryType(library.libraryType).type;
  return type === "tv" ? <ShowsRecommended library={library} /> : <MoviesRecommended library={library} />;
}

function MoviesRecommended({ library }: { library: LibraryDetail }) {
  const navigate = useNavigate();
  const player = usePlayer();
  const { user } = useSession();
  const libraryId = library.id;
  const recent = useQuery(LibraryMoviesDocument, { variables: { where: { libraryId: { eq: libraryId } }, orderBy: [{ createdAt: "DESC" }], page: { limit: SHELF, offset: 0 } } });
  const released = useQuery(LibraryMoviesDocument, { variables: { where: { libraryId: { eq: libraryId }, hasFile: { eq: true } }, orderBy: [{ releaseDate: "DESC" }], page: { limit: SHELF, offset: 0 } } });
  const collections = useQuery(LibraryCollectionsDocument, { variables: { where: { libraryId: { eq: libraryId } }, orderBy: [{ updatedAt: "DESC" }], page: { limit: SHELF, offset: 0 } } });
  const resume = useContinueWatching({ libraryId });
  const genres = useGenreShelves(libraryId);
  const { preview, previewOnFocus, clearPreview } = useFocusPreview();

  const recentList = useMemo(() => recent.data?.movies.edges.map((edge) => edge.node) ?? [], [recent.data]);
  const releasedList = released.data?.movies.edges.map((edge) => edge.node) ?? [];
  const collectionList = collections.data?.collections.edges.map((edge) => edge.node) ?? [];
  const fileIds = useMemo(() => recentList.map((movie) => movie.mediaFileId).filter((id): id is string => Boolean(id)), [recentList]);
  const progress = useMediaProgress(fileIds, user?.id);
  // Not started: has a file and no saved position. Only judged once progress has loaded.
  const unwatched = progress.loading ? [] : recentList.filter((movie) => movie.mediaFileId && !progress.progressByFile.get(movie.mediaFileId));
  const spotlight = spotlightItems(unwatched.length ? unwatched : recentList, []);

  const play = (movie: Omit<MovieTile, "runtime">) => {
    if (!movie.mediaFileId) return;
    player.playVideo({ mediaFileId: movie.mediaFileId, title: movie.title, subtitle: movie.year ? String(movie.year) : undefined, artwork: moviePoster(movie.id), entity: { kind: "movie", id: movie.id }, href: `/movies/${movie.id}` });
    void navigate({ to: "/watch/$mediaFileId", params: { mediaFileId: movie.mediaFileId } });
  };
  const browse = <Link to="/libraries/$libraryId/movies" params={{ libraryId }} className="nav-focus rounded hover:text-foreground">See all</Link>;
  const poster = (movie: MovieTile) => (
    <PosterCard key={movie.id} width="row" title={movie.title} meta={[movie.year, movie.runtime ? formatRuntime(movie.runtime, "minutes") : null].filter(Boolean).join(" · ")}
      image={moviePoster(movie.id)} tint={LIBRARY_TYPES.movies.tintVar} to="/movies/$movieId" params={{ movieId: movie.id }} onPlay={movie.mediaFileId ? () => play(movie) : undefined}
      onFocus={previewOnFocus(movieSpotlight(movie))} />
  );

  return (
    <RecommendedPage
      loading={recent.loading && !recent.data}
      spotlight={spotlight}
      preview={preview}
      onBrowse={clearPreview}
      onPlay={(item) => {
        // A previewed title can come from any shelf, not only Recently added.
        const movie = [recentList, releasedList, ...genres.map((shelf) => shelf.movies)].flat().find((candidate) => candidate.id === item.id);
        if (movie) play(movie);
      }}
      shelves={[
        resume.items.length > 0 ? <ResumeShelf key="resume" items={resume.items} /> : null,
        recentList.length > 0 ? <Section key="recent" title="Recently added" bleed trailing={browse}><MediaRow ariaLabel="Recently added">{recentList.map(poster)}</MediaRow></Section> : null,
        unwatched.length > 0 ? <Section key="unwatched" title="Not watched yet" bleed><MediaRow ariaLabel="Not watched yet">{unwatched.map(poster)}</MediaRow></Section> : null,
        releasedList.length > 1 ? <Section key="released" title="Newest releases" bleed><MediaRow ariaLabel="Newest releases">{releasedList.map(poster)}</MediaRow></Section> : null,
        collectionList.length > 0 ? (
          <Section key="collections" title="Collections" bleed trailing={<Link to="/libraries/$libraryId/collections" params={{ libraryId }} className="nav-focus rounded hover:text-foreground">See all</Link>}>
            <MediaRow ariaLabel="Collections">
              {collectionList.map((collection) => (
                <PosterCard key={collection.id} width="row" title={collection.name} meta={`${collection.movieCount} films`} image={collectionPoster(collection)} tint={LIBRARY_TYPES.movies.tintVar} to="/collections/$collectionId" params={{ collectionId: collection.id }} />
              ))}
            </MediaRow>
          </Section>
        ) : null,
        ...genres.map((shelf) => (
          <Section key={`genre:${shelf.genre}`} title={shelf.genre} bleed trailing={<span className="text-numeric">{shelf.total}</span>}>
            <MediaRow ariaLabel={shelf.genre}>{shelf.movies.map(poster)}</MediaRow>
          </Section>
        )),
      ]}
    />
  );
}

function ShowsRecommended({ library }: { library: LibraryDetail }) {
  const libraryId = library.id;
  const recent = useQuery(LibraryShowsDocument, { variables: { where: { libraryId: { eq: libraryId } }, orderBy: [{ createdAt: "DESC" }], page: { limit: SHELF, offset: 0 } } });
  const updated = useQuery(LibraryShowsDocument, { variables: { where: { libraryId: { eq: libraryId } }, orderBy: [{ updatedAt: "DESC" }], page: { limit: SHELF, offset: 0 } } });
  const resume = useContinueWatching({ libraryId });
  const genres = useGenreShelves(libraryId);
  const { preview, previewOnFocus, clearPreview } = useFocusPreview();
  const recentList = recent.data?.shows.edges.map((edge) => edge.node) ?? [];
  const updatedList = updated.data?.shows.edges.map((edge) => edge.node) ?? [];
  const spotlight = spotlightItems([], recentList);
  const poster = (show: Pick<(typeof recentList)[number], "id" | "name" | "year" | "network" | "genres" | "overview" | "backdropUrl">) => (
    <PosterCard key={show.id} width="row" title={show.name} meta={[show.year, show.network].filter(Boolean).join(" · ")} image={showPoster(show)} tint={LIBRARY_TYPES.tv.tintVar} to="/shows/$showId" params={{ showId: show.id }}
      onFocus={previewOnFocus(showSpotlight(show))} />
  );
  const browse = <Link to="/libraries/$libraryId/shows" params={{ libraryId }} className="nav-focus rounded hover:text-foreground">See all</Link>;
  return (
    <RecommendedPage
      loading={recent.loading && !recent.data}
      spotlight={spotlight}
      preview={preview}
      onBrowse={clearPreview}
      shelves={[
        resume.items.length > 0 ? <ResumeShelf key="resume" items={resume.items} /> : null,
        recentList.length > 0 ? <Section key="recent" title="Recently added" bleed trailing={browse}><MediaRow ariaLabel="Recently added">{recentList.map(poster)}</MediaRow></Section> : null,
        updatedList.length > 1 ? <Section key="updated" title="Recently updated" bleed><MediaRow ariaLabel="Recently updated">{updatedList.map(poster)}</MediaRow></Section> : null,
        ...genres.map((shelf) => (
          <Section key={`genre:${shelf.genre}`} title={shelf.genre} bleed trailing={<span className="text-numeric">{shelf.total}</span>}>
            <MediaRow ariaLabel={shelf.genre}>{shelf.shows.map(poster)}</MediaRow>
          </Section>
        )),
      ]}
    />
  );
}

interface RecommendedPageProps {
  spotlight: SpotlightItem[];
  shelves: ReactNode[];
  loading: boolean;
  onPlay?: (item: SpotlightItem) => void;
  /** The shelf card a remote rests on, shown in the spotlight (see `useFocusPreview`). */
  preview: SpotlightItem | null;
  onBrowse: () => void;
}

function RecommendedPage({ spotlight, shelves, loading, onPlay, preview, onBrowse }: RecommendedPageProps) {
  const setArt = useContext(LibraryArtContext);
  const visible = shelves.filter(Boolean);
  const opening = useRef<HTMLDivElement>(null);
  // The spotlight and the first shelf share the first screen: the block is at least as tall
  // as the space under the library header, and the spotlight grows into what is left.
  useLayoutEffect(() => {
    const element = opening.current;
    const scroller = element?.closest<HTMLElement>("#main-scroll");
    if (!element || !scroller) return;
    const measure = () => {
      const above = element.getBoundingClientRect().top - scroller.getBoundingClientRect().top + scroller.scrollTop;
      element.style.setProperty("--above", `${Math.round(above)}px`);
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(scroller);
    return () => observer.disconnect();
  }, []);
  // Rows bleed to the page edge; the layout's gutter is undone so headings still align.
  const shelf = (node: ReactNode, index: number) => (
    <div key={index} data-nav-section className="media-rise bleed-gutter min-w-0 !px-0" style={{ "--rise-order": Math.min(index, 6) } as CSSProperties}>{node}</div>
  );
  return (
    <div data-print-flow className="flex min-w-0 flex-col gap-10 pb-12">
      <div ref={opening} className="flex min-w-0 flex-col gap-10 md:min-h-[calc(var(--view-h,100svh)-var(--above,0px)-var(--space-6))]">
        {spotlight.length > 0 ? (
          <HomeSpotlight className="bleed-gutter min-w-0 flex-1 !px-0" items={spotlight} preview={preview} onBrowse={onBrowse} fill="library" poster={false} eyebrow={() => null} onArtChange={setArt} onPlay={onPlay ?? (() => {})} />
        ) : loading ? <div aria-hidden data-fill="library" className="media-hero flex-1" /> : null}
        {visible[0] ? shelf(visible[0], 0) : null}
      </div>
      {visible.slice(1).map((node, index) => shelf(node, index + 1))}
    </div>
  );
}

function ResumeShelf({ items }: { items: ReturnType<typeof useContinueWatching>["items"] }) {
  const navigate = useNavigate();
  const player = usePlayer();
  return (
    <Section title="Continue watching" bleed>
      <MediaRow ariaLabel="Continue watching">
        {items.map((item) => (
          <BackdropCard key={item.key} width="row" title={item.title} subtitle={item.subtitle} image={item.image} progress={item.progress}
            corner={item.duration ? `${formatClock(Math.max(0, item.duration - item.position))} left` : undefined}
            onPress={() => {
              if (item.kind === "movie" || item.kind === "episode") {
                player.playVideo({ mediaFileId: item.mediaFileId, title: item.title, subtitle: item.subtitle, artwork: item.image, entity: item.entity, href: item.href, startPosition: item.position });
                void navigate({ to: "/watch/$mediaFileId", params: { mediaFileId: item.mediaFileId } });
              } else void navigate({ href: item.href });
            }} />
        ))}
      </MediaRow>
    </Section>
  );
}
