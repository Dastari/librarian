import { useLazyQuery, useMutation } from "@apollo/client/react";
import { Checkbox, SearchField, toast } from "@heroui/react";
import { IconPlus, IconSearch, IconX } from "@tabler/icons-react";
import { useEffect, useMemo, useState } from "react";

import { Artwork, Button, Dialog, EmptyState, Spinner } from "@/components/ui";
import { useDebouncedValue } from "@/hooks/useDebouncedValue";
import {
  AddAlbumDocument,
  AddAudiobookDocument,
  AddMovieCollectionDocument,
  AddMovieDocument,
  AddTvShowDocument,
  SearchAlbumsDocument,
  SearchAudiobooksDocument,
  SearchMovieCollectionsDocument,
  SearchMoviesDocument,
  SearchTvShowsDocument,
} from "@/graphql/generated/graphql";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPES } from "@/lib/library-types";

export type ProviderKind = "movie" | "show" | "album" | "audiobook" | "collection";

interface ResultItem {
  key: string;
  title: string;
  subtitle?: string;
  overview?: string | null;
  image?: string | null;
  add: () => Promise<void>;
}

interface ProviderSearchDialogProps {
  kind: ProviderKind;
  libraryId: string;
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  onAdded?: () => void;
  initialQuery?: string;
}

const COPY: Record<ProviderKind, { title: string; placeholder: string; provider: string }> = {
  movie: { title: "Add movie", placeholder: "Search TMDB", provider: "TMDB" },
  show: { title: "Add show", placeholder: "Search TVmaze", provider: "TVmaze" },
  album: { title: "Add album", placeholder: "Search MusicBrainz", provider: "MusicBrainz" },
  audiobook: { title: "Add audiobook", placeholder: "Search Open Library", provider: "Open Library" },
  collection: { title: "Add collection", placeholder: "Search TMDB collections", provider: "TMDB" },
};

/**
 * Search a metadata provider and add the chosen result to the library. One dialog serves every
 * media type; the type only changes which query and mutation run.
 */
