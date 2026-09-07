import { useMutation, useQuery } from "@apollo/client/react";
import { ListBox, ListBoxItem, Select, toast } from "@heroui/react";
import { useNavigate } from "@tanstack/react-router";
import { IconCalendarStats, IconMovie, IconPlus } from "@tabler/icons-react";
import { useEffect, useMemo, useState } from "react";

import { EmptyState, ErrorState, MediaRow, PosterCard, Section, SkeletonRow } from "@/components/ui";
import { AddMovieDocument, LibrariesOverviewDocument, LibraryMoviesByTmdbIdsDocument, MovieReleaseGuideDocument, type MovieReleaseGuideQuery } from "@/graphql/generated/graphql";
import { formatDate } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPES, libraryType } from "@/lib/library-types";

type Release = MovieReleaseGuideQuery["movieReleases"][number];

/** Region for TMDB's theatrical lists, from the browser locale; TMDB falls back to its default list. */
function localeRegion(): string | undefined {
  try {
    const region = new Intl.Locale(navigator.language).maximize().region;
    return region && /^[A-Z]{2}$/.test(region) ? region : undefined;
  } catch {
    return undefined;
  }
}

function daysUntil(date: string, today: Date): number {
  return Math.round((new Date(`${date}T00:00:00`).getTime() - today.getTime()) / 86400_000);
}

/**
 * Theatrical release rows from TMDB: what is in cinemas now and what is coming. Titles already
 * in a library link to their page; anything else can be added to a movie library in place.
 */
