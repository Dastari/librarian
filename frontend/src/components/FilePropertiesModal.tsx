import { useState, useEffect } from "react";
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
} from "@heroui/modal";
import { Button } from "@heroui/button";
import { Chip } from "@heroui/chip";
import { Spinner } from "@heroui/spinner";
import { Tabs, Tab } from "@heroui/tabs";
import { Tooltip } from "@heroui/tooltip";
import {
  IconFile,
  IconVideo,
  IconVolume,
  IconFileText,
  IconList,
  IconCopy,
  IconCheck,
  IconAlertCircle,
  IconInfoCircle,
  IconTags,
} from "@tabler/icons-react";
import { formatBytes } from "../lib/format";
import {
  MediaFileMetadataDocument,
  MediaFilePropertiesDocument,
  type MediaFileMetadataQuery,
  type MediaFileMetadataQueryVariables,
  type MediaFilePropertiesQuery,
  type MediaFilePropertiesQueryVariables,
} from "../lib/graphql/generated/graphql";
import { apolloClient } from "../lib/graphql/client";
import {
  JsonView,
  allExpanded,
  darkStyles,
  defaultStyles,
} from "react-json-view-lite";
import "react-json-view-lite/dist/index.css";
import { useTheme } from "../hooks/useTheme";

interface FilePropertiesModalProps {
  isOpen: boolean;
  onClose: () => void;
  /** Media file ID to fetch details for */
  mediaFileId: string | null;
  /** Optional title override (e.g., episode name) */
  title?: string;
}

type VideoStreamNode = NonNullable<
  MediaFilePropertiesQuery["VideoStreams"]
>["Edges"][number]["Node"];
type AudioStreamNode = NonNullable<
  MediaFilePropertiesQuery["AudioStreams"]
>["Edges"][number]["Node"];
type SubtitleNode = NonNullable<
  MediaFilePropertiesQuery["Subtitles"]
>["Edges"][number]["Node"];
type ChapterNode = NonNullable<
  MediaFilePropertiesQuery["MediaChapters"]
>["Edges"][number]["Node"];

