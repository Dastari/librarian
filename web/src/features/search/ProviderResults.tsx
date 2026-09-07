import { useMutation, useQuery } from "@apollo/client/react";
import { ListBox, ListBoxItem, Select, toast } from "@heroui/react";
import { IconPlus, IconSearch } from "@tabler/icons-react";
import { useEffect, useMemo, useState } from "react";

import { Artwork, Button, EmptyState, Spinner } from "@/components/ui";
import {
  AddAlbumDocument,
  AddAudiobookDocument,
  AddMovieDocument,
  AddTvShowDocument,
  SearchAlbumsDocument,
  SearchAudiobooksDocument,
  SearchMoviesDocument,
  SearchTvShowsDocument,
  type LibrariesOverviewQuery,
} from "@/graphql/generated/graphql";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPES, libraryType, type LibraryType } from "@/lib/library-types";

type Kind = "movie" | "show" | "album" | "audiobook";
type Library = LibrariesOverviewQuery["libraries"]["edges"][number]["node"];

const KIND_TYPE: Record<Kind, LibraryType> = { movie: "movies", show: "tv", album: "music", audiobook: "audiobooks" };

interface Card {
  key: string;
  title: string;
  subtitle?: string;
  overview?: string | null;
  image?: string | null;
  add: (libraryId: string) => Promise<string>;
}

/** Provider search results as a poster grid with a target-library picker. */
export function ProviderResults({ kind, query, libraries }: { kind: Kind; query: string; libraries: Library[] }) {
  const targets = useMemo(() => libraries.filter((library) => libraryType(library.libraryType).type === KIND_TYPE[kind]), [libraries, kind]);
  const [libraryId, setLibraryId] = useState<string | null>(null);
  useEffect(() => {
    if (!libraryId || !targets.some((library) => library.id === libraryId)) setLibraryId(targets[0]?.id ?? null);
  }, [targets, libraryId]);
  const [busy, setBusy] = useState<string | null>(null);
  const enabled = query.length >= 2;

  const movies = useQuery(SearchMoviesDocument, { variables: { query }, skip: !enabled || kind !== "movie" });
  const shows = useQuery(SearchTvShowsDocument, { variables: { query }, skip: !enabled || kind !== "show" });
  const albums = useQuery(SearchAlbumsDocument, { variables: { query, includeSingles: true, includeEps: true, includeCompilations: false, includeLive: false, includeSoundtracks: true }, skip: !enabled || kind !== "album" });
  const books = useQuery(SearchAudiobooksDocument, { variables: { query }, skip: !enabled || kind !== "audiobook" });
  const [addMovie] = useMutation(AddMovieDocument);
  const [addShow] = useMutation(AddTvShowDocument);
  const [addAlbum] = useMutation(AddAlbumDocument);
  const [addBook] = useMutation(AddAudiobookDocument);

  const active = kind === "movie" ? movies : kind === "show" ? shows : kind === "album" ? albums : books;
  const cards = useMemo<Card[]>(() => {
    if (kind === "movie") return (movies.data?.searchMovies ?? []).map((result) => ({ key: String(result.providerId), title: result.title, subtitle: [result.year, result.voteAverage ? `★ ${result.voteAverage.toFixed(1)}` : null].filter(Boolean).join(" · "), overview: result.overview, image: result.posterUrl, add: async (target) => { assertSuccess((await addMovie({ variables: { libraryId: target, input: { tmdbId: result.providerId, monitored: true } } })).data?.addMovie, "Could not add"); return result.title; } }));
    if (kind === "show") return (shows.data?.searchTvShows ?? []).map((result) => ({ key: String(result.providerId), title: result.name, subtitle: [result.year, result.network].filter(Boolean).join(" · "), overview: result.overview, image: result.posterUrl, add: async (target) => { assertSuccess((await addShow({ variables: { libraryId: target, input: { tvmazeId: result.providerId, autoDownloadMode: "WANTED" } } })).data?.addTvShow, "Could not add"); return result.name; } }));
    if (kind === "album") return (albums.data?.searchAlbums ?? []).map((result) => ({ key: result.providerId, title: result.title, subtitle: [result.artistName, result.year].filter(Boolean).join(" · "), image: result.coverUrl, add: async (target) => { assertSuccess((await addAlbum({ variables: { input: { libraryId: target, musicbrainzId: result.providerId } } })).data?.addAlbum, "Could not add"); return result.title; } }));
    return (books.data?.searchAudiobooks ?? []).map((result) => ({ key: result.providerId, title: result.title, subtitle: [result.authorName, result.year].filter(Boolean).join(" · "), overview: result.description, image: result.coverUrl, add: async (target) => { assertSuccess((await addBook({ variables: { input: { libraryId: target, openlibraryId: result.providerId } } })).data?.addAudiobook, "Could not add"); return result.title; } }));
  }, [kind, movies.data, shows.data, albums.data, books.data, addMovie, addShow, addAlbum, addBook]);

  const meta = LIBRARY_TYPES[KIND_TYPE[kind]];

  if (targets.length === 0) return <EmptyState icon={meta.icon} title={`No ${meta.label.toLowerCase()} library`} description="Create one under Settings → Libraries first." className="border-none" />;
  if (!enabled) return <EmptyState icon={IconSearch} title={`Search ${kind === "movie" ? "TMDB" : kind === "show" ? "TVmaze" : kind === "album" ? "MusicBrainz" : "Open Library"}`} description="Type at least two characters." className="border-none" />;

  return (
    <div className="flex flex-col gap-4">
      {targets.length > 1 ? (
        <Select aria-label="Add to library" selectedKey={libraryId} onSelectionChange={(key) => setLibraryId(key === null ? null : String(key))} className="w-64" variant="secondary">
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
      ) : null}
      {active.loading && !active.data ? (
        <div className="grid h-48 place-items-center">
          <Spinner size={28} />
        </div>
      ) : active.error ? (
        <EmptyState icon={IconSearch} title="Search failed" description={errorMessage(active.error)} className="border-none" />
      ) : cards.length === 0 ? (
        <EmptyState icon={IconSearch} title="No results" className="border-none" />
      ) : (
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {cards.map((card) => (
            <div key={card.key} className="flex gap-3 rounded-card border border-border bg-surface p-3 shadow-surface">
              <Artwork src={card.image} alt="" aspect={kind === "album" ? "square" : "poster"} tint={meta.tintVar} className="w-20 shrink-0 rounded-md" />
              <div className="flex min-w-0 flex-1 flex-col">
                <p className="truncate text-title-sm text-foreground">{card.title}</p>
                {card.subtitle ? <p className="truncate text-label-sm text-muted">{card.subtitle}</p> : null}
                {card.overview ? <p className="mt-1 line-clamp-3 text-label-sm text-muted">{card.overview}</p> : null}
                <div className="mt-auto pt-2">
                  <Button
                    size="sm"
                    variant="secondary"
                    isPending={busy === card.key}
                    isDisabled={!libraryId}
                    onPress={() => {
                      if (!libraryId) return;
                      setBusy(card.key);
                      card
                        .add(libraryId)
                        .then((title) => toast.success(`${title} added`))
                        .catch((error) => toast.danger(errorMessage(error)))
                        .finally(() => setBusy(null));
                    }}
                  >
                    <IconPlus size={14} /> Add
                  </Button>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
