import { useLazyQuery, useMutation } from "@apollo/client/react";
import { Button } from "glass-ux/button";
import { Dialog } from "glass-ux/dialog";
import { IconCheck, IconPlus, IconSearch, IconSparkle } from "glass-ux/icons";
import { SearchField } from "glass-ux/input";
import { ListRow } from "glass-ux/list-row";
import { ScrollArea } from "glass-ux/scroll-area";
import { toast } from "glass-ux/toast";
import { useEffect, useMemo, useState } from "react";

import { EmptyState, PosterCard, SkeletonBlock } from "@/components/ui";
import { useDialogReturnFocus } from "@/components/ui/Dialog";
import { AfterAddDialog, type AddedTitle } from "@/features/discover/AfterAddDialog";
import { useDebouncedValue } from "@/hooks/useDebouncedValue";
import {
  AddAlbumDocument,
  AddAudiobookDocument,
  AddMovieCollectionDocument,
  AddMovieDocument,
  AddTvShowDocument,
  SearchAlbumsDocument,
  SearchAudiobooksDocument,
  OwnedAlbumsByMusicbrainzDocument,
  OwnedAudiobooksByAudibleDocument,
  OwnedMoviesByTmdbDocument,
  OwnedShowsByTvmazeDocument,
  SearchMovieCollectionsDocument,
  SearchMoviesDocument,
  SearchTvShowsDocument,
} from "@/graphql/generated/graphql";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPES } from "@/lib/library-types";

export type ProviderKind = "movie" | "show" | "album" | "audiobook" | "collection";

/** A provider's result. `add` puts it in the library; a collection says whether to want what's missing. */
interface ResultItem {
  key: string;
  title: string;
  subtitle?: string;
  image?: string | null;
  link?: { to: string; params: Record<string, string> };
  add: (wantMissing: boolean) => Promise<AddedTitle | null>;
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
  audiobook: { title: "Add audiobook", placeholder: "Search Audible", provider: "Audible" },
  collection: { title: "Add collection", placeholder: "Search TMDB collections", provider: "TMDB" },
};

const META: Record<ProviderKind, (typeof LIBRARY_TYPES)[keyof typeof LIBRARY_TYPES]> = {
  movie: LIBRARY_TYPES.movies, collection: LIBRARY_TYPES.movies, show: LIBRARY_TYPES.tv, album: LIBRARY_TYPES.music, audiobook: LIBRARY_TYPES.audiobooks,
};

/**
 * Search a metadata provider and add what you find, as posters like everywhere else in the app.
 * Adding asks how to get a copy afterwards (`AfterAddDialog`) and then comes back here, so a whole
 * series can be added in one go. One dialog serves every media type.
 */
