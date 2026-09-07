import { useMutation } from "@apollo/client/react";
import { toast } from "@heroui/react";
import { IconLink } from "@tabler/icons-react";
import { useEffect, useState } from "react";

import { Button, Dialog, EmptyState, Spinner, StatusChip } from "@/components/ui";
import { MatchMediaFileDocument, type MatchMediaFileMutation } from "@/graphql/generated/graphql";
import { errorMessage } from "@/lib/graphql/errors";

type Candidate = MatchMediaFileMutation["matchMediaFile"]["candidates"][number];

interface ManualMatchDialogProps {
  file: { id: string; path: string } | null;
  libraryId: string;
  onClose: () => void;
  onMatched?: () => void;
}

/**
 * Asks the matcher for candidates without auto-linking, then links the one the user picks.
 * Manual matches are never overwritten by later scans.
 */
export function ManualMatchDialog({ file, libraryId, onClose, onMatched }: ManualMatchDialogProps) {
  const [matchFile, { loading }] = useMutation(MatchMediaFileDocument);
  const [candidates, setCandidates] = useState<Candidate[] | null>(null);
  const [reason, setReason] = useState<string | null>(null);
  const [linking, setLinking] = useState<string | null>(null);

  useEffect(() => {
    if (!file) return;
    setCandidates(null);
    setReason(null);
    void matchFile({ variables: { input: { mediaFileId: file.id, libraryId, autoMatch: false, candidateLimit: 12, wantedPolicy: "ALL", allowProviderFallback: false } } })
      .then(({ data }) => {
        setCandidates(data?.matchMediaFile.candidates ?? []);
        setReason(data?.matchMediaFile.reason ?? null);
      })
      .catch((error) => setReason(errorMessage(error)));
  }, [file, libraryId, matchFile]);

  const link = async (candidate: Candidate) => {
    if (!file) return;
    setLinking(candidate.targetId);
    const target: Record<string, string> = {};
    const key = `${candidate.targetType.toLowerCase()}Id`;
    if (["movieId", "episodeId", "trackId", "chapterId"].includes(key)) target[key] = candidate.targetId;
    try {
      const { data } = await matchFile({ variables: { input: { mediaFileId: file.id, libraryId, autoMatch: true, force: true, ...target } } });
      if (data?.matchMediaFile.success) {
        toast.success(`Linked to ${candidate.targetName ?? candidate.targetType}`);
        onMatched?.();
        onClose();
      } else toast.warning(data?.matchMediaFile.reason ?? "Could not link");
    } catch (error) {
      toast.danger(errorMessage(error));
    } finally {
      setLinking(null);
    }
  };

  return (
    <Dialog isOpen={Boolean(file)} onOpenChange={(open) => !open && onClose()} title="Match file" description={file ? file.path.split(/[\\/]/).pop() : undefined} size="md">
      {loading && !candidates ? (
        <div className="grid h-40 place-items-center">
          <Spinner size={28} />
        </div>
      ) : !candidates || candidates.length === 0 ? (
        <EmptyState compact icon={IconLink} title="No candidates" description={reason ?? "Add the movie, show, album or book to the library first, then match again."} />
      ) : (
        <ul className="flex flex-col gap-1">
          {candidates.map((candidate) => (
            <li key={`${candidate.targetType}-${candidate.targetId}`} className="flex items-center gap-3 rounded-card p-2 hover:bg-surface-hover">
              <div className="min-w-0 flex-1">
                <p className="truncate text-title-sm text-foreground">{candidate.targetName ?? candidate.targetId}</p>
                <p className="truncate text-label-sm text-muted">{[candidate.targetType, candidate.reason].filter(Boolean).join(" · ")}</p>
              </div>
              {candidate.wanted ? <StatusChip status={{ label: "Wanted", tone: "warning", dot: "bg-warning" }} /> : null}
              <span className="text-numeric text-label text-muted">{Math.round(candidate.score)}%</span>
              <Button size="sm" variant="secondary" isPending={linking === candidate.targetId} onPress={() => void link(candidate)}>
                Link
              </Button>
            </li>
          ))}
        </ul>
      )}
    </Dialog>
  );
}
