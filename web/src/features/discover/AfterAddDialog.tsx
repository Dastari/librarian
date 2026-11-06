import { useMutation } from "@apollo/client/react";
import { Button } from "glass-ux/button";
import { Dialog } from "glass-ux/dialog";
import { IconCheck, IconSearch, IconSparkle } from "glass-ux/icons";
import { ListRow } from "glass-ux/list-row";
import { toast } from "glass-ux/toast";
import { useState } from "react";

import { Artwork } from "@/components/ui";
import { ReleaseSearchDialog, type ReleaseTarget } from "@/features/downloads/ReleaseSearchDialog";
import {
  EntityAlbumUpdateDocument, EntityAudiobookUpdateDocument, EntityChapterUpdateManyDocument, EntityMovieUpdateDocument, EntityShowUpdateDocument,
  SearchMissingDocument,
} from "@/graphql/generated/graphql";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

export type AddedKind = "movie" | "show" | "audiobook" | "album";

/** A title that has just gone into a library, and what a release search needs to find it. */
export interface AddedTitle {
  kind: AddedKind;
  id: string;
  title: string;
  /** Its poster or cover, where the question is asked. */
  image?: string | null;
  libraryId: string;
  libraryName?: string;
  year?: number | null;
  imdbId?: string | null;
  author?: string | null;
  artist?: string | null;
}

const AUTOMATIC: Record<AddedKind, string> = {
  movie: "Grabs the best match as soon as one turns up.",
  show: "Grabs what's missing, then new episodes as they air.",
  audiobook: "Grabs the best match as soon as one turns up.",
  album: "Grabs the best match as soon as one turns up.",
};

function scope(added: AddedTitle): ReleaseTarget {
  switch (added.kind) {
    case "movie": return { movieId: added.id };
    case "show": return { showId: added.id };
    case "album": return { albumId: added.id };
    default: return { audiobookId: added.id };
  }
}

/**
 * The one question left once a title is in a library: how to get a copy. Nothing downloads until
 * someone answers, and `onDone` runs whichever way they answer.
 */
export function AfterAddDialog({ added, onDone }: { added: AddedTitle | null; onDone: () => void }) {
  const [finding, setFinding] = useState(false);
  const [automating, setAutomating] = useState(false);
  const [updateMovie] = useMutation(EntityMovieUpdateDocument);
  const [updateShow] = useMutation(EntityShowUpdateDocument);
  const [updateBook] = useMutation(EntityAudiobookUpdateDocument);
  const [updateAlbum] = useMutation(EntityAlbumUpdateDocument);
  const [updateChapters] = useMutation(EntityChapterUpdateManyDocument);
  const [searchMissing] = useMutation(SearchMissingDocument);

  const automate = async () => {
    if (!added) return;
    setAutomating(true);
    try {
      const { id } = added;
      if (added.kind === "movie") assertSuccess((await updateMovie({ variables: { id, input: { monitored: true, wanted: true } } })).data?.updateMovie, "Could not turn on downloads");
      else if (added.kind === "show") assertSuccess((await updateShow({ variables: { id, input: { autoDownload: true, autoDownloadMode: "WANTED" } } })).data?.updateShow, "Could not turn on downloads");
      else if (added.kind === "album") assertSuccess((await updateAlbum({ variables: { id, input: { autoDownload: true, autoDownloadMode: "WANTED" } } })).data?.updateAlbum, "Could not turn on downloads");
      else {
        assertSuccess((await updateBook({ variables: { id, input: { autoDownload: true, autoDownloadMode: "WANTED" } } })).data?.updateAudiobook, "Could not turn on downloads");
        // The grabber looks for chapters that are wanted. A book without any is searched as a whole.
        assertSuccess((await updateChapters({ variables: { where: { audiobookId: { eq: id } }, input: { wanted: true } } })).data?.updateChapters, "Could not turn on downloads");
      }
      const { data } = await searchMissing({ variables: { input: scope(added) } });
      const result = data?.searchMissing;
      if (result?.success) toast.success(result.queued ? `Downloading ${added.title}` : `${added.title} will download as soon as a match turns up`);
      else toast.warning(result?.error ?? "Automatic downloads are on, but the first search did not run");
      onDone();
    } catch (error) {
      toast.danger(errorMessage(error));
    } finally {
      setAutomating(false);
    }
  };

  return (
    <>
      <Dialog.Root open={Boolean(added) && !finding} onOpenChange={(open) => { if (!open && added) onDone(); }}>
        <Dialog.Popup size="md">
          <Dialog.Header title={`${added?.title} is in your library`} icon={added?.image ? undefined : <IconCheck />} media={added?.image ? <Artwork src={added.image} alt="" aspect={added.kind === "album" ? "square" : "poster"} displayWidth={96} className="w-12 shrink-0 rounded-md" /> : undefined} subtitle={`Added to ${added?.libraryName ?? "your library"}. Nothing has downloaded yet.`} />
          <Dialog.Body className="flex flex-col gap-3">
            <p className="text-body-sm text-muted">Do you want to download it now?</p>
            <div className="flex flex-col gap-1">
              <ListRow icon={<IconSearch />} title="Choose a release" meta="See what's available and pick one yourself." disabled={automating} onClick={() => setFinding(true)} />
              <ListRow icon={<IconSparkle />} title="Download automatically" meta={added ? AUTOMATIC[added.kind] : undefined} disabled={automating} onClick={() => void automate()} />
            </div>
          </Dialog.Body>
          <Dialog.Footer>
            <Button variant="ghost" disabledReason={automating ? "Turning on downloads" : undefined} onClick={onDone}>Not now</Button>
          </Dialog.Footer>
        </Dialog.Popup>
      </Dialog.Root>

      {added ? (
        <ReleaseSearchDialog
          isOpen={finding}
          onOpenChange={(open) => { if (!open) { setFinding(false); onDone(); } }}
          query={added.title}
          year={added.year}
          imdbId={added.imdbId}
          author={added.author}
          artist={added.artist}
          album={added.kind === "album" ? added.title : undefined}
          libraryId={added.libraryId}
          target={scope(added)}
        />
      ) : null}
    </>
  );
}
