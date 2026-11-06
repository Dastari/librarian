import { useMutation } from "@apollo/client/react";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { Select } from "glass-ux/select";
import { Surface } from "glass-ux/surface";
import { toast } from "glass-ux/toast";
import { IconPlus, IconSearch } from "glass-ux/icons";
import { useNavigate } from "@tanstack/react-router";
import { useEffect, useMemo, useState } from "react";

import { Artwork, Button, EmptyState, PosterCard, Spinner } from "@/components/ui";
import {
  AddAlbumDocument,
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

/** Provider search results as posters that open each title's page. Albums, which have no such page yet, add from their card. */
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
  const [addAlbum] = useMutation(AddAlbumDocument);

  const active = kind === "movie" ? movies : kind === "show" ? shows : kind === "album" ? albums : books;
  const cards = useMemo<Card[]>(() => (kind === "album" ? (albums.data?.searchAlbums ?? []).map((result) => ({ key: result.providerId, title: result.title, subtitle: [result.artistName, result.year].filter(Boolean).join(" · "), image: result.coverUrl, add: async (target) => { assertSuccess((await addAlbum({ variables: { input: { libraryId: target, musicbrainzId: result.providerId } } })).data?.addAlbum, "Could not add"); return result.title; } })) : []), [kind, albums.data, addAlbum]);

  const navigate = useNavigate();
  // Everything but albums has a page of its own, with Add to library on it.
  const pages = useMemo(() => {
    if (kind === "movie") return (movies.data?.searchMovies ?? []).map((result) => ({ key: String(result.providerId), title: result.title, meta: [result.year, result.voteAverage ? `★ ${result.voteAverage.toFixed(1)}` : null].filter(Boolean).join(" · "), image: result.posterUrl, open: () => void navigate({ to: "/discover/movies/$tmdbId", params: { tmdbId: String(result.providerId) } }) }));
    if (kind === "show") return (shows.data?.searchTvShows ?? []).map((result) => ({ key: String(result.providerId), title: result.name, meta: [result.year, result.network].filter(Boolean).join(" · "), image: result.posterUrl, open: () => void navigate({ to: "/discover/shows/$tvmazeId", params: { tvmazeId: String(result.providerId) } }) }));
    if (kind === "audiobook") return (books.data?.searchAudiobooks ?? []).map((result) => ({ key: result.providerId, title: result.title, meta: [result.authorName, result.year].filter(Boolean).join(" · "), image: result.coverUrl, open: () => void navigate({ to: "/discover/audiobooks/$providerId", params: { providerId: result.providerId } }) }));
    return [];
  }, [kind, movies.data, shows.data, books.data, navigate]);

  const meta = LIBRARY_TYPES[KIND_TYPE[kind]];

  if (targets.length === 0) return <EmptyState icon={meta.icon} title={`No ${meta.label.toLowerCase()} library`} description="Create one under Settings → Libraries first." className="border-none" />;
  if (!enabled) return <EmptyState icon={IconSearch} title={`Search ${kind === "movie" ? "TMDB" : kind === "show" ? "TVmaze" : kind === "album" ? "MusicBrainz" : "Open Library"}`} description="Type at least two characters." className="border-none" />;

  return (
    <div className="flex flex-col gap-4">
      {kind === "album" && targets.length > 1 ? (
        <Select aria-label="Add to library" value={libraryId} onValueChange={setLibraryId} options={targets.map((library) => ({ value: library.id, label: library.name }))} className="w-64 max-w-full" />
      ) : null}
      {active.loading && !active.data ? (
        <div className="grid h-48 place-items-center">
          <Spinner size={28} />
        </div>
      ) : active.error ? (
        <EmptyState icon={IconSearch} title="Search failed" description={errorMessage(active.error)} className="border-none" />
      ) : (kind === "album" ? cards.length : pages.length) === 0 ? (
        <EmptyState icon={IconSearch} title="No results" className="border-none" />
      ) : kind !== "album" ? (
        <div data-nav-group className="grid grid-cols-[repeat(auto-fill,minmax(10rem,1fr))] gap-x-4 gap-y-6">
          {pages.map((page) => <PosterCard key={page.key} width="fill" title={page.title} meta={page.meta} image={page.image} tint={meta.tintVar} onPress={page.open} />)}
        </div>
      ) : (
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {cards.map((card) => (
            <Surface key={card.key} material="card" radius="tile" padding="tile" className="flex gap-3">
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
                    <IconPlus /> Add
                  </Button>
                </div>
              </div>
            </Surface>
          ))}
        </div>
      )}
    </div>
  );
}