/** Format duration from seconds to HH:MM:SS */
function formatDuration(seconds: number | null): string {
  if (seconds === null || seconds === undefined) return "-";
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  if (h > 0) {
    return `${h}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  }
  return `${m}:${s.toString().padStart(2, "0")}`;
}

/** Format bitrate to human readable */
function formatBitrate(bps: number | null): string {
  if (bps === null || bps === undefined) return "-";
  if (bps >= 1000000) {
    return `${(bps / 1000000).toFixed(1)} Mbps`;
  }
  if (bps >= 1000) {
    return `${(bps / 1000).toFixed(0)} Kbps`;
  }
  return `${bps} bps`;
}

/** Format sample rate */
function formatSampleRate(hz: number | null): string {
  if (hz === null || hz === undefined) return "-";
  if (hz >= 1000) {
    return `${(hz / 1000).toFixed(1)} kHz`;
  }
  return `${hz} Hz`;
}

function parseJsonMetadata(value: string | null): unknown | null {
  if (!value) return null;
  try {
    return JSON.parse(value);
  } catch {
    return value;
  }
}

/** Get language display name */
function getLanguageName(code: string | null): string {
  if (!code) return "Unknown";
  try {
    const displayName = new Intl.DisplayNames(["en"], { type: "language" });
    return displayName.of(code) || code;
  } catch {
    return code;
  }
}

/** Format video codec for display */
function formatVideoCodec(codec: string): string {
  const normalized = codec.toLowerCase();
  if (normalized.includes("hevc") || normalized === "h265")
    return "HEVC (H.265)";
  if (normalized.includes("h264") || normalized === "avc") return "H.264 (AVC)";
  if (normalized.includes("av1")) return "AV1";
  if (normalized.includes("vp9")) return "VP9";
  if (normalized.includes("mpeg4")) return "MPEG-4";
  if (normalized.includes("mpeg2")) return "MPEG-2";
  return codec.toUpperCase();
}

/** Format audio codec for display */
function formatAudioCodec(codec: string): string {
  const normalized = codec.toLowerCase();
  if (normalized.includes("truehd")) return "Dolby TrueHD";
  if (normalized.includes("atmos")) return "Dolby Atmos";
  if (normalized.includes("dts-hd")) return "DTS-HD MA";
  if (normalized.includes("dts")) return "DTS";
  if (normalized.includes("ac3") || normalized.includes("ac-3"))
    return "Dolby Digital (AC3)";
  if (normalized.includes("eac3") || normalized.includes("e-ac-3"))
    return "Dolby Digital Plus (EAC3)";
  if (normalized.includes("aac")) return "AAC";
  if (normalized.includes("flac")) return "FLAC";
  if (normalized.includes("opus")) return "Opus";
  if (normalized.includes("pcm")) return "PCM (Lossless)";
  if (normalized.includes("mp3")) return "MP3";
  return codec.toUpperCase();
}

/** Property row component */
function PropertyRow({
  label,
  value,
  mono = false,
}: {
  label: string;
  value: React.ReactNode;
  mono?: boolean;
}) {
  return (
    <div className="flex justify-between items-center py-2 border-b border-default-100/50 last:border-0">
      <span className="text-default-400 text-sm">{label}</span>
      <span
        className={`text-sm text-right max-w-[60%] truncate text-default-foreground ${mono ? "font-mono text-xs" : ""}`}
      >
        {value || <span className="text-default-300 italic">—</span>}
      </span>
    </div>
  );
}

/** Stream card for video/audio streams */
function StreamCard({
  icon,
  title,
  subtitle,
  badges,
  children,
}: {
  icon: React.ReactNode;
  title: string;
  subtitle?: string;
  badges?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div className="bg-default-100/50 rounded-xl p-4 mb-3 last:mb-0 border border-default-200/30">
      <div className="flex items-start gap-3 mb-3">
        <div className="text-primary-400 mt-0.5">{icon}</div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            <span className="font-semibold text-sm text-default-foreground">
              {title}
            </span>
            {badges}
          </div>
          {subtitle && (
            <span className="text-xs text-default-400 block mt-0.5">
              {subtitle}
            </span>
          )}
        </div>
      </div>
      <div className="pl-7">{children}</div>
    </div>
  );
}

export function FilePropertiesModal({
  isOpen,
  onClose,
  mediaFileId,
  title: overrideTitle,
}: FilePropertiesModalProps) {
  const { isDark } = useTheme();
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [details, setDetails] = useState<MediaFilePropertiesQuery | null>(null);
  const [rawMetadata, setRawMetadata] = useState<unknown | null>(null);
  const [copied, setCopied] = useState(false);
  const [selectedTab, setSelectedTab] = useState("overview");

  useEffect(() => {
    if (!isOpen || !mediaFileId) {
      setDetails(null);
      setRawMetadata(null);
      setError(null);
      return;
    }

    const fetchDetails = async () => {
      setLoading(true);
      setError(null);

      const [result, metadataResult] = await Promise.all([
        apolloClient.query<
          MediaFilePropertiesQuery,
          MediaFilePropertiesQueryVariables
        >({
          query: MediaFilePropertiesDocument,
          variables: { Id: mediaFileId },
          fetchPolicy: "network-only",
        }),
        apolloClient.query<
          MediaFileMetadataQuery,
          MediaFileMetadataQueryVariables
        >({
          query: MediaFileMetadataDocument,
          variables: { Id: mediaFileId },
          fetchPolicy: "network-only",
        }),
      ]);

      if (result.error) {
        setError(result.error.message);
        setRawMetadata(null);
      } else if (result.data?.MediaFile) {
        const parsedRawMetadata = parseJsonMetadata(
          metadataResult.data?.MediaFile?.Metadata ?? null,
        );
        setDetails(result.data);
        setRawMetadata(parsedRawMetadata);
      } else {
        setError("Media file not found or not yet analyzed");
        setRawMetadata(null);
      }
      setLoading(false);
    };

    fetchDetails();
  }, [isOpen, mediaFileId]);

  const handleCopyPath = async () => {
    if (details?.MediaFile?.Path) {
      await navigator.clipboard.writeText(details.MediaFile.Path);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const file = details?.MediaFile;
  const videoStreams = details?.VideoStreams?.Edges?.map((edge) => edge.Node) ?? [];
  const audioStreams = details?.AudioStreams?.Edges?.map((edge) => edge.Node) ?? [];
  const subtitles = details?.Subtitles?.Edges?.map((edge) => edge.Node) ?? [];
  const chapters = details?.MediaChapters?.Edges?.map((edge) => edge.Node) ?? [];
  const filename =
    file?.Path.split("/").pop() || file?.OriginalName || "Unknown";

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      size="2xl"
      scrollBehavior="inside"
      classNames={{
        base: "bg-content1",
        header: "border-b border-default-200/50",
        body: "py-4",
        footer: "border-t border-default-200/50",
      }}
    >
      <ModalContent>
        <ModalHeader className="flex flex-col gap-1">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-primary-500/10 rounded-lg">
              <IconFile size={20} className="text-primary-400" />
            </div>
            <div className="flex-1 min-w-0">
              <span className="truncate block font-semibold">
                {overrideTitle || filename}
              </span>
              {overrideTitle && file && (
                <span className="text-xs text-default-400 font-normal truncate block mt-0.5">
                  {filename}
                </span>
              )}
            </div>
          </div>
        </ModalHeader>

        <ModalBody>
          {loading ? (
            <div className="flex items-center justify-center py-12">
              <Spinner size="lg" />
            </div>
          ) : error ? (
            <div className="flex flex-col items-center justify-center py-12 gap-3">
              <IconAlertCircle size={48} className="text-warning-400" />
              <p className="text-default-500 text-center">{error}</p>
              <p className="text-default-400 text-sm text-center">
                The file may not have been analyzed yet. Try rescanning the
                library.
              </p>
            </div>
          ) : details ? (
            <Tabs
              selectedKey={selectedTab}
              onSelectionChange={(key) => setSelectedTab(key as string)}
              variant="underlined"
              color="primary"
              classNames={{
                tabList:
                  "gap-6 w-full relative rounded-none p-0 border-b border-default-200/50",
                cursor: "w-full bg-primary-500",
                tab: "max-w-fit px-0 h-10",
                tabContent: "group-data-[selected=true]:text-primary-500",
              }}
            >
              <Tab
                key="overview"
                title={
                  <div className="flex items-center gap-1.5">
                    <IconInfoCircle size={16} />
                    <span>Overview</span>
                  </div>
                }
              >
                <div className="pt-4 space-y-6">
                  {/* Media Summary Badges - show first as visual highlight */}
                  <div className="flex flex-wrap gap-2">
                    {file?.Resolution && (
                      <Chip
                        size="md"
                        variant="flat"
                        color="primary"
                        classNames={{ content: "font-semibold" }}
                      >
                        {file.Resolution}
                      </Chip>
                    )}
                    {file?.VideoCodec && (
                      <Chip
                        size="md"
                        variant="flat"
                        color="secondary"
                        classNames={{ content: "font-semibold" }}
                      >
                        {formatVideoCodec(file.VideoCodec)}
                      </Chip>
                    )}
                    {file?.HdrType && (
                      <Chip
                        size="md"
                        variant="flat"
                        color="warning"
                        classNames={{ content: "font-semibold" }}
                      >
                        {file.HdrType}
                      </Chip>
                    )}
                    {file?.AudioCodec && (
                      <Chip
                        size="md"
                        variant="flat"
                        color="default"
                        classNames={{ content: "font-medium" }}
                      >
                        {formatAudioCodec(file.AudioCodec)}
                      </Chip>
                    )}
                  </div>

                  {/* File Info Section */}
                  <div className="bg-default-100/30 rounded-xl p-4 border border-default-200/30">
                    <h4 className="text-sm font-semibold text-primary-400 mb-3 uppercase tracking-wide">
                      File Information
                    </h4>
                    <PropertyRow label="File Name" value={filename} />
                    {file?.OriginalName && file.OriginalName !== filename && (
                      <PropertyRow
                        label="Original Name"
                        value={file.OriginalName}
                      />
                    )}
                    <PropertyRow label="Size" value={file ? formatBytes(file.Size) : null} />
                    <PropertyRow
                      label="Container"
                      value={file?.Container?.toUpperCase()}
                    />
                    <PropertyRow
                      label="Duration"
                      value={formatDuration(file?.Duration ?? null)}
                    />
                    <PropertyRow
                      label="Overall Bitrate"
                      value={formatBitrate(file?.Bitrate ?? null)}
                    />
                    <PropertyRow
                      label="Added"
                      value={
                        file?.AddedAt
                          ? new Date(file.AddedAt).toLocaleString()
                          : null
                      }
                    />
                  </div>

                  {/* Path Section */}
                  <div className="bg-default-100/30 rounded-xl p-4 border border-default-200/30">
                    <div className="flex items-center justify-between mb-2">
                      <h4 className="text-sm font-semibold text-primary-400 uppercase tracking-wide">
                        File Path
                      </h4>
                      <Tooltip content={copied ? "Copied!" : "Copy path"}>
                        <Button
                          size="sm"
                          variant="flat"
                          color={copied ? "success" : "default"}
                          isIconOnly
                          onPress={handleCopyPath}
                        >
                          {copied ? (
                            <IconCheck size={14} />
                          ) : (
                            <IconCopy size={14} />
                          )}
                        </Button>
                      </Tooltip>
                    </div>
                    <code className="text-xs text-default-400 break-all block border p-3 rounded-lg font-mono">
                      {file?.Path}
                    </code>
                  </div>

                  {/* Stream Counts */}
                  <div className="grid grid-cols-4 gap-3">
                    <div className="bg-default-100/30 rounded-lg p-3 text-center border border-default-200/30">
                      <div className="text-2xl font-bold text-default-foreground">
                        {videoStreams.length}
                      </div>
                      <div className="text-xs text-default-400 mt-1">Video</div>
                    </div>
                    <div className="bg-default-100/30 rounded-lg p-3 text-center border border-default-200/30">
                      <div className="text-2xl font-bold text-default-foreground">
                        {audioStreams.length}
                      </div>
                      <div className="text-xs text-default-400 mt-1">Audio</div>
                    </div>
                    <div className="bg-default-100/30 rounded-lg p-3 text-center border border-default-200/30">
                      <div className="text-2xl font-bold text-default-foreground">
                        {subtitles.length}
                      </div>
                      <div className="text-xs text-default-400 mt-1">
                        Subtitles
                      </div>
                    </div>
                    <div className="bg-default-100/30 rounded-lg p-3 text-center border border-default-200/30">
                      <div className="text-2xl font-bold text-default-foreground">
                        {chapters.length}
                      </div>
                      <div className="text-xs text-default-400 mt-1">
                        Chapters
                      </div>
                    </div>
                  </div>
                </div>
              </Tab>

              <Tab
                key="video"
                title={
                  <div className="flex items-center gap-1.5">
                    <IconVideo size={16} />
                    <span>Video ({videoStreams.length})</span>
                  </div>
                }
              >
                <div className="pt-4">
                  {videoStreams.length === 0 ? (
                    <div className="flex flex-col items-center justify-center py-12 text-default-400">
                      <IconVideo size={48} className="mb-2 opacity-50" />
                      <p>No video streams found</p>
                    </div>
                  ) : (
                    <div className="space-y-3">
                      {videoStreams.map((stream) => (
                        <VideoStreamCard key={stream.Id} stream={stream} />
                      ))}
                    </div>
                  )}
                </div>
              </Tab>

              <Tab
                key="audio"
                title={
                  <div className="flex items-center gap-1.5">
                    <IconVolume size={16} />
                    <span>Audio ({audioStreams.length})</span>
                  </div>
                }
              >
                <div className="pt-4">
                  {audioStreams.length === 0 ? (
                    <div className="flex flex-col items-center justify-center py-12 text-default-400">
                      <IconVolume size={48} className="mb-2 opacity-50" />
                      <p>No audio streams found</p>
                    </div>
                  ) : (
                    <div className="space-y-3">
                      {audioStreams.map((stream) => (
                        <AudioStreamCard key={stream.Id} stream={stream} />
                      ))}
                    </div>
                  )}
                </div>
              </Tab>

              <Tab
                key="subtitles"
                title={
                  <div className="flex items-center gap-1.5">
                    <IconFileText size={16} />
                    <span>Subtitles ({subtitles.length})</span>
                  </div>
                }
              >
                <div className="pt-4">
                  {subtitles.length === 0 ? (
                    <div className="flex flex-col items-center justify-center py-12 text-default-400">
                      <IconFileText size={48} className="mb-2 opacity-50" />
                      <p>No subtitles found</p>
                    </div>
                  ) : (
                    <div className="space-y-3">
                      {subtitles.map((sub) => (
                        <SubtitleCard key={sub.Id} subtitle={sub} />
                      ))}
                    </div>
                  )}
                </div>
              </Tab>

              <Tab
                key="chapters"
                title={
                  <div className="flex items-center gap-1.5">
                    <IconList size={16} />
                    <span>Chapters ({chapters.length})</span>
                  </div>
                }
              >
                <div className="pt-4">
                  {chapters.length === 0 ? (
                    <div className="flex flex-col items-center justify-center py-12 text-default-400">
                      <IconList size={48} className="mb-2 opacity-50" />
                      <p>No chapters found</p>
                    </div>
                  ) : (
                    <div className="space-y-2">
                      {chapters.map((chapter) => (
                        <ChapterRow key={chapter.Id} chapter={chapter} />
                      ))}
                    </div>
                  )}
                </div>
              </Tab>

              <Tab
                key="metadata"
                title={
                  <div className="flex items-center gap-1.5">
                    <IconTags size={16} />
                    <span>Metadata</span>
                  </div>
                }
              >
                <div className="pt-4">
                  <MetadataTab
                    rawMetadata={rawMetadata}
                    isDark={isDark}
                  />
                </div>
              </Tab>
            </Tabs>
          ) : null}
        </ModalBody>

        <ModalFooter>
          <Button variant="flat" onPress={onClose}>
            Close
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
}

/** Video stream details card */
function VideoStreamCard({ stream }: { stream: VideoStreamNode }) {
  return (
    <StreamCard
      icon={<IconVideo size={16} />}
      title={formatVideoCodec(stream.Codec)}
      subtitle={stream.CodecLongName || undefined}
      badges={
        <>
          {stream.IsDefault && (
            <Chip size="sm" variant="flat" color="primary">
              Default
            </Chip>
          )}
          {stream.HdrType && (
            <Chip size="sm" variant="flat" color="warning">
              {stream.HdrType}
            </Chip>
          )}
        </>
      }
    >
      <div className="grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
        <PropertyRow
          label="Resolution"
          value={`${stream.Width}x${stream.Height}`}
        />
        <PropertyRow label="Aspect Ratio" value={stream.AspectRatio} />
        <PropertyRow label="Frame Rate" value={stream.FrameRate} />
        <PropertyRow
          label="Bitrate"
          value={formatBitrate(stream.Bitrate ?? null)}
        />
        <PropertyRow label="Pixel Format" value={stream.PixelFormat} />
        <PropertyRow
          label="Bit Depth"
          value={stream.BitDepth ? `${stream.BitDepth}-bit` : null}
        />
        {stream.Language && (
          <PropertyRow
            label="Language"
            value={getLanguageName(stream.Language)}
          />
        )}
        {stream.Title && <PropertyRow label="Title" value={stream.Title} />}
      </div>
    </StreamCard>
  );
}

/** Audio stream details card */
function AudioStreamCard({ stream }: { stream: AudioStreamNode }) {
  return (
    <StreamCard
      icon={<IconVolume size={16} />}
      title={formatAudioCodec(stream.Codec)}
      subtitle={stream.CodecLongName || undefined}
      badges={
        <>
          {stream.IsDefault && (
            <Chip size="sm" variant="flat" color="primary">
              Default
            </Chip>
          )}
          {stream.IsCommentary && (
            <Chip size="sm" variant="flat" color="secondary">
              Commentary
            </Chip>
          )}
          {stream.Language && (
            <Chip size="sm" variant="flat" color="default">
              {getLanguageName(stream.Language)}
            </Chip>
          )}
        </>
      }
    >
      <div className="grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
        <PropertyRow
          label="Channels"
          value={stream.ChannelLayout || `${stream.Channels} ch`}
        />
        <PropertyRow
          label="Sample Rate"
          value={formatSampleRate(stream.SampleRate ?? null)}
        />
        <PropertyRow
          label="Bitrate"
          value={formatBitrate(stream.Bitrate ?? null)}
        />
        <PropertyRow
          label="Bit Depth"
          value={stream.BitDepth ? `${stream.BitDepth}-bit` : null}
        />
        {stream.Title && <PropertyRow label="Title" value={stream.Title} />}
      </div>
    </StreamCard>
  );
}

/** Subtitle track card */
function SubtitleCard({ subtitle }: { subtitle: SubtitleNode }) {
  const sourceLabel =
    {
      EMBEDDED: "Embedded",
      EXTERNAL: "External File",
      DOWNLOADED: "Downloaded",
    }[subtitle.SourceType ?? "EMBEDDED"] || subtitle.SourceType;

  return (
    <StreamCard
      icon={<IconFileText size={16} />}
      title={
        subtitle.Language
          ? getLanguageName(subtitle.Language)
          : "Unknown Language"
      }
      subtitle={subtitle.Codec || undefined}
      badges={
        <>
          <Chip size="sm" variant="flat" color="default">
            {sourceLabel}
          </Chip>
          {subtitle.IsDefault && (
            <Chip size="sm" variant="flat" color="primary">
              Default
            </Chip>
          )}
          {subtitle.IsForced && (
            <Chip size="sm" variant="flat" color="warning">
              Forced
            </Chip>
          )}
          {subtitle.IsHearingImpaired && (
            <Chip size="sm" variant="flat" color="secondary">
              SDH
            </Chip>
          )}
        </>
      }
    >
      <div className="text-xs">
        {subtitle.Title && <PropertyRow label="Title" value={subtitle.Title} />}
        {subtitle.FilePath && (
          <PropertyRow
            label="File"
            value={subtitle.FilePath.split("/").pop()}
          />
        )}
      </div>
    </StreamCard>
  );
}

/** Chapter row */
function ChapterRow({ chapter }: { chapter: ChapterNode }) {
  const duration = chapter.EndSecs - chapter.StartSecs;
  return (
    <div className="flex items-center gap-3 py-2.5 px-4 bg-default-100/30 rounded-lg hover:bg-default-100/50 transition-colors border border-default-200/20">
      <span className="text-default-400 text-xs w-6 font-medium">
        {chapter.ChapterIndex + 1}
      </span>
      <span className="flex-1 text-sm truncate text-default-foreground">
        {chapter.Title || `Chapter ${chapter.ChapterIndex + 1}`}
      </span>
      <span className="text-xs text-default-400 font-mono tabular-nums">
        {formatDuration(chapter.StartSecs)}
      </span>
      <span className="text-xs text-default-300">→</span>
      <span className="text-xs text-default-400 font-mono tabular-nums">
        {formatDuration(chapter.EndSecs)}
      </span>
      <span className="text-xs text-primary-400 w-16 text-right font-medium tabular-nums">
        ({formatDuration(duration)})
      </span>
    </div>
  );
}

/** Metadata tab showing raw ffprobe metadata and legacy embedded tags */
function MetadataTab({
  rawMetadata,
  isDark,
}: {
  rawMetadata: unknown | null;
  isDark: boolean;
}) {
  const hasRawMetadata = rawMetadata !== null && rawMetadata !== undefined;

  if (!hasRawMetadata) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-default-400">
        <IconTags size={48} className="mb-2 opacity-50" />
        <p>No metadata available</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {hasRawMetadata && (
        <div className="p-4">
          <h4 className="text-sm font-semibold text-primary-400 mb-3 uppercase tracking-wide">
            Raw FFprobe JSON
          </h4>
          <div className="max-h-[28rem] overflow-auto rounded-lg border border-default-200/40 bg-content2 p-3">
            {typeof rawMetadata === "string" ? (
              <pre className="text-xs whitespace-pre-wrap text-default-700">
                {rawMetadata}
              </pre>
            ) : (
              <JsonView
                data={rawMetadata as object}
                shouldExpandNode={allExpanded}
                style={isDark ? darkStyles : defaultStyles}
              />
            )}
          </div>
        </div>
      )}

    </div>
  );
}

export default FilePropertiesModal;
