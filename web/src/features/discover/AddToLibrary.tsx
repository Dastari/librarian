import { useMutation } from "@apollo/client/react";
import { useNavigate } from "@tanstack/react-router";
import { Button } from "glass-ux/button";
import { Dialog } from "glass-ux/dialog";
import { IconPlus } from "glass-ux/icons";
import { Select } from "glass-ux/select";
import { toast } from "glass-ux/toast";
import { useMemo, useState } from "react";

import { AddAudiobookDocument, AddMovieDocument, AddTvShowDocument, LibrariesOverviewDocument } from "@/graphql/generated/graphql";
import { useIsAdmin } from "@/lib/auth/useSession";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { libraryType, type LibraryType } from "@/lib/library-types";

import { AfterAddDialog, type AddedTitle } from "./AfterAddDialog";

export type AddTarget =
  | { kind: "movie"; tmdbId: number; title: string; image?: string | null; year?: number | null; imdbId?: string | null }
  | { kind: "show"; tvmazeId: number; title: string; image?: string | null; year?: number | null; imdbId?: string | null }
  | { kind: "audiobook"; providerId: string; title: string; image?: string | null; author?: string | null };

const LIBRARY_KIND: Record<AddTarget["kind"], LibraryType> = { movie: "movies", show: "tv", audiobook: "audiobooks" };
/**
 * "Add to library": the title goes in straight away, so it has a real page whatever happens next,
 * and then the one remaining question is how to get a copy. With several libraries of the kind,
 * which one is asked first. Administrators only, as with every other way of adding.
 */
export function AddToLibrary({ target }: { target: AddTarget }) {
  const isAdmin = useIsAdmin();
  const navigate = useNavigate();
  const libraries = useQuery(LibrariesOverviewDocument, { skip: !isAdmin });
  const options = useMemo(
    () => (libraries.data?.libraries.edges ?? []).map(({ node }) => node).filter((library) => libraryType(library.libraryType).type === LIBRARY_KIND[target.kind]),
    [libraries.data, target.kind],
  );
  const [choosing, setChoosing] = useState(false);
  const [chosen, setChosen] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);
  const [added, setAdded] = useState<AddedTitle | null>(null);

  const [addMovie] = useMutation(AddMovieDocument);
  const [addShow] = useMutation(AddTvShowDocument);
  const [addBook] = useMutation(AddAudiobookDocument);

  if (!isAdmin) return null;

  const add = async (libraryId: string) => {
    setAdding(true);
    try {
      let id: string | undefined;
      if (target.kind === "movie") {
        const { data } = await addMovie({ variables: { libraryId, input: { tmdbId: target.tmdbId, monitored: false } } });
        id = assertSuccess(data?.addMovie, "Could not add this movie").movie?.id;
      } else if (target.kind === "show") {
        const { data } = await addShow({ variables: { libraryId, input: { tvmazeId: target.tvmazeId, autoDownloadMode: "NONE" } } });
        id = assertSuccess(data?.addTvShow, "Could not add this show").show?.id;
      } else {
        const { data } = await addBook({ variables: { input: { libraryId, providerId: target.providerId, autoDownloadMode: "NONE" } } });
        id = assertSuccess(data?.addAudiobook, "Could not add this audiobook").audiobook?.id;
      }
      if (!id) throw new Error("The server did not return the new title.");
      setChoosing(false);
      setAdded({
        id, libraryId, kind: target.kind, title: target.title, image: target.image,
        libraryName: options.find((library) => library.id === libraryId)?.name,
        year: target.kind === "audiobook" ? undefined : target.year,
        imdbId: target.kind === "audiobook" ? undefined : target.imdbId,
        author: target.kind === "audiobook" ? target.author : undefined,
      });
    } catch (error) {
      toast.danger(errorMessage(error));
    } finally {
      setAdding(false);
    }
  };

  const openPage = (id: string) => {
    if (target.kind === "movie") void navigate({ to: "/movies/$movieId", params: { movieId: id }, replace: true });
    else if (target.kind === "show") void navigate({ to: "/shows/$showId", params: { showId: id }, replace: true });
    else void navigate({ to: "/audiobooks/$audiobookId", params: { audiobookId: id }, replace: true });
  };

  const reason = libraries.loading ? "Loading your libraries" : options.length === 0 ? `Create a ${libraryType(LIBRARY_KIND[target.kind]).label.toLowerCase()} library in Settings first` : undefined;
  const selected = chosen ?? options[0]?.id ?? null;

  return (
    <>
      <Button
        variant="primary"
        size="lg"
        icon={<IconPlus />}
        data-tv-autofocus
        disabledReason={reason}
        status={adding ? "pending" : undefined}
        onClick={() => {
          if (options.length > 1) setChoosing(true);
          else if (options[0]) void add(options[0].id);
        }}
      >
        Add to library
      </Button>

      <Dialog.Root open={choosing} onOpenChange={setChoosing}>
        <Dialog.Popup size="sm">
          <Dialog.Header title={`Add ${target.title}`} icon={<IconPlus />} subtitle="Which library should it go in?" />
          <Dialog.Body>
            <Select aria-label="Library" value={selected} onValueChange={setChosen} options={options.map((library) => ({ value: library.id, label: library.name }))} />
          </Dialog.Body>
          <Dialog.Footer>
            <Dialog.Close render={<Button variant="ghost">Cancel</Button>} />
            <Button variant="primary" disabledReason={selected ? undefined : "Choose a library"} status={adding ? "pending" : undefined} onClick={() => { if (selected) void add(selected); }}>Add to library</Button>
          </Dialog.Footer>
        </Dialog.Popup>
      </Dialog.Root>

      <AfterAddDialog added={added} onDone={() => { if (added) openPage(added.id); }} />
    </>
  );
}
