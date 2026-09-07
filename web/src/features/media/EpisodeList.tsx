import { useMutation } from "@apollo/client/react";
import { toast } from "@heroui/react";
import { IconBookmark, IconBookmarkFilled, IconBookmarkOff, IconDownload, IconEye, IconEyeOff, IconPlayerPlayFilled } from "@tabler/icons-react";
import { useMemo, useState } from "react";

import { Artwork, Button, EmptyState, SkeletonBlock, StatusChip } from "@/components/ui";
import { ReleaseSearchDialog } from "@/features/downloads/ReleaseSearchDialog";
import { EntityEpisodeUpdateDocument, EntityEpisodeUpdateManyDocument, type ContentStatus, type PlaybackProgressFieldsFragment, type UpdateEpisodeInput } from "@/graphql/generated/graphql";
import { episodeThumb } from "@/lib/artwork";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatBytes, formatDate, formatRuntime } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { statusMeta } from "@/lib/status";
import { cn } from "@/lib/utils";

import { groupBySeason, type SeasonGroup } from "./seasons";
import type { EpisodeRow } from "./ShowPage";

interface EpisodeListProps {
  episodes: EpisodeRow[];
  statuses: Map<string, ContentStatus>;
  progressByFile: Map<string | null | undefined, PlaybackProgressFieldsFragment>;
  loading: boolean;
  onPlay: (episode: EpisodeRow) => void;
  showId: string;
  showName: string;
  libraryId: string;
  /** Called after a bulk or single update so the parent can refetch. */
  onChanged?: () => void;
}

