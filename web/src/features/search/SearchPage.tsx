import { useQuery } from "@apollo/client/react";
import { SearchField } from "@heroui/react";
import { useNavigate } from "@tanstack/react-router";
import { IconDownload, IconSearch, IconX } from "@tabler/icons-react";
import { parseAsString, parseAsStringEnum, useQueryState } from "nuqs";
import { useEffect, useRef, useState } from "react";

import { Button, EmptyState, MediaRow, PageHeader, PosterCard, Section, SegmentTabs, SkeletonRow } from "@/components/ui";
import { ReleaseSearchDialog } from "@/features/downloads/ReleaseSearchDialog";
import { LibrariesOverviewDocument, LibrarySearchDocument } from "@/graphql/generated/graphql";
import { useDebouncedValue } from "@/hooks/useDebouncedValue";
import { albumCover, artistImage, audiobookCover, moviePoster, showPoster } from "@/lib/artwork";
import { useIsAdmin } from "@/lib/auth/useSession";
import { LIBRARY_TYPES, libraryType } from "@/lib/library-types";

import { ProviderResults } from "./ProviderResults";

const SCOPES = ["library", "movie", "show", "album", "audiobook"] as const;
const ADD_LABELS = { movies: "Add movies", tv: "Add TV shows", music: "Add music", audiobooks: "Add audiobooks" } as const;
type Scope = (typeof SCOPES)[number];

/**
 * Search: your library first, then metadata providers to add new items, plus a shortcut to
 * search sources for releases. The scope tabs keep provider searches explicit so we never hit
 * external APIs on every keystroke by accident.
 */
