import { Select } from "glass-ux/select";
import { useMemo, useState } from "react";

import { Artwork, HeroFacts, MetaLine, Section } from "@/components/ui";
import { DetailShell } from "@/features/media/DetailShell";
import type { ProviderAudiobookQuery, ProviderMovieQuery, ProviderShowQuery } from "@/graphql/generated/graphql";
import { artworkUrl } from "@/lib/api/urls";
import { formatDate, formatRuntime } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { AddToLibrary } from "./AddToLibrary";

type ProviderShow = ProviderShowQuery["providerShow"];

/**
 * Pages for titles that are not in a library yet. They look like the real thing because they
 * are the same page: the details and pictures come from the provider, through the server's
 * caches, and Add to library is the one action. Owned titles never reach them (see the routes).
 */
export function DiscoverMoviePage({ movie }: { movie: ProviderMovieQuery["providerMovie"] }) {
  return (
    <DetailShell
      backdrop={artworkUrl("provider-movie", movie.tmdbId, "backdrop")}
      hero={{
        poster: artworkUrl("provider-movie", movie.tmdbId, "poster"),
        tint: LIBRARY_TYPES.movies.tintVar,
        eyebrow: "Not in your library",
        title: movie.title,
        meta: <MetaLine items={[movie.year, movie.runtime ? formatRuntime(movie.runtime, "minutes") : null, movie.certification, movie.voteAverage ? `★ ${movie.voteAverage.toFixed(1)}` : null]} />,
        actions: <AddToLibrary target={{ kind: "movie", tmdbId: movie.tmdbId, title: movie.title, image: artworkUrl("provider-movie", movie.tmdbId, "poster"), year: movie.year, imdbId: movie.imdbId }} />,
        tagline: movie.tagline,
        description: movie.overview,
        facts: <HeroFacts items={[
          { label: "Director", value: movie.director },
          { label: "Starring", value: movie.castNames.slice(0, 5).join(", ") },
          { label: "Genres", value: movie.genres.join(", ") },
        ]} />,
      }}
    >
      {[]}
    </DetailShell>
  );
}

function ProviderEpisodes({ show }: { show: ProviderShow }) {
  const seasons = useMemo(() => [...new Set(show.episodes.map((episode) => episode.season))].sort((a, b) => a - b), [show.episodes]);
  const [season, setSeason] = useState(seasons[0] ?? 1);
  const episodes = show.episodes.filter((episode) => episode.season === season);
  if (seasons.length === 0) return null;
  return (
    <Section title="Episodes">
      {seasons.length > 1 ? <Select<string> aria-label="Season" size="sm" className="mb-4 w-40" value={String(season)} onValueChange={(value) => { if (value !== null) setSeason(Number(value)); }} options={seasons.map((value) => ({ value: String(value), label: `Season ${value}` }))} /> : null}
      <div data-nav-group className="grid grid-cols-[repeat(auto-fill,minmax(16rem,1fr))] gap-4">
        {episodes.map((episode) => (
          <article key={episode.id} tabIndex={0} data-focusable className="nav-focus tv-lift flex min-w-0 flex-col gap-2 rounded-tile">
            <Artwork src={episode.hasStill ? artworkUrl("provider-episode", `${show.tvmazeId}-${episode.id}`, "thumbnail") : undefined} alt="" aspect="backdrop" displayWidth={420} tint={LIBRARY_TYPES.tv.tintVar} className="tv-lift-target overflow-clip rounded-tile" />
            <div className="min-w-0">
              <p className="truncate text-title-sm text-foreground">{episode.number}. {episode.name}</p>
              <p className="truncate text-label-sm text-muted"><MetaLine items={[episode.airDate ? formatDate(episode.airDate) : null, episode.runtime ? formatRuntime(episode.runtime, "minutes") : null]} /></p>
              {episode.overview ? <p className="mt-1 line-clamp-3 text-label-sm text-muted">{episode.overview}</p> : null}
            </div>
          </article>
        ))}
      </div>
    </Section>
  );
}

export function DiscoverShowPage({ show }: { show: ProviderShow }) {
  const seasons = new Set(show.episodes.map((episode) => episode.season)).size;
  return (
    <DetailShell
      backdrop={artworkUrl("provider-show", show.tvmazeId, "backdrop")}
      hero={{
        poster: artworkUrl("provider-show", show.tvmazeId, "poster"),
        tint: LIBRARY_TYPES.tv.tintVar,
        eyebrow: "Not in your library",
        title: show.name,
        meta: <MetaLine items={[show.year, show.network, show.status, seasons ? `${seasons} season${seasons === 1 ? "" : "s"}` : null, show.episodes.length ? `${show.episodes.length} episodes` : null]} />,
        actions: <AddToLibrary target={{ kind: "show", tvmazeId: show.tvmazeId, title: show.name, image: artworkUrl("provider-show", show.tvmazeId, "poster"), year: show.year, imdbId: show.imdbId }} />,
        description: show.overview,
        facts: <HeroFacts items={[{ label: "Genres", value: show.genres.join(", ") }]} />,
      }}
    >
      <ProviderEpisodes show={show} />
    </DetailShell>
  );
}

export function DiscoverAudiobookPage({ book }: { book: ProviderAudiobookQuery["providerAudiobook"] }) {
  return (
    <DetailShell
      backdrop={null}
      hero={{
        poster: artworkUrl("provider-audiobook", book.providerId, "cover"),
        tint: LIBRARY_TYPES.audiobooks.tintVar,
        eyebrow: "Not in your library",
        title: book.title,
        meta: <MetaLine items={[book.authorName, book.publishedDate]} />,
        actions: <AddToLibrary target={{ kind: "audiobook", providerId: book.providerId, title: book.title, image: artworkUrl("provider-audiobook", book.providerId, "cover"), author: book.authorName }} />,
        description: book.description,
        facts: <HeroFacts items={[{ label: "Publisher", value: book.publisher }, { label: "Language", value: book.language }, { label: "ISBN", value: book.isbn }]} />,
      }}
    >
      {[]}
    </DetailShell>
  );
}