/** Episode rows grouped by season, with season-wide actions and per-episode actions. */
export function EpisodeList({ episodes, statuses, progressByFile, loading, onPlay, showId, showName, libraryId, onChanged }: EpisodeListProps) {
  const isAdmin = useIsAdmin();
  const [updateEpisode] = useMutation(EntityEpisodeUpdateDocument);
  const [updateEpisodes, { loading: bulkPending }] = useMutation(EntityEpisodeUpdateManyDocument);
  const [searching, setSearching] = useState<EpisodeRow | null>(null);
  const groups = useMemo(() => groupBySeason(episodes), [episodes]);

  const toggleWanted = async (episode: EpisodeRow) => {
    try {
      const { data } = await updateEpisode({ variables: { id: episode.id, input: { wanted: !episode.wanted } } });
      assertSuccess(data?.updateEpisode, "Could not update episode");
      onChanged?.();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const setIgnored = async (episode: EpisodeRow, ignored: boolean) => {
    try {
      const { data } = await updateEpisode({ variables: { id: episode.id, input: ignored ? { ignored: true, wanted: false } : { ignored: false } } });
      assertSuccess(data?.updateEpisode, "Could not update episode");
      onChanged?.();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const bulk = async (season: number, input: UpdateEpisodeInput, missingOnly: boolean, message: string) => {
    try {
      const { data } = await updateEpisodes({
        variables: {
          where: missingOnly ? { showId: { eq: showId }, season: { eq: season }, mediaFileId: { isNull: true } } : { showId: { eq: showId }, season: { eq: season } },
          input,
        },
      });
      const result = assertSuccess(data?.updateEpisodes, "Could not update the season");
      toast.success(`${message} (${result.affectedCount})`);
      onChanged?.();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  if (loading) {
    return (
      <div className="flex flex-col gap-2">
        {Array.from({ length: 6 }, (_, index) => (
          <SkeletonBlock key={index} className="h-24 rounded-card" />
        ))}
      </div>
    );
  }
  if (episodes.length === 0) return <EmptyState compact icon={IconPlayerPlayFilled} title="No episodes in this season" />;

  return (
    <div className="flex flex-col gap-6">
      {groups.map((group) => (
        <section key={group.season} className="flex flex-col gap-2">
          <SeasonHeader group={group} isAdmin={isAdmin} pending={bulkPending} onBulk={bulk} />
          <ol className="flex flex-col gap-2">
            {group.episodes.map((episode) => {
              const saved = episode.mediaFileId ? progressByFile.get(episode.mediaFileId) : undefined;
              const fraction = saved?.isWatched ? 1 : saved && saved.duration ? saved.currentPosition / saved.duration : 0;
              const status = statusMeta(statuses.get(episode.id));
              const playable = Boolean(episode.mediaFileId);
              const ignored = episode.ignored === true;
              return (
                <li
                  key={episode.id}
                  className={cn(
                    "group flex gap-4 rounded-card border border-border bg-surface p-3 shadow-surface transition-colors",
                    playable && "hover:bg-surface-hover",
                    ignored && "opacity-55",
                  )}
                >
                  <button
                    type="button"
                    data-focusable
                    disabled={!playable}
                    aria-label={playable ? `Play episode ${episode.episode}` : undefined}
                    onClick={() => onPlay(episode)}
                    className="nav-focus relative w-36 shrink-0 overflow-hidden rounded-lg sm:w-44"
                  >
                    <Artwork src={playable ? episodeThumb(episode.id) : undefined} alt="" aspect="backdrop" tint="var(--media-tv)" fallback={<span className="text-numeric text-title-lg text-foreground/70">{episode.episode}</span>} />
                    {playable ? (
                      <span className="absolute inset-0 grid place-items-center bg-black/30 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100">
                        <span className="grid size-10 place-items-center rounded-full bg-brand text-brand-foreground">
                          <IconPlayerPlayFilled size={18} />
                        </span>
                      </span>
                    ) : null}
                    {fraction > 0 ? (
                      <span className="absolute inset-x-0 bottom-0 h-1 bg-white/20">
                        <span className="block h-full bg-brand" style={{ width: `${Math.min(100, fraction * 100)}%` }} />
                      </span>
                    ) : null}
                  </button>
                  <div className="flex min-w-0 flex-1 flex-col gap-1">
                    <div className="flex items-start justify-between gap-3">
                      <div className="min-w-0">
                        <p className="truncate text-title-sm text-foreground">
                          <span className="text-numeric mr-2 text-muted">{episode.episode}</span>
                          {episode.title ?? `Episode ${episode.episode}`}
                        </p>
                        <p className="text-label-sm text-muted">{[formatDate(episode.airDate), episode.runtime ? formatRuntime(episode.runtime, "minutes") : null, episode.mediaFile ? [episode.mediaFile.resolution, episode.mediaFile.videoCodec?.toUpperCase(), formatBytes(episode.mediaFile.size)].filter(Boolean).join(" · ") : null].filter(Boolean).join(" · ")}</p>
                      </div>
                      <StatusChip status={ignored ? { label: "Ignored", tone: "default", dot: "bg-muted" } : status} className="shrink-0" />
                    </div>
                    {episode.overview ? <p className="line-clamp-2 text-body-sm text-muted">{episode.overview}</p> : null}
                    {isAdmin && !playable ? (
                      <div className="mt-auto flex gap-1 pt-1">
                        {ignored ? (
                          <Button size="sm" variant="ghost" onPress={() => void setIgnored(episode, false)}>
                            <IconEye size={14} /> Unignore
                          </Button>
                        ) : (
                          <>
                            <Button size="sm" variant="ghost" onPress={() => void toggleWanted(episode)}>
                              {episode.wanted ? <IconBookmarkFilled size={14} className="text-brand" /> : <IconBookmark size={14} />} {episode.wanted ? "Wanted" : "Want"}
                            </Button>
                            <Button size="sm" variant="ghost" onPress={() => setSearching(episode)}>
                              <IconDownload size={14} /> Find
                            </Button>
                            <Button size="sm" variant="ghost" onPress={() => void setIgnored(episode, true)}>
                              <IconEyeOff size={14} /> Ignore
                            </Button>
                          </>
                        )}
                      </div>
                    ) : null}
                  </div>
                </li>
              );
            })}
          </ol>
        </section>
      ))}
      <ReleaseSearchDialog isOpen={Boolean(searching)} onOpenChange={(open) => !open && setSearching(null)} query={showName} season={searching?.season} episode={searching?.episode} libraryId={libraryId} target={{ showId, episodeId: searching?.id }} />
    </div>
  );
}

interface SeasonHeaderProps {
  group: SeasonGroup<EpisodeRow>;
  isAdmin: boolean;
  pending: boolean;
  onBulk: (season: number, input: UpdateEpisodeInput, missingOnly: boolean, message: string) => void | Promise<void>;
}

/** Season summary with the actions that apply to every episode in it. */
function SeasonHeader({ group, isAdmin, pending, onBulk }: SeasonHeaderProps) {
  return (
    <header className="flex flex-wrap items-center gap-x-3 gap-y-2 px-1">
      <h3 className="text-title-sm text-foreground">{group.label}</h3>
      <span className="text-numeric text-label-sm text-muted">
        {group.have} of {group.total} · {group.missing} missing
        {group.wanted > 0 ? ` · ${group.wanted} wanted` : ""}
        {group.ignored > 0 ? ` · ${group.ignored} ignored` : ""}
      </span>
      {isAdmin ? (
        <div className="ml-auto flex flex-wrap items-center gap-1">
          <Button size="sm" variant="ghost" isDisabled={pending || group.missing === 0} onPress={() => void onBulk(group.season, { wanted: true, ignored: false }, true, `${group.label}: missing episodes wanted`)}>
            <IconBookmark size={14} /> Want all missing
          </Button>
          <Button size="sm" variant="ghost" isDisabled={pending} onPress={() => void onBulk(group.season, { wanted: false }, false, `${group.label}: no longer wanted`)}>
            <IconBookmarkOff size={14} /> Unwant all
          </Button>
          {group.allIgnored ? (
            <Button size="sm" variant="ghost" isDisabled={pending} onPress={() => void onBulk(group.season, { ignored: false }, false, `${group.label}: no longer ignored`)}>
              <IconEye size={14} /> Unignore season
            </Button>
          ) : (
            <Button size="sm" variant="ghost" isDisabled={pending} onPress={() => void onBulk(group.season, { ignored: true, wanted: false }, false, `${group.label}: ignored`)}>
              <IconEyeOff size={14} /> Ignore season
            </Button>
          )}
        </div>
      ) : null}
    </header>
  );
}