export function ProviderSearchDialog({ kind, libraryId, isOpen, onOpenChange, onAdded, initialQuery = "" }: ProviderSearchDialogProps) {
  const finalFocus = useDialogReturnFocus(isOpen);
  const [query, setQuery] = useState(initialQuery);
  const [busyKey, setBusyKey] = useState<string | null>(null);
  const [addedKeys, setAddedKeys] = useState<ReadonlySet<string>>(new Set());
  const [justAdded, setJustAdded] = useState<AddedTitle | null>(null);
  const [collection, setCollection] = useState<ResultItem | null>(null);
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

  const [findMovies, ownedMovies] = useLazyQuery(OwnedMoviesByTmdbDocument, { fetchPolicy: "network-only" });
  const [findShows, ownedShows] = useLazyQuery(OwnedShowsByTvmazeDocument, { fetchPolicy: "network-only" });
  const [findAlbums, ownedAlbums] = useLazyQuery(OwnedAlbumsByMusicbrainzDocument, { fetchPolicy: "network-only" });
  const [findBooks, ownedBooks] = useLazyQuery(OwnedAudiobooksByAudibleDocument, { fetchPolicy: "network-only" });

  const active = kind === "movie" ? movies : kind === "show" ? shows : kind === "album" ? albums : kind === "audiobook" ? books : collections;

  // Which of the results are already in the library, by the id each kind of title keeps.
  const resultIds = kind === "movie" ? (movies.data?.searchMovies ?? []).map((result) => String(result.providerId))
    : kind === "show" ? (shows.data?.searchTvShows ?? []).map((result) => String(result.providerId))
    : kind === "album" ? (albums.data?.searchAlbums ?? []).map((result) => result.providerId)
    : kind === "audiobook" ? (books.data?.searchAudiobooks ?? []).map((result) => result.providerId)
    : [];
  const idsKey = resultIds.join("|");
  useEffect(() => {
    if (!isOpen || !idsKey) return;
    const ids = idsKey.split("|");
    if (kind === "movie") void findMovies({ variables: { ids: ids.map(Number) } });
    else if (kind === "show") void findShows({ variables: { ids: ids.map(Number) } });
    else if (kind === "album") void findAlbums({ variables: { ids } });
    else if (kind === "audiobook") void findBooks({ variables: { ids } });
  }, [idsKey, isOpen, kind, findMovies, findShows, findAlbums, findBooks]);
  const ownedIds = useMemo(() => new Set<string>(
    kind === "movie" ? (ownedMovies.data?.movies.edges ?? []).map((edge) => String(edge.node.tmdbId))
    : kind === "show" ? (ownedShows.data?.shows.edges ?? []).map((edge) => String(edge.node.tvmazeId))
    : kind === "album" ? (ownedAlbums.data?.albums.edges ?? []).map((edge) => String(edge.node.musicbrainzId))
    : kind === "audiobook" ? (ownedBooks.data?.audiobooks.edges ?? []).map((edge) => String(edge.node.audibleId))
    : [],
  ), [kind, ownedMovies.data, ownedShows.data, ownedAlbums.data, ownedBooks.data]);

  const items = useMemo<ResultItem[]>(() => {
    const created = (id: string | undefined, added: Omit<AddedTitle, "id" | "libraryId">): AddedTitle => {
      if (!id) throw new Error("The server did not return the new title.");
      return { ...added, id, libraryId };
    };
    if (kind === "movie") {
      return (movies.data?.searchMovies ?? []).map((result) => ({
        key: `${result.provider}-${result.providerId}`,
        title: result.title,
        subtitle: [result.year, result.voteAverage ? `★ ${result.voteAverage.toFixed(1)}` : null].filter(Boolean).join(" · "),
        image: result.posterUrl,
        link: { to: "/discover/movies/$tmdbId", params: { tmdbId: String(result.providerId) } },
        add: async () => {
          const { data } = await addMovie({ variables: { libraryId, input: { tmdbId: result.providerId, monitored: false } } });
          return created(assertSuccess(data?.addMovie, "Could not add movie").movie?.id, { kind: "movie", title: result.title, image: result.posterUrl, year: result.year });
        },
      }));
    }
    if (kind === "show") {
      return (shows.data?.searchTvShows ?? []).map((result) => ({
        key: `${result.provider}-${result.providerId}`,
        title: result.name,
        subtitle: [result.year, result.network].filter(Boolean).join(" · "),
        image: result.posterUrl,
        link: { to: "/discover/shows/$tvmazeId", params: { tvmazeId: String(result.providerId) } },
        add: async () => {
          const { data } = await addShow({ variables: { libraryId, input: { tvmazeId: result.providerId, autoDownloadMode: "NONE" } } });
          return created(assertSuccess(data?.addTvShow, "Could not add show").show?.id, { kind: "show", title: result.name, image: result.posterUrl, year: result.year });
        },
      }));
    }
    if (kind === "album") {
      return (albums.data?.searchAlbums ?? []).map((result) => ({
        key: `${result.provider}-${result.providerId}`,
        title: result.title,
        subtitle: [result.artistName, result.year].filter(Boolean).join(" · "),
        image: result.coverUrl,
        add: async () => {
          const { data } = await addAlbum({ variables: { input: { libraryId, musicbrainzId: result.providerId, autoDownloadMode: "NONE" } } });
          return created(assertSuccess(data?.addAlbum, "Could not add album").album?.id, { kind: "album", title: result.title, image: result.coverUrl, artist: result.artistName });
        },
      }));
    }
    if (kind === "audiobook") {
      return (books.data?.searchAudiobooks ?? []).map((result) => ({
        key: `${result.provider}-${result.providerId}`,
        title: result.title,
        subtitle: [result.authorName, result.year].filter(Boolean).join(" · "),
        image: result.coverUrl,
        link: { to: "/discover/audiobooks/$providerId", params: { providerId: result.providerId } },
        add: async () => {
          const { data } = await addBook({ variables: { input: { libraryId, providerId: result.providerId, autoDownloadMode: "NONE" } } });
          return created(assertSuccess(data?.addAudiobook, "Could not add audiobook").audiobook?.id, { kind: "audiobook", title: result.title, image: result.coverUrl, author: result.authorName });
        },
      }));
    }
    return (collections.data?.searchMovieCollections ?? []).map((result) => ({
      key: `${result.provider}-${result.collectionId}`,
      title: result.name,
      image: result.posterUrl,
      add: async (wantMissing) => {
        const { data } = await addCollection({ variables: { libraryId, input: { collectionId: result.collectionId, wantedMissing: wantMissing } } });
        assertSuccess(data?.addMovieCollection, "Could not add collection");
        toast.success(`${result.name} added`);
        return null;
      },
    }));
  }, [kind, movies.data, shows.data, albums.data, books.data, collections.data, addMovie, addShow, addAlbum, addBook, addCollection, libraryId]);

  const run = async (item: ResultItem, wantMissing = false) => {
    setBusyKey(item.key);
    try {
      const added = await item.add(wantMissing);
      setAddedKeys((keys) => new Set(keys).add(item.key));
      setCollection(null);
      setJustAdded(added);
      onAdded?.();
    } catch (error) {
      toast.danger(errorMessage(error));
    } finally {
      setBusyKey(null);
    }
  };

  const copy = COPY[kind];
  const meta = META[kind];
  const aspect = kind === "album" ? "square" : "poster";

  return (
    <>
      <Dialog.Root open={isOpen} onOpenChange={onOpenChange}>
        <Dialog.Popup size="xl" kind="settings" aria-label={copy.title} finalFocus={finalFocus}>
          <Dialog.Header title={copy.title} icon={<meta.icon />} subtitle={`Searches ${copy.provider}`} />
          <Dialog.Body flush className="flex min-h-0 flex-1 flex-col">
            <div className="shrink-0 px-(--inset-panel) pb-3">
              <SearchField aria-label={copy.placeholder} value={query} onValueChange={setQuery} placeholder={copy.placeholder} autoFocus className="w-full" />
            </div>
            <ScrollArea aria-label="Results" className="min-h-0 flex-1">
              <div className="px-(--inset-panel) pb-(--inset-panel) pt-1">
                {active.loading ? (
                  <div className="provider-results">
                    {Array.from({ length: 10 }, (_, index) => <SkeletonBlock key={index} className={aspect === "square" ? "aspect-square rounded-tile" : "aspect-[2/3] rounded-tile"} />)}
                  </div>
                ) : active.error ? (
                  <EmptyState compact icon={IconSearch} title="Search failed" description={errorMessage(active.error)} />
                ) : debounced.length < 2 ? (
                  <EmptyState compact icon={IconSearch} title={`Search ${copy.provider}`} description="Type at least two characters." />
                ) : items.length === 0 ? (
                  <EmptyState compact icon={IconSearch} title="No results" description="Try a different spelling or add the year." />
                ) : (
                  <ul className="provider-results">
                    {items.map((item) => {
                      const added = addedKeys.has(item.key);
                      const owned = !added && ownedIds.has(item.key.slice(item.key.indexOf("-") + 1));
                      return (
                        <li key={item.key} className="flex min-w-0 flex-col gap-2">
                          <PosterCard width="fill" wrapTitle title={item.title} meta={item.subtitle} image={item.image} aspect={aspect} tint={meta.tintVar} to={item.link?.to as never} params={item.link?.params as never} />
                          <Button
                            size="sm"
                            variant={added || owned ? "ghost" : "secondary"}
                            className="mt-auto w-full"
                            icon={added || owned ? <IconCheck /> : <IconPlus />}
                            data-focusable
                            aria-label={added ? `${item.title} added` : owned ? `${item.title} is already in your library` : `Add ${item.title}`}
                            disabledReason={added ? "Already added" : owned ? "Already in your library" : undefined}
                            status={busyKey === item.key ? "pending" : undefined}
                            onClick={() => { if (kind === "collection") setCollection(item); else void run(item); }}
                          >
                            {added ? "Added" : owned ? "Already in library" : "Add"}
                          </Button>
                        </li>
                      );
                    })}
                  </ul>
                )}
              </div>
            </ScrollArea>
          </Dialog.Body>
        </Dialog.Popup>
      </Dialog.Root>

      <Dialog.Root open={Boolean(collection)} onOpenChange={(open) => { if (!open) setCollection(null); }}>
        <Dialog.Popup size="md">
          <Dialog.Header title={`Add ${collection?.title}`} icon={<IconPlus />} subtitle="Do you want the movies you don't have yet?" />
          <Dialog.Body>
            <div className="flex flex-col gap-1">
              <ListRow icon={<IconSparkle />} title="Download what's missing" meta="Adds the collection and grabs its other movies as matches turn up." disabled={Boolean(busyKey)} onClick={() => { if (collection) void run(collection, true); }} />
              <ListRow icon={<IconPlus />} title="Just add the collection" meta="Nothing downloads." disabled={Boolean(busyKey)} onClick={() => { if (collection) void run(collection); }} />
            </div>
          </Dialog.Body>
        </Dialog.Popup>
      </Dialog.Root>

      <AfterAddDialog added={justAdded} onDone={() => setJustAdded(null)} />
    </>
  );
}
