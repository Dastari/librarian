import { useQuery } from "@apollo/client/react";
import { toast } from "@heroui/react";
import { IconRefresh, IconRulerMeasure } from "@tabler/icons-react";

import { Button, KeyValueList, Panel, StatusChip } from "@/components/ui";
import { useMutation } from "@apollo/client/react";
import { EvaluateMediaFileQualityDocument, MediaFileStreamsDocument, type MediaFileFieldsFragment } from "@/graphql/generated/graphql";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatBytes, formatDateTime, formatRuntime } from "@/lib/format";
import { errorMessage } from "@/lib/graphql/errors";
import { qualityStatus } from "@/lib/status";

/** Technical panel for a linked media file: container, streams, subtitles and quality verdict. */
export function FileDetails({ file }: { file: MediaFileFieldsFragment }) {
  const isAdmin = useIsAdmin();
  const streams = useQuery(MediaFileStreamsDocument, { variables: { mediaFileId: file.id } });
  const [evaluate, { loading }] = useMutation(EvaluateMediaFileQualityDocument);
  const video = streams.data?.videoStreams.edges.map((edge) => edge.node) ?? [];
  const audio = streams.data?.audioStreams.edges.map((edge) => edge.node) ?? [];
  const subtitles = streams.data?.subtitles.edges.map((edge) => edge.node) ?? [];
  const quality = qualityStatus(file.qualityStatus);

  const reevaluate = async () => {
    try {
      const { data } = await evaluate({ variables: { mediaFileId: file.id } });
      const result = data?.evaluateMediaFileQuality;
      if (result?.success) toast.success(`Quality: ${qualityStatus(result.qualityStatus).label}${result.reasons.length ? ` (${result.reasons.join(", ")})` : ""}`);
      else toast.warning(result?.error ?? "Could not evaluate");
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  return (
    <Panel
      title="File"
      actions={
        <>
          <StatusChip status={quality} />
          {isAdmin ? (
            <Button size="sm" variant="ghost" onPress={() => void reevaluate()} isPending={loading}>
              <IconRulerMeasure size={16} /> Re-evaluate
            </Button>
          ) : null}
        </>
      }
    >
      <KeyValueList
        columns={2}
        items={[
          { label: "Path", value: file.path, mono: true },
          { label: "Size", value: formatBytes(file.size) },
          { label: "Container", value: file.container?.toUpperCase() },
          { label: "Duration", value: formatRuntime(file.duration) },
          { label: "Video", value: [file.resolution, file.videoCodec?.toUpperCase(), file.isHdr ? (file.hdrType ?? "HDR") : null].filter(Boolean).join(" · ") || undefined },
          { label: "Audio", value: [file.audioCodec?.toUpperCase(), file.audioChannels].filter(Boolean).join(" · ") || undefined },
          { label: "Bitrate", value: file.bitrate ? `${Math.round(file.bitrate / 1000)} kb/s` : undefined },
          { label: "Match", value: file.matchType ? `${file.matchType}${file.matchConfirmedAt ? ` · ${formatDateTime(file.matchConfirmedAt)}` : ""}` : undefined },
          { label: "Analyzed", value: file.analyzedAt ? formatDateTime(file.analyzedAt) : "Not yet" },
          { label: "Added", value: formatDateTime(file.addedAt) },
        ]}
      />
      {video.length || audio.length || subtitles.length ? (
        <div className="mt-5 grid gap-4 sm:grid-cols-3">
          <StreamList title="Video" items={video.map((stream) => `${stream.codec.toUpperCase()} · ${stream.width}×${stream.height}${stream.frameRate ? ` · ${stream.frameRate.replace(/\/1$/, "")} fps` : ""}${stream.hdrType ? ` · ${stream.hdrType}` : ""}`)} />
          <StreamList title="Audio" items={audio.map((stream) => `${stream.codec.toUpperCase()} · ${stream.channelLayout ?? `${stream.channels}ch`}${stream.language ? ` · ${stream.language}` : ""}${stream.isCommentary ? " · commentary" : ""}`)} />
          <StreamList title="Subtitles" items={subtitles.map((subtitle) => `${subtitle.language ?? subtitle.title ?? subtitle.codec ?? "Track"}${subtitle.isForced ? " · forced" : ""}${subtitle.isHearingImpaired ? " · SDH" : ""}`)} />
        </div>
      ) : null}
      {streams.loading ? <p className="mt-3 inline-flex items-center gap-1 text-label-sm text-muted"><IconRefresh size={12} className="animate-spin" /> Loading streams</p> : null}
    </Panel>
  );
}

function StreamList({ title, items }: { title: string; items: string[] }) {
  return (
    <div>
      <p className="text-overline mb-1.5 text-muted">{title}</p>
      {items.length === 0 ? <p className="text-body-sm text-muted">None</p> : <ul className="flex flex-col gap-1 text-body-sm text-foreground">{items.map((item, index) => <li key={index}>{item}</li>)}</ul>}
    </div>
  );
}