export function MovieReleases() {
  const navigate = useNavigate();
  const today = useMemo(() => {
    const date = new Date();
    date.setHours(0, 0, 0, 0);
    return date;
  }, []);
  const region = useMemo(localeRegion, []);
  const nowPlaying = useQuery(MovieReleaseGuideDocument, { variables: { kind: "NOW_PLAYING", region, page: 1 } });
  const upcoming = useQuery(MovieReleaseGuideDocument, { variables: { kind: "UPCOMING", region, page: 1 } });

  const recentList = useMemo(() => [...(nowPlaying.data?.movieReleases ?? [])].sort((a, b) => (b.releaseDate ?? "").localeCompare(a.releaseDate ?? "")), [nowPlaying.data]);
  const upcomingList = useMemo(() => {
    const playing = new Set(recentList.map((movie) => movie.providerId));
    // TMDB's upcoming list includes re-releases that keep their original date; keep them, but last.
    const todayIso = today.toISOString().slice(0, 10);
    const rank = (movie: Release) => (movie.releaseDate && movie.releaseDate < todayIso ? `~${movie.releaseDate}` : (movie.releaseDate ?? "9999"));
    return (upcoming.data?.movieReleases ?? []).filter((movie) => !playing.has(movie.providerId)).sort((a, b) => rank(a).localeCompare(rank(b)));
  }, [upcoming.data, recentList, today]);

  // Which of these are already in a library?
  const tmdbIds = useMemo(() => [...new Set([...recentList, ...upcomingList].map((movie) => movie.providerId))], [recentList, upcomingList]);
  const owned = useQuery(LibraryMoviesByTmdbIdsDocument, { variables: { ids: tmdbIds }, skip: tmdbIds.length === 0 });
  const ownedByTmdb = useMemo(() => new Map((owned.data?.movies.edges ?? []).map(({ node }) => [node.tmdbId, node])), [owned.data]);

  // Target library for "Add".
  const libraries = useQuery(LibrariesOverviewDocument);
  const targets = useMemo(() => (libraries.data?.libraries.edges ?? []).map((edge) => edge.node).filter((library) => libraryType(library.libraryType).type === "movies"), [libraries.data]);
  const [libraryId, setLibraryId] = useState<string | null>(null);
  useEffect(() => {
    if (!libraryId || !targets.some((library) => library.id === libraryId)) setLibraryId(targets[0]?.id ?? null);
  }, [targets, libraryId]);
  const [addMovie] = useMutation(AddMovieDocument, { refetchQueries: [LibraryMoviesByTmdbIdsDocument] });
  const [busy, setBusy] = useState<number | null>(null);

  const add = (movie: Release) => {
    if (!libraryId) return;
    setBusy(movie.providerId);
    addMovie({ variables: { libraryId, input: { tmdbId: movie.providerId, monitored: true } } })
      .then(({ data }) => {
        assertSuccess(data?.addMovie, "Could not add");
        toast.success(`${movie.title} added`);
      })
      .catch((error) => toast.danger(errorMessage(error)))
      .finally(() => setBusy(null));
  };

  const card = (movie: Release, badge: string | undefined) => {
    const existing = ownedByTmdb.get(movie.providerId);
    const meta = formatDate(movie.releaseDate) || (movie.year ? String(movie.year) : undefined);
    const status = existing ? (existing.hasFile ? "Downloaded" : existing.wanted ? "Wanted" : "In library") : undefined;
    return (
      <PosterCard
        key={movie.providerId}
        width="row"
        title={movie.title}
        meta={meta}
        image={movie.posterUrl}
        tint={LIBRARY_TYPES.movies.tintVar}
        badge={badge ?? status}
        {...(existing ? { to: "/movies/$movieId" as const, params: { movieId: existing.id } } : { onPress: () => void navigate({ to: "/search", search: { q: movie.title, in: "movie" } as never }) })}
        actions={
          !existing && libraryId ? (
            <button
              type="button"
              aria-label={`Add ${movie.title} to library`}
              data-focusable
              disabled={busy === movie.providerId}
              onClick={(event) => {
                event.preventDefault();
                event.stopPropagation();
                add(movie);
              }}
              className="nav-focus glass-control glass-brand grid size-8 place-items-center rounded-full disabled:opacity-60"
            >
              <IconPlus size={16} />
            </button>
          ) : undefined
        }
      />
    );
  };

  const picker =
    targets.length > 1 ? (
      <Select aria-label="Add to library" selectedKey={libraryId} onSelectionChange={(key) => setLibraryId(key === null ? null : String(key))} className="w-48" variant="secondary">
        <Select.Trigger>
          <Select.Value />
          <Select.Indicator />
        </Select.Trigger>
        <Select.Popover>
          <ListBox>
            {targets.map((library) => (
              <ListBoxItem key={library.id} id={library.id} textValue={library.name}>
                {library.name}
              </ListBoxItem>
            ))}
          </ListBox>
        </Select.Popover>
      </Select>
    ) : (
      <span>{region ? `In cinemas · ${region}` : "In cinemas"}</span>
    );

  const row = (query: typeof nowPlaying, list: Release[], label: string, empty: { title: string; description: string; icon: typeof IconMovie }, badge: (movie: Release) => string | undefined) => {
    if (query.loading && list.length === 0) return <SkeletonRow />;
    if (query.error) {
      return (
        <div className="page-gutter">
          <ErrorState compact title={`Could not load ${label.toLowerCase()}`} error={query.error} onRetry={() => void query.refetch()} />
        </div>
      );
    }
    if (list.length === 0) {
      return (
        <div className="page-gutter">
          <EmptyState compact icon={empty.icon} title={empty.title} description={empty.description} />
        </div>
      );
    }
    return <MediaRow ariaLabel={label}>{list.map((movie) => card(movie, badge(movie)))}</MediaRow>;
  };

  return (
    <>
      <Section title="Recently released" bleed trailing={picker}>
        {row(nowPlaying, recentList, "Recently released movies", { icon: IconMovie, title: "No current releases", description: "TMDB's now-playing list is empty for this region." }, () => undefined)}
      </Section>
      <Section title="Upcoming releases" bleed>
        {row(upcoming, upcomingList, "Upcoming movie releases", { icon: IconCalendarStats, title: "Nothing scheduled", description: "TMDB has no upcoming releases listed for this region." }, (movie) => {
          if (!movie.releaseDate) return undefined;
          const days = daysUntil(movie.releaseDate.slice(0, 10), today);
          if (days < 0) return "Re-release";
          return days === 0 ? "Today" : days === 1 ? "Tomorrow" : `In ${days} days`;
        })}
      </Section>
    </>
  );
}
