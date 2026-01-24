import { Chip } from "@heroui/chip";
import { Progress } from "@heroui/progress";

type ChipColor =
  | "success"
  | "warning"
  | "danger"
  | "default"
  | "primary"
  | "secondary";

/**
 * Derived status based on mediaFileId presence.
 * Uses lowercase for consistency across media types.
 */
export type DerivedMediaStatus = "downloaded" | "downloading" | "wanted";

interface StatusConfig {
  color: ChipColor;
  label: string;
}

const STATUS_CONFIG: Record<DerivedMediaStatus, StatusConfig> = {
  downloaded: { color: "success", label: "Downloaded" },
  downloading: { color: "primary", label: "Downloading" },
  wanted: { color: "warning", label: "Wanted" },
};

/**
 * Derive media item status from mediaFileId and optional download progress.
 *
 * - If mediaFileId is set, status is 'downloaded'
 * - If downloading (has progress), status is 'downloading'
 * - Otherwise status is 'wanted'
 */
export function deriveMediaStatus(
  mediaFileId: string | null | undefined,
  downloadProgress?: number | null
): DerivedMediaStatus {
  if (mediaFileId) return "downloaded";
  if (downloadProgress != null && downloadProgress > 0) return "downloading";
  return "wanted";
}

/**
 * Get the color for a media status (for use in other contexts)
 */
export function getMediaStatusColor(status: DerivedMediaStatus): ChipColor {
  return STATUS_CONFIG[status]?.color ?? "default";
}

/**
 * Get the label for a media status
 */
export function getMediaStatusLabel(status: DerivedMediaStatus): string {
  return STATUS_CONFIG[status]?.label ?? status;
}

interface MediaItemStatusChipProps {
  /** Media file ID - if set, item is downloaded */
  mediaFileId?: string | null;
  /** Size of the chip */
  size?: "sm" | "md" | "lg";
  /** Download progress (0.0 to 1.0) when downloading */
  downloadProgress?: number | null;
}

/**
 * A unified status chip for displaying media item status consistently across the app.
 *
 * Works for episodes, tracks, chapters, and any other media items.
 * Status is derived from mediaFileId: present = Downloaded, absent = Wanted.
 * Shows a progress bar when downloading with progress info.
 */
export function MediaItemStatusChip({
  mediaFileId,
  size = "sm",
  downloadProgress,
}: MediaItemStatusChipProps) {
  const status = deriveMediaStatus(mediaFileId, downloadProgress);
  const config = STATUS_CONFIG[status];

  // Show progress bar when downloading with progress info
  if (status === "downloading" && downloadProgress != null) {
    const percent = Math.round(downloadProgress * 100);
    return (
      <div className="flex items-center gap-2 min-w-[100px]">
        <Progress
          size="sm"
          value={percent}
          color="primary"
          aria-label={`Download progress: ${percent}%`}
          classNames={{
            track: "h-2",
            indicator: "h-2",
          }}
        />
        <span className="text-xs text-default-500 whitespace-nowrap">
          {percent}%
        </span>
      </div>
    );
  }

  return (
    <Chip size={size} color={config.color} variant="flat">
      {config.label}
    </Chip>
  );
}

// ============================================================================
// Backwards-compatible aliases
// ============================================================================

/** @deprecated Use MediaItemStatusChip instead */
export type DerivedEpisodeStatus = DerivedMediaStatus;
/** @deprecated Use MediaItemStatusChip instead */
export type DerivedTrackStatus = DerivedMediaStatus;
/** @deprecated Use MediaItemStatusChip instead */
export type DerivedChapterStatus = DerivedMediaStatus;

/** @deprecated Use deriveMediaStatus instead */
export const deriveEpisodeStatus = deriveMediaStatus;
/** @deprecated Use deriveMediaStatus instead */
export const deriveTrackStatus = deriveMediaStatus;
/** @deprecated Use deriveMediaStatus instead */
export const deriveChapterStatus = deriveMediaStatus;

/** @deprecated Use getMediaStatusColor instead */
export const getEpisodeStatusColor = getMediaStatusColor;
/** @deprecated Use getMediaStatusColor instead */
export const getTrackStatusColor = getMediaStatusColor;
/** @deprecated Use getMediaStatusColor instead */
export const getChapterStatusColor = getMediaStatusColor;

/** @deprecated Use getMediaStatusLabel instead */
export const getEpisodeStatusLabel = getMediaStatusLabel;
/** @deprecated Use getMediaStatusLabel instead */
export const getTrackStatusLabel = getMediaStatusLabel;
/** @deprecated Use getMediaStatusLabel instead */
export const getChapterStatusLabel = getMediaStatusLabel;

/** @deprecated Use MediaItemStatusChip instead */
export const EpisodeStatusChip = MediaItemStatusChip;
/** @deprecated Use MediaItemStatusChip instead */
export const TrackStatusChip = MediaItemStatusChip;
/** @deprecated Use MediaItemStatusChip instead */
export const ChapterStatusChip = MediaItemStatusChip;