export function ProviderSearchDialog({ kind, libraryId, isOpen, onOpenChange, onAdded, initialQuery = "" }: ProviderSearchDialogProps) {
  const [query, setQuery] = useState(initialQuery);
  const [wanted, setWanted] = useState(true);
  const [busyKey, setBusyKey] = useState<string | null>(null);
  const debounced = useDebouncedValue(query.trim(), 350);

  const [searchMovies, movies] = useLazyQuery(SearchMoviesDocument);
  const [searchShows, shows] = useLazyQuery(SearchTvShowsDocument);
  const [searchAlbums, albums] = useLazyQuery(SearchAlbumsDocument);
  const [searchBooks, books] = useLazyQuery(SearchAudiobooksDocument);
  const [searchCollections, collections] = useLazyQuery(SearchMovieCollectionsDocument);

  const [addMovie] = useMutation(AddMovieDocument);
  const [addShow] = useMutation(AddTvShowDocument);
  const [addAlbum] = useMutation(AddAlbumDocument);
  const [addBook] = useMutation(AddAudiobookDocument);
  const [addCollection] = useMutation(AddMovieCollectionDocument);

  useEffect(() => {
    if (!isOpen || debounced.length < 2) return;
    switch (kind) {
      case "movie":
        void searchMovies({ variables: { query: debounced } });
        break;
      case "show":
        void searchShows({ variables: { query: debounced } });
        break;
      case "album":
        void searchAlbums({ variables: { query: debounced, includeSingles: true, includeEps: true, includeCompilations: false, includeLive: false, includeSoundtracks: true } });
        break;
      case "audiobook":
        void searchBooks({ variables: { query: debounced } });
        break;
      case "collection":
        void searchCollections({ variables: { query: debounced } });
        break;
    }
  }, [debounced, isOpen, kind, searchAlbums, searchBooks, searchCollections, searchMovies, searchShows]);

  const active = kind === "movie" ? movies : kind === "show" ? shows : kind === "album" ? albums : kind === "audiobook" ? books : collections;

  const finish = (label: string) => {
    toast.success(`${label} added`);
    onAdded?.();
    onOpenChange(false);
  };

  const items = useMemo<ResultItem[]>(() => {
    if (kind === "movie") {
      return (movies.data?.searchMovies ?? []).map((result) => ({
        key: `${result.provider}-${result.providerId}`,
        title: result.title,
        subtitle: [result.year, result.voteAverage ? `★ ${result.voteAverage.toFixed(1)}` : null].filter(Boolean).join(" · "),
        overview: result.overview,
        image: result.posterUrl,
        add: async () => {
          const { data } = await addMovie({ variables: { libraryId, input: { tmdbId: result.providerId, monitored: wanted } } });
          assertSuccess(data?.addMovie, "Could not add movie");
          finish(result.title);
        },
      }));
    }
    if (kind === "show") {
      return (shows.data?.searchTvShows ?? []).map((result) => ({
        key: `${result.provider}-${result.providerId}`,
        title: result.name,
        subtitle: [result.year, result.network, result.status].filter(Boolean).join(" · "),
        overview: result.overview,
        image: result.posterUrl,
        add: async () => {
          const { data } = await addShow({ variables: { libraryId, input: { tvmazeId: result.providerId, autoDownloadMode: wanted ? "WANTED" : "NONE" } } });
          assertSuccess(data?.addTvShow, "Could not add show");
          finish(result.name);
        },
      }));
    }
    if (kind === "album") {
      return (albums.data?.searchAlbums ?? []).map((result) => ({
        key: `${result.provider}-${result.providerId}`,
        title: result.title,
        subtitle: [result.artistName, result.year, result.albumType].filter(Boolean).join(" · "),
        image: result.coverUrl,
        add: async () => {
          const { data } = await addAlbum({ variables: { input: { libraryId, musicbrainzId: result.providerId, autoDownloadMode: wanted ? "WANTED" : "NONE" } } });
          assertSuccess(data?.addAlbum, "Could not add album");
          finish(result.title);
        },
      }));
    }
    if (kind === "audiobook") {
      return (books.data?.searchAudiobooks ?? []).map((result) => ({
        key: `${result.provider}-${result.providerId}`,
        title: result.title,
        subtitle: [result.authorName, result.year].filter(Boolean).join(" · "),
        overview: result.description,
        image: result.coverUrl,
        add: async () => {
          const { data } = await addBook({ variables: { input: { libraryId, openlibraryId: result.providerId, autoDownloadMode: wanted ? "WANTED" : "NONE" } } });
          assertSuccess(data?.addAudiobook, "Could not add audiobook");
          finish(result.title);
        },
      }));
    }
    return (collections.data?.searchMovieCollections ?? []).map((result) => ({
      key: `${result.provider}-${result.collectionId}`,
      title: result.name,
      overview: result.overview,
      image: result.posterUrl,
      add: async () => {
        const { data } = await addCollection({ variables: { libraryId, input: { collectionId: result.collectionId, wantedMissing: wanted } } });
        assertSuccess(data?.addMovieCollection, "Could not add collection");
        finish(result.name);
      },
    }));
  }, [kind, movies.data, shows.data, albums.data, books.data, collections.data, addMovie, addShow, addAlbum, addBook, addCollection, libraryId, wanted]);

  const copy = COPY[kind];
  const aspect = kind === "album" ? "square" : "poster";
  const tint = kind === "movie" || kind === "collection" ? LIBRARY_TYPES.movies.tintVar : kind === "show" ? LIBRARY_TYPES.tv.tintVar : kind === "album" ? LIBRARY_TYPES.music.tintVar : LIBRARY_TYPES.audiobooks.tintVar;

  return (
    <Dialog isOpen={isOpen} onOpenChange={onOpenChange} title={copy.title} size="xl">
      <div className="flex flex-col gap-4">
        <div className="flex flex-wrap items-center gap-3">
          <SearchField aria-label={copy.placeholder} value={query} onChange={setQuery} autoFocus className="min-w-0 flex-1">
            <SearchField.Group>
              <SearchField.SearchIcon>
                <IconSearch size={16} />
              </SearchField.SearchIcon>
              <SearchField.Input placeholder={copy.placeholder} />
              <SearchField.ClearButton>
                <IconX size={14} />
              </SearchField.ClearButton>
            </SearchField.Group>
          </SearchField>
          <Checkbox isSelected={wanted} onChange={setWanted}>
            <Checkbox.Control>
              <Checkbox.Indicator />
            </Checkbox.Control>
            <Checkbox.Content>{kind === "collection" ? "Want missing movies" : "Search for downloads"}</Checkbox.Content>
          </Checkbox>
        </div>

        <div className="scrollbar-thin -mx-1 max-h-[55svh] min-h-48 overflow-y-auto px-1">
          {active.loading ? (
            <div className="grid h-48 place-items-center">
              <Spinner size={28} />
            </div>
          ) : active.error ? (
            <EmptyState compact icon={IconSearch} title="Search failed" description={errorMessage(active.error)} />
          ) : debounced.length < 2 ? (
            <EmptyState compact icon={IconSearch} title={`Search ${copy.provider}`} description="Type at least two characters." />
          ) : items.length === 0 ? (
            <EmptyState compact icon={IconSearch} title="No results" description="Try a different spelling or add the year." />
          ) : (
            <ul className="flex flex-col gap-1">
              {items.map((item) => (
                <li key={item.key} className="flex items-center gap-3 rounded-card p-2 hover:bg-surface-hover">
                  <Artwork src={item.image} alt="" aspect={aspect} tint={tint} className="w-12 shrink-0 rounded-md" />
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-title-sm text-foreground">{item.title}</p>
                    {item.subtitle ? <p className="truncate text-label-sm text-muted">{item.subtitle}</p> : null}
                    {item.overview ? <p className="mt-0.5 line-clamp-2 text-label-sm text-muted">{item.overview}</p> : null}
                  </div>
                  <Button
                    size="sm"
                    variant="secondary"
                    isPending={busyKey === item.key}
                    onPress={() => {
                      setBusyKey(item.key);
                      item.add().catch((error) => toast.danger(errorMessage(error))).finally(() => setBusyKey(null));
                    }}
                  >
                    <IconPlus size={16} /> Add
                  </Button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </Dialog>
  );
}
