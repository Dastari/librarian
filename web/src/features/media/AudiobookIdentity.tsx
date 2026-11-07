import type { ActionSet } from "glass-ux/actions";
import { EntityIdentity, type EntityIdentityProps, type EntityState, type EntityTierProp } from "glass-ux/entity";
import type { ReactNode } from "react";

import { autoDownloadMeta } from "@/features/acquisition/mode";
import type { AutoDownloadMode } from "@/graphql/generated/graphql";
import { audiobookCover } from "@/lib/artwork";
import { sizedArtwork } from "@/lib/api/urls";
import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { formatRuntime } from "@/lib/format";

export interface AudiobookEntity {
  id: string;
  title: string;
  libraryId: string;
  userId: string;
  authorName?: string | null;
  narratorName?: string | null;
  totalDurationSecs?: number | null;
  chapterCount?: number | null;
  hasFiles: boolean;
  providerChapters?: string | null;
  audibleId?: string | null;
  chapterCheckState?: string | null;
  autoDownloadMode: AutoDownloadMode;
}

/** What a book is doing: being listened to first, then whether its files are here. */
export function audiobookState(book: AudiobookEntity): EntityState {
  if (book.chapterCheckState === "listening") return { id: "listening", label: "Listening for chapters", tone: "info", activity: true };
  if (book.hasFiles) return { id: "available", label: "Available", tone: "success" };
  const mode = autoDownloadMeta(book.autoDownloadMode);
  return book.autoDownloadMode === "NONE" ? { id: "missing", label: "No files yet", tone: "neutral" } : { id: "fetching", label: mode.label, tone: "info" };
}

/** What AudiobookIdentity draws, for surfaces that take an identity's props (a settings dialog's header). */
export function audiobookIdentityProps<T extends AudiobookEntity>(book: T, { actions }: {
  actions?: ActionSet<T>;
} = {}): EntityIdentityProps<T> {
  return {
    kind: ENTITY_KINDS.audiobook.noun,
    name: book.title,
    icon: <ENTITY_KINDS.audiobook.icon />,
    image: <img src={sizedArtwork(audiobookCover(book), 120)} alt="" />,
    state: audiobookState(book),
    facts: [book.authorName ?? "", book.narratorName ? `read by ${book.narratorName}` : "", book.totalDurationSecs ? formatRuntime(book.totalDurationSecs) : ""],
    target: book,
    actions,
  };
}

/** A book wherever it's named in a list or a heading: its cover or glyph, who wrote and read it, how long. */
export function AudiobookIdentity<T extends AudiobookEntity>({ book, tier = "row", actions, headerActions, heading, className, onOpen }: {
  book: T;
  tier?: EntityTierProp;
  actions?: ActionSet<T>;
  headerActions?: ReactNode;
  heading?: "h1" | "h2" | "h3" | "h4" | "p";
  className?: string;
  onOpen?: () => void;
}) {
  return (
    <EntityIdentity
      {...audiobookIdentityProps(book, { actions })}
      tier={tier}
      heading={heading}
      className={className}
      slots={headerActions ? { actions: headerActions } : undefined}
      onOpen={onOpen}
    />
  );
}