export function SearchPage() {
  const navigate = useNavigate();
  const isAdmin = useIsAdmin();
  const [query, setQuery] = useQueryState("q", parseAsString.withDefault(""));
  const [scope, setScope] = useQueryState("in", parseAsStringEnum<Scope>([...SCOPES]).withDefault("library"));
  const debounced = useDebouncedValue(query.trim(), 300);
  const inputRef = useRef<HTMLInputElement>(null);
  const [releases, setReleases] = useState(false);

  const libraries = useQuery(LibrariesOverviewDocument);
  const libraryList = libraries.data?.libraries.edges.map((edge) => edge.node) ?? [];
  const results = useQuery(LibrarySearchDocument, { variables: { needle: debounced, limit: 20 }, skip: debounced.length < 2 || scope !== "library" });

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const data = results.data ?? results.previousData;
  const movies = data?.movies.edges.map((edge) => edge.node) ?? [];
  const shows = data?.shows.edges.map((edge) => edge.node) ?? [];
  const albums = data?.albums.edges.map((edge) => edge.node) ?? [];
  const artists = data?.artists.edges.map((edge) => edge.node) ?? [];
  const audiobooks = data?.audiobooks.edges.map((edge) => edge.node) ?? [];
  const episodes = data?.episodes.edges.map((edge) => edge.node) ?? [];
  const total = movies.length + shows.length + albums.length + artists.length + audiobooks.length + episodes.length;

  const availableScopes: Array<{ key: Scope; label: string }> = [
    { key: "library", label: "Your library" },
    ...(isAdmin
      ? (["movies", "tv", "music", "audiobooks"] as const)
          .filter((type) => libraryList.some((library) => libraryType(library.libraryType).type === type))
          .map((type) => ({ key: (type === "movies" ? "movie" : type === "tv" ? "show" : type === "music" ? "album" : "audiobook") as Scope, label: ADD_LABELS[type] }))
      : []),
  ];

  return (
    <div className="page-gutter flex flex-col gap-6 py-8">
      <PageHeader
        title="Search"
        actions={
          isAdmin && debounced ? (
            <Button variant="secondary" onPress={() => setReleases(true)}>
              <IconDownload size={16} /> Find releases
            </Button>
          ) : null
        }
      />
      <SearchField aria-label="Search" value={query} onChange={(value) => void setQuery(value || null)} className="w-full max-w-2xl" data-spatial-start>
        <SearchField.Group className="h-14 rounded-pill px-2 text-body-lg">
          <SearchField.SearchIcon>
            <IconSearch size={20} />
          </SearchField.SearchIcon>
          <SearchField.Input ref={inputRef} placeholder="Titles, people, albums…" />
          <SearchField.ClearButton>
            <IconX size={16} />
          </SearchField.ClearButton>
        </SearchField.Group>
      </SearchField>
      {availableScopes.length > 1 ? <SegmentTabs ariaLabel="Search scope" items={availableScopes} selected={scope} onSelect={(key) => void setScope(key as Scope)} size="sm" className="max-w-2xl" /> : null}

      {scope !== "library" ? (
        <ProviderResults kind={scope} query={debounced} libraries={libraryList} />
      ) : debounced.length < 2 ? (
        <EmptyState icon={IconSearch} title="Search your library" description="Type at least two characters." className="border-none" />
      ) : results.loading && !data ? (
        <SkeletonRow />
      ) : total === 0 ? (
        <EmptyState icon={IconSearch} title={`Nothing in your library matches “${debounced}”`} description={isAdmin ? "Switch to an “Add” tab to search metadata providers." : undefined} className="border-none" />
      ) : (
        <div className="-mx-[calc(var(--page-gutter)+var(--safe-left))] flex flex-col gap-8">
          {movies.length ? (
            <Section title="Movies" bleed={false} className="[&>div:first-child]:page-gutter">
              <MediaRow ariaLabel="Movies">
                {movies.map((movie) => (
                  <PosterCard key={movie.id} width="row" title={movie.title} meta={movie.year ?? undefined} image={moviePoster(movie.id)} tint={LIBRARY_TYPES.movies.tintVar} to="/movies/$movieId" params={{ movieId: movie.id }} />
                ))}
              </MediaRow>
            </Section>
          ) : null}
          {shows.length ? (
            <Section title="Shows" bleed={false} className="[&>div:first-child]:page-gutter">
              <MediaRow ariaLabel="Shows">
                {shows.map((show) => (
                  <PosterCard key={show.id} width="row" title={show.name} meta={show.year ?? undefined} image={showPoster(show)} tint={LIBRARY_TYPES.tv.tintVar} to="/shows/$showId" params={{ showId: show.id }} />
                ))}
              </MediaRow>
            </Section>
          ) : null}
          {episodes.length ? (
            <Section title="Episodes" bleed={false} className="[&>div:first-child]:page-gutter">
              <MediaRow ariaLabel="Episodes">
                {episodes.map((episode) => (
                  <PosterCard key={episode.id} width="row" title={episode.title ?? `Episode ${episode.episode}`} meta={`${episode.show?.name ?? ""} · S${String(episode.season).padStart(2, "0")}E${String(episode.episode).padStart(2, "0")}`} image={episode.show ? showPoster(episode.show) : undefined} tint={LIBRARY_TYPES.tv.tintVar} onPress={() => void navigate({ to: "/shows/$showId", params: { showId: episode.showId } })} />
                ))}
              </MediaRow>
            </Section>
          ) : null}
          {albums.length ? (
            <Section title="Albums" bleed={false} className="[&>div:first-child]:page-gutter">
              <MediaRow ariaLabel="Albums">
                {albums.map((album) => (
                  <PosterCard key={album.id} width="row" aspect="square" title={album.name} meta={album.year ?? undefined} image={albumCover(album)} tint={LIBRARY_TYPES.music.tintVar} to="/albums/$albumId" params={{ albumId: album.id }} />
                ))}
              </MediaRow>
            </Section>
          ) : null}
          {artists.length ? (
            <Section title="Artists" bleed={false} className="[&>div:first-child]:page-gutter">
              <MediaRow ariaLabel="Artists">
                {artists.map((artist) => (
                  <PosterCard key={artist.id} width="row" aspect="square" title={artist.name} image={artistImage(artist)} tint={LIBRARY_TYPES.music.tintVar} to="/artists/$artistId" params={{ artistId: artist.id }} />
                ))}
              </MediaRow>
            </Section>
          ) : null}
          {audiobooks.length ? (
            <Section title="Audiobooks" bleed={false} className="[&>div:first-child]:page-gutter">
              <MediaRow ariaLabel="Audiobooks">
                {audiobooks.map((book) => (
                  <PosterCard key={book.id} width="row" title={book.title} meta={book.authorName ?? undefined} image={audiobookCover(book)} tint={LIBRARY_TYPES.audiobooks.tintVar} to="/audiobooks/$audiobookId" params={{ audiobookId: book.id }} />
                ))}
              </MediaRow>
            </Section>
          ) : null}
        </div>
      )}
      <ReleaseSearchDialog isOpen={releases} onOpenChange={setReleases} query={debounced} />
    </div>
  );
}
