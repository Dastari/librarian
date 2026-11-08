import { EntityIdentity, type EntityIdentityProps, type EntityState, type EntityTierProp } from "glass-ux/entity";

import { ArtGlyph, ArtImage } from "@/components/entities/ArtGlyph";
import { AlbumIdentity } from "@/features/media/AlbumIdentity";
import { AudiobookIdentity } from "@/features/media/AudiobookIdentity";
import { MovieIdentity } from "@/features/media/MovieIdentity";
import { ShowIdentity } from "@/features/media/ShowIdentity";
import type { PersonalEntryFragment } from "@/graphql/generated/graphql";
import { sizedArtwork } from "@/lib/api/urls";
import { albumCover, audiobookCover, episodeThumb } from "@/lib/artwork";
import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { formatClock, formatRuntime } from "@/lib/format";

import { episodeCode } from "./types";

const playable = (mediaFileId: string | null | undefined): EntityState => mediaFileId ? { id: "available", label: "Playable", tone: "success" } : { id: "missing", label: "No file yet", tone: "neutral" };

function thin(props: { kind: string; name: string; art: string; glyph: EntityIdentityProps<never>["icon"]; shape?: "square" | "poster"; mediaFileId: string | null | undefined; facts: string[]; tier: EntityTierProp }) {
  return (
    <EntityIdentity
      kind={props.kind}
      name={props.name}
      icon={<ArtGlyph src={sizedArtwork(props.art, 96)} fallback={props.glyph} shape={props.shape ?? "square"} />}
      image={<ArtImage src={sizedArtwork(props.art, 120)} fallback={props.glyph} />}
      state={playable(props.mediaFileId)}
      facts={props.facts}
      tier={props.tier}
    />
  );
}

const TrackGlyph = ENTITY_KINDS.track.icon;
const EpisodeGlyph = ENTITY_KINDS.episode.icon;
const BookGlyph = ENTITY_KINDS.audiobook.icon;

/** What a playlist entry points at, drawn as that thing's own identity: a movie as a movie, a track as a track. */
export function EntryIdentity({ entry, tier = "row" }: { entry: PersonalEntryFragment; tier?: EntityTierProp }) {
  if (entry.movie) return <MovieIdentity movie={entry.movie} tier={tier} />;
  if (entry.show) return <ShowIdentity show={entry.show} tier={tier} />;
  if (entry.album) return <AlbumIdentity album={entry.album} tier={tier} />;
  if (entry.audiobook) return <AudiobookIdentity book={entry.audiobook} tier={tier} />;
  if (entry.track) {
    const track = entry.track;
    return thin({ kind: ENTITY_KINDS.track.noun, name: track.title, art: albumCover({ id: track.albumId }), glyph: <TrackGlyph />, mediaFileId: track.mediaFileId, tier, facts: [track.artistName ?? "", track.album?.name ?? "", track.durationSecs ? formatClock(track.durationSecs) : ""] });
  }
  if (entry.episode) {
    const episode = entry.episode;
    return thin({ kind: ENTITY_KINDS.episode.noun, name: episode.title ?? `Episode ${episode.episode}`, art: episodeThumb(episode.id), glyph: <EpisodeGlyph />, shape: "poster", mediaFileId: episode.mediaFileId, tier, facts: [episode.show?.name ?? "", episodeCode(episode), episode.runtime ? formatRuntime(episode.runtime, "minutes") : ""] });
  }
  if (entry.chapter) {
    const chapter = entry.chapter;
    return thin({ kind: "Chapter", name: chapter.title ?? `Chapter ${chapter.chapterNumber}`, art: audiobookCover({ id: chapter.audiobookId }), glyph: <BookGlyph />, mediaFileId: chapter.mediaFileId, tier, facts: [chapter.audiobook?.title ?? "", chapter.durationSecs ? formatClock(chapter.durationSecs) : ""] });
  }
  return <EntityIdentity kind="Item" name="Unavailable item" icon={<TrackGlyph />} state={{ id: "gone", label: "No longer in the library", tone: "warning" }} tier={tier} />;
}

/** The kind of thing an entry points at, in a word. */
export const entryKindLabel = (entry: PersonalEntryFragment) =>
  entry.movie ? ENTITY_KINDS.movie.noun : entry.show ? ENTITY_KINDS.show.noun : entry.episode ? ENTITY_KINDS.episode.noun
    : entry.album ? ENTITY_KINDS.album.noun : entry.track ? ENTITY_KINDS.track.noun : entry.audiobook ? ENTITY_KINDS.audiobook.noun
    : entry.chapter ? "Chapter" : "Unavailable";

/** How long an entry runs, where it knows: a movie's runtime, a track's length, an album's total. */
export function entryLength(entry: PersonalEntryFragment): string {
  const seconds = entry.track?.durationSecs ?? entry.chapter?.durationSecs ?? entry.album?.totalDurationSecs ?? entry.audiobook?.totalDurationSecs
    ?? (entry.movie?.runtime ? entry.movie.runtime * 60 : null) ?? (entry.episode?.runtime ? entry.episode.runtime * 60 : null);
  return seconds ? (seconds < 3600 ? formatClock(seconds) : formatRuntime(seconds)) : "";
}
