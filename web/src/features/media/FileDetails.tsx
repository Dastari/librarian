import { CopyButton } from "glass-ux/copy-button";
import { IconCalendar, IconClock, IconDatabase, IconFolder, IconRefresh, IconSearch, IconVideo, IconVolume } from "glass-ux/icons";
import { TruncatedText } from "glass-ux/truncated-text";

import { DetailGrid, DetailTile, type DetailSection } from "@/components/entities/EntityDetailDialog";
import { Panel } from "@/components/ui";
import { openFileDialog } from "@/features/files/fileDialog";
import { FileIdentity } from "@/features/files/FileIdentity";
import { useFileActions, type FileEntity } from "@/features/files/useFileActions";
import { MediaFileStreamsDocument, type MediaFileFieldsFragment } from "@/graphql/generated/graphql";
import { formatBytes, formatDateTime, formatRuntime } from "@/lib/format";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

type Streams = ReturnType<typeof useFileStreams>;

export function useFileStreams(fileId: string) {
  const streams = useQuery(MediaFileStreamsDocument, { variables: { mediaFileId: fileId } });
  const data = streams.data;
  const video = (data?.videoStreams.edges ?? []).map(({ node: stream }) => ({
    key: stream.id,
    title: [stream.codec.toUpperCase(), `${stream.width}×${stream.height}`].join(" · "),
    meta: [stream.frameRate ? `${frameRate(stream.frameRate)} fps` : null, stream.hdrType].filter(Boolean).join(" · "),
  }));
  const audio = (data?.audioStreams.edges ?? []).map(({ node: stream }) => ({
    key: stream.id,
    title: [stream.codec.toUpperCase(), stream.channelLayout ?? `${stream.channels}ch`].join(" · "),
    meta: [stream.language, stream.isCommentary ? "Commentary" : null].filter(Boolean).join(" · "),
  }));
  const subtitles = (data?.subtitles.edges ?? []).map(({ node: subtitle }) => ({
    key: subtitle.id,
    title: subtitle.title || subtitle.language || subtitle.codec || "Track",
    meta: [subtitle.language && subtitle.title ? subtitle.language : null, subtitle.isForced ? "Forced" : null, subtitle.isHearingImpaired ? "SDH" : null].filter(Boolean).join(" · "),
  }));
  return { video, audio, subtitles, count: video.length + audio.length + subtitles.length, loading: streams.loading };
}

/** "24000/1001" as the 23.976 people expect, and "24/1" as 24. */
function frameRate(value: string): string {
  const [top, bottom] = value.split("/").map(Number);
  if (!top) return value;
  return bottom ? String(Math.round((top / bottom) * 1000) / 1000) : String(top);
}

/** The facts someone opens a file to see. */
export function FileSummary({ file }: { file: MediaFileFieldsFragment }) {
  const video = [file.videoCodec?.toUpperCase(), file.isHdr ? file.hdrType ?? "HDR" : null].filter(Boolean).join(" · ");
  return (
    <DetailGrid label="File summary">
      <DetailTile icon={<IconVideo />} label="Video" value={file.resolution ?? "Unknown"} detail={video || undefined} />
      <DetailTile icon={<IconVolume />} label="Audio" value={file.audioCodec?.toUpperCase() ?? "Unknown"} detail={file.audioChannels ?? undefined} />
      <DetailTile icon={<IconClock />} label="Length" numeric value={formatRuntime(file.duration)} detail={file.bitrate ? `${Math.round(file.bitrate / 1000)} kb/s` : undefined} />
      <DetailTile icon={<IconDatabase />} label="Size" numeric value={formatBytes(file.size)} detail={file.container?.split(",")[0]?.toUpperCase()} />
    </DetailGrid>
  );
}

function StreamGroup({ title, rows }: { title: string; rows: Streams["video"] }) {
  if (!rows.length) return null;
  return (
    <section className="flex min-w-0 flex-col gap-1">
      <h3 className="text-overline text-muted">{title}</h3>
      <ul className="flex min-w-0 flex-col">
        {rows.map((row) => (
          <li key={row.key} className="flex min-w-0 items-baseline justify-between gap-4 border-b border-separator py-2 last:border-b-0">
            <span className="min-w-0 truncate text-body-sm text-foreground">{row.title}</span>
            {row.meta ? <span className="shrink-0 text-label-sm text-muted">{row.meta}</span> : null}
          </li>
        ))}
      </ul>
    </section>
  );
}

function FileStreams({ streams }: { streams: Streams }) {
  if (streams.count === 0) return <p className="text-body-sm text-muted">{streams.loading ? "Loading streams…" : "No streams yet. Re-analyze the file to read them."}</p>;
  return (
    <div className="flex min-w-0 flex-col gap-5">
      <StreamGroup title="Video" rows={streams.video} />
      <StreamGroup title="Audio" rows={streams.audio} />
      <StreamGroup title="Subtitles" rows={streams.subtitles} />
    </div>
  );
}

function FileFacts({ file }: { file: MediaFileFieldsFragment }) {
  return (
    <div className="flex min-w-0 flex-col gap-3">
      <DetailTile icon={<IconFolder />} label="Path" value={
        <span className="flex min-w-0 items-center gap-1">
          <TruncatedText middle className="min-w-0 flex-1 font-mono text-label-sm">{file.path}</TruncatedText>
          <CopyButton value={file.path} label="Copy file path" size="sm" variant="ghost" />
        </span>
      } />
      <DetailGrid label="History">
        <DetailTile icon={<IconCalendar />} label="Added" value={formatDateTime(file.addedAt)} />
        <DetailTile icon={<IconRefresh />} label="Analyzed" value={file.analyzedAt ? formatDateTime(file.analyzedAt) : "Not yet"} />
        <DetailTile icon={<IconSearch />} label="Matched by" value={file.matchType ?? "Not matched"} detail={file.matchConfirmedAt ? formatDateTime(file.matchConfirmedAt) : undefined} />
      </DetailGrid>
    </div>
  );
}

/** Streams and details, a tab each in the file dialog. */
export function fileSections(file: MediaFileFieldsFragment, streams: Streams): DetailSection[] {
  return [
    { key: "streams", label: "Streams", count: streams.count || undefined, content: <FileStreams streams={streams} /> },
    { key: "details", label: "Details", content: <FileFacts file={file} /> },
  ];
}

/** A title's file on its page: the shared identity and summary; the rest is in its dialog. */
export function FileDetails({ file }: { file: FileEntity }) {
  const { actions, dialogs } = useFileActions();
  return (
    <Panel title={<FileIdentity file={file} actions={actions} onOpen={() => openFileDialog(file.id)} />}>
      <FileSummary file={file} />
      {dialogs}
    </Panel>
  );
}
