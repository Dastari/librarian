import { useLazyQuery, useMutation } from "@apollo/client/react";
import { Button } from "glass-ux/button";
import { Dialog } from "glass-ux/dialog";
import { IconLink } from "glass-ux/icons";
import { ListRow } from "glass-ux/list-row";
import { Select } from "glass-ux/select";
import { toast } from "glass-ux/toast";
import { useEffect, useState } from "react";

import { EmptyState, SkeletonBlock } from "@/components/ui";
import { useCompleteConnection } from "@/features/libraries/browser/useCompleteConnection";
import { EntityLibraryListDocument, MatchTorrentFileDocument, TorrentFileCandidatesDocument } from "@/graphql/generated/graphql";
import { errorMessage } from "@/lib/graphql/errors";

export interface MatchableFile {
  infoHash: string;
  fileIndex: number;
  name: string;
  libraryId?: string | null;
}

/**
 * Say what one file of a download is. The matcher suggests titles from the chosen library, and
 * picking one imports the file as that, marked as a manual match that later scans leave alone.
 */
export function MatchFileDialog({ file, onClose, onMatched }: { file: MatchableFile | null; onClose: () => void; onMatched?: () => void }) {
  const libraries = useCompleteConnection(EntityLibraryListDocument, { orderBy: [{ name: "ASC" }] }, (data) => data.libraries, { skip: !file });
  const [chosen, setChosen] = useState<string | null>(null);
  const [linking, setLinking] = useState<string | null>(null);
  const [findCandidates, candidates] = useLazyQuery(TorrentFileCandidatesDocument, { fetchPolicy: "network-only" });
  const [match] = useMutation(MatchTorrentFileDocument);
  const libraryId = chosen ?? file?.libraryId ?? libraries.rows[0]?.id ?? null;

  useEffect(() => {
    if (file && libraryId) void findCandidates({ variables: { infoHash: file.infoHash, fileIndex: file.fileIndex, libraryId } });
  }, [file, libraryId, findCandidates]);
  useEffect(() => { setChosen(null); }, [file?.infoHash, file?.fileIndex]);

  const link = async (targetType: string, targetId: string, label: string) => {
    if (!file || !libraryId) return;
    setLinking(targetId);
    try {
      const { data } = await match({ variables: { input: { infoHash: file.infoHash, fileIndex: file.fileIndex, libraryId, targetType, targetId } } });
      if (data?.matchTorrentFile.success) {
        toast.success(`Imported as ${label}`);
        onMatched?.();
        onClose();
      } else toast.danger(data?.matchTorrentFile.error ?? "The file could not be imported");
    } catch (error) {
      toast.danger(errorMessage(error));
    } finally {
      setLinking(null);
    }
  };

  const list = candidates.data?.torrentFileCandidates ?? [];
  return (
    <Dialog.Root open={Boolean(file)} onOpenChange={(open) => { if (!open) onClose(); }}>
      <Dialog.Popup size="md">
        <Dialog.Header title="What is this file?" icon={<IconLink />} subtitle={<span className="[overflow-wrap:anywhere]">{file?.name}</span>} />
        <Dialog.Body className="flex min-w-0 flex-col gap-3">
          <Select aria-label="Library" value={libraryId} onValueChange={setChosen} placeholder="Choose a library" width="full" options={libraries.rows.map((library) => ({ value: library.id, label: library.name }))} />
          {candidates.loading ? <SkeletonBlock className="h-32 rounded-xl" />
            : candidates.error ? <EmptyState compact icon={IconLink} title="Couldn't look for matches" description={errorMessage(candidates.error)} />
            : list.length === 0 ? <EmptyState compact icon={IconLink} title="Nothing to match it to" description="Add the title to this library first, then try again." />
            : (
              <div className="flex min-w-0 flex-col gap-1">
                {list.map((candidate) => {
                  const label = candidate.targetName ?? candidate.targetId;
                  return <ListRow key={`${candidate.targetType}-${candidate.targetId}`} data-focusable title={label} meta={[candidate.targetType, candidate.reason].filter(Boolean).join(" · ")} trailing={`${Math.round(candidate.score * (candidate.score <= 1 ? 100 : 1))}%`} disabled={Boolean(linking)} onClick={() => void link(candidate.targetType, candidate.targetId, label)} />;
                })}
              </div>
            )}
        </Dialog.Body>
        <Dialog.Footer>
          <Dialog.Close render={<Button variant="ghost">Cancel</Button>} />
        </Dialog.Footer>
      </Dialog.Popup>
    </Dialog.Root>
  );
}
