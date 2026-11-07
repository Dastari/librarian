import { useNavigate } from "@tanstack/react-router";

import { PosterCard } from "@/components/ui";
import { usePlayer } from "@/features/player/usePlayer";
import { formatClock } from "@/lib/format";

import type { ResumeItem } from "./useContinueWatching";

/**
 * Something part-watched or part-heard, as a poster like the rest of the shelves: its own cover,
 * the time left over it and progress along the bottom. Video picks up in the player; music and
 * books open their page.
 */
export function ResumeCard({ item }: { item: ResumeItem }) {
  const navigate = useNavigate();
  const player = usePlayer();
  const left = item.duration ? `${formatClock(Math.max(0, item.duration - item.position))} left` : undefined;
  return (
    <PosterCard
      width="row"
      aspect={item.posterAspect}
      title={item.title}
      meta={item.subtitle}
      image={item.poster}
      progress={item.progress}
      badge={left}
      onPress={() => {
        if (item.kind === "movie" || item.kind === "episode") {
          player.playVideo({ mediaFileId: item.mediaFileId, title: item.title, subtitle: item.subtitle, artwork: item.image, entity: item.entity, href: item.href, startPosition: item.position });
          void navigate({ to: "/watch/$mediaFileId", params: { mediaFileId: item.mediaFileId } });
        } else {
          void navigate({ href: item.href });
        }
      }}
    />
  );
}
