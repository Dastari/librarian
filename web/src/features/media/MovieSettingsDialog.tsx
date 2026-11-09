import { useMutation } from "@apollo/client/react";
import { useNavigate } from "@tanstack/react-router";
import { EntitySettingsDialog, type EntitySettingsTab } from "glass-ux/entity-settings";
import { formBehaviour, useAppForm } from "glass-ux/form";
import { IconClock, IconDownload, IconFile, IconImage, IconInfo, IconSettings, IconTag, IconTrash } from "glass-ux/icons";
import { toast } from "glass-ux/toast";
import { useEffect, useMemo, useState } from "react";
import { z } from "zod";

import { DetailGrid, DetailTile } from "@/components/entities/EntityDetailDialog";
import { ArtworkSettings } from "@/features/artwork/ArtworkSettings";
import { withArtworkLocks } from "@/features/artwork/slots";
import { Button, ConfirmDialog, EmptyState } from "@/components/ui";
import { EntityMovieDeleteDocument, EntityMovieUpdateDocument, MovieDetailDocument, type MovieDetailQuery } from "@/graphql/generated/graphql";
import { contentStatusMeta, useContentStatuses } from "@/hooks/useContentStatuses";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatDateTime, formatRuntime } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { withoutAction } from "@/lib/actions/withoutAction";

import { DownloadSettingsFields } from "./entity/DownloadSettings";
import { changedFields, yearValue } from "./entity/metadataInput";
import { MetadataTagsField, MetadataTextareaField, MetadataTextField } from "./entity/MetadataFields";
import { FileDetails } from "./FileDetails";
import { movieIdentityProps } from "./MovieIdentity";
import { useMovieActions, type MovieSettingsTab } from "./useMovieActions";

type Movie = NonNullable<MovieDetailQuery["movie"]>;

/** What someone who can't change a movie can still look at. */
const READ_ONLY_TABS: readonly MovieSettingsTab[] = ["files", "info"];

const schema = z.object({
  title: z.string().trim().min(1, "Give the movie a title"),
  year: z.string().trim().regex(/^(\d{4})?$/, "A four-digit year"),
  releaseDate: z.string().trim().regex(/^(\d{4}-\d{2}-\d{2})?$/, "As YYYY-MM-DD"),
});

const valuesOf = (movie: Movie) => ({
  title: movie.title,
  sortTitle: movie.sortTitle ?? "",
  originalTitle: movie.originalTitle ?? "",
  year: movie.year ? String(movie.year) : "",
  releaseDate: movie.releaseDate ?? "",
  certification: movie.certification ?? "",
  tagline: movie.tagline ?? "",
  overview: movie.overview ?? "",
  director: movie.director ?? "",
  genres: movie.genres,
  productionCountries: movie.productionCountries,
  spokenLanguages: movie.spokenLanguages,
  lockedFields: movie.lockedFields ?? [],
  monitored: movie.monitored,
  wanted: movie.wanted,
  qualityProfileId: movie.qualityProfileId ?? "",
});

/** A movie's details and how it's fetched, with its identity and actions in the header. */
export function MovieSettingsDialog({ movieId, open, onOpenChange, tab: chosenTab, onTabChange, onRemoved }: {
  movieId: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  tab: MovieSettingsTab;
  onTabChange: (tab: MovieSettingsTab) => void;
  onRemoved?: (movie: Movie) => void;
}) {
  const { data } = useQuery(MovieDetailDocument, { variables: { id: movieId }, skip: !open });
  const movie = data?.movie;
  if (!movie || movie.id !== movieId) return null;
  return <LoadedMovieSettings movie={movie} open={open} onOpenChange={onOpenChange} chosenTab={chosenTab} onTabChange={onTabChange} onRemoved={onRemoved} />;
}

function LoadedMovieSettings({ movie, open, onOpenChange, chosenTab, onTabChange, onRemoved }: {
  movie: Movie;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  chosenTab: MovieSettingsTab;
  onTabChange: (tab: MovieSettingsTab) => void;
  onRemoved?: (movie: Movie) => void;
}) {
  const navigate = useNavigate();
  const isAdmin = useIsAdmin();
  const tab = isAdmin || READ_ONLY_TABS.includes(chosenTab) ? chosenTab : READ_ONLY_TABS[0]!;
  const statuses = useContentStatuses("MOVIE", [movie.id]);
  const [update] = useMutation(EntityMovieUpdateDocument);
  const [remove, { loading: removing }] = useMutation(EntityMovieDeleteDocument);
  const [confirmRemove, setConfirmRemove] = useState(false);

  const form = useAppForm({
    ...formBehaviour,
    defaultValues: valuesOf(movie),
    validators: { onDynamic: schema },
    onSubmit: async ({ value, formApi }) => {
      const { year, ...input } = changedFields(valuesOf(movie), value);
      const { data } = await update({
        variables: {
          id: movie.id,
          input: {
            ...input,
            ...(year !== undefined ? { year: yearValue(value.year) } : {}),
            ...(input.qualityProfileId !== undefined ? { qualityProfileId: value.qualityProfileId || null } : {}),
            // A picture chosen while the form was open keeps its lock.
            ...(input.lockedFields !== undefined ? { lockedFields: withArtworkLocks(value.lockedFields, movie.lockedFields) } : {}),
          },
        },
      });
      assertSuccess(data?.updateMovie, "Could not save the movie");
      formApi.reset(value);
    },
  });
  // Another movie, or this one reopened: start from what's saved.
  useEffect(() => { if (open) form.reset(valuesOf(movie)); }, [open, movie.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const { actions, dialogs } = useMovieActions<Movie>({ settings: (_, next) => onTabChange(next ?? "general"), open: false });
  const headerActions = useMemo(() => withoutAction(actions, "settings", "remove"), [actions]);
  const onRemove = async () => {
    try {
      assertSuccess((await remove({ variables: { id: movie.id } })).data?.deleteMovie, "Could not remove the movie");
      toast.success("Movie removed from the library");
      onOpenChange(false);
      if (onRemoved) onRemoved(movie);
      else await navigate({ to: "/libraries/$libraryId/movies", params: { libraryId: movie.libraryId } });
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const allTabs: EntitySettingsTab[] = [
    {
      value: "general", label: "General", icon: <IconSettings />, fields: ["title", "sortTitle", "originalTitle", "year", "releaseDate", "certification", "tagline", "overview", "director"],
      content: (
        <div className="flex flex-col gap-3">
          <form.AppField name="title">{() => <MetadataTextField label="Title" required />}</form.AppField>
          <form.AppField name="sortTitle">{() => <MetadataTextField label="Sort title" hint="Leave blank to sort by the title without a leading The, A or An." />}</form.AppField>
          <form.AppField name="originalTitle">{() => <MetadataTextField label="Original title" />}</form.AppField>
          <div className="grid gap-3 sm:grid-cols-3">
            <form.AppField name="year">{() => <MetadataTextField label="Year" inputMode="numeric" />}</form.AppField>
            <form.AppField name="releaseDate">{() => <MetadataTextField label="Released" placeholder="YYYY-MM-DD" />}</form.AppField>
            <form.AppField name="certification">{() => <MetadataTextField label="Content rating" />}</form.AppField>
          </div>
          <form.AppField name="tagline">{() => <MetadataTextField label="Tagline" />}</form.AppField>
          <form.AppField name="overview">{() => <MetadataTextareaField label="Summary" minRows={5} />}</form.AppField>
          <form.AppField name="director">{() => <MetadataTextField label="Director" />}</form.AppField>
        </div>
      ),
    },
    {
      value: "tags", label: "Tags", icon: <IconTag />, fields: ["genres", "productionCountries", "spokenLanguages"],
      content: (
        <div className="flex flex-col gap-3">
          <form.AppField name="genres">{() => <MetadataTagsField label="Genres" placeholder="Add a genre" />}</form.AppField>
          <form.AppField name="productionCountries">{() => <MetadataTagsField label="Countries" placeholder="Add a country" />}</form.AppField>
          <form.AppField name="spokenLanguages">{() => <MetadataTagsField label="Languages" placeholder="Add a language" />}</form.AppField>
        </div>
      ),
    },
    {
      value: "artwork", label: "Artwork", icon: <IconImage />,
      content: <ArtworkSettings target={{ entityType: "MOVIE", entityId: movie.id, title: movie.title }} active={open && tab === "artwork"} />,
    },
    {
      value: "downloads", label: "Downloads", icon: <IconDownload />, fields: ["monitored", "wanted", "qualityProfileId"],
      content: (
        <DownloadSettingsFields libraryId={movie.libraryId} scope={{ movieId: movie.id }} active={open && tab === "downloads"}
          mode={(
            <>
              <form.AppField name="monitored">{(field) => <field.SwitchRow label="Monitored" description="Keep watching for a better release once the movie is in the library." />}</form.AppField>
              <form.AppField name="wanted" listeners={{ onChange: ({ value }) => { if (value) form.setFieldValue("monitored", true); } }}>{(field) => <field.SwitchRow label="Wanted" description="Search sources for this movie until a file arrives." />}</form.AppField>
            </>
          )}
          profile={(options) => <form.AppField name="qualityProfileId">{(field) => <field.SelectField label="Quality profile" options={options} />}</form.AppField>}
        />
      ),
    },
    {
      value: "files", label: "Files", icon: <IconFile />, count: movie.mediaFile ? 1 : 0,
      content: tab === "files" ? (
        movie.mediaFile ? <FileDetails file={movie.mediaFile} /> : <EmptyState icon={IconFile} title="No file yet" description={movie.wanted ? "Librarian is looking for a release." : "Mark it as wanted to search automatically."} />
      ) : null,
    },
    {
      value: "info", label: "Info", icon: <IconInfo />,
      content: (
        <DetailGrid label="About this movie">
          <DetailTile icon={<IconClock />} label="Length" numeric value={movie.runtime ? formatRuntime(movie.runtime, "minutes") : "Unknown"} />
          <DetailTile icon={<IconInfo />} label="TMDB rating" numeric value={movie.tmdbRating ? `${Number(movie.tmdbRating).toFixed(1)}` : "None"} detail={movie.tmdbVoteCount ? `${movie.tmdbVoteCount} votes` : undefined} />
          <DetailTile icon={<IconInfo />} label="Status" value={movie.tmdbStatus ?? "Unknown"} />
          <DetailTile icon={<IconInfo />} label="Collection" value={movie.collectionName ?? "None"} />
          <DetailTile icon={<IconInfo />} label="TMDB" value={movie.tmdbId ? <a className="nav-focus rounded text-brand hover:underline" href={`https://www.themoviedb.org/movie/${movie.tmdbId}`} target="_blank" rel="noreferrer">{movie.tmdbId}</a> : "None"} />
          <DetailTile icon={<IconInfo />} label="IMDb" value={movie.imdbId ? <a className="nav-focus rounded text-brand hover:underline" href={`https://www.imdb.com/title/${movie.imdbId}`} target="_blank" rel="noreferrer">{movie.imdbId}</a> : "None"} />
          <DetailTile icon={<IconClock />} label="Added" value={formatDateTime(movie.createdAt)} />
          <DetailTile icon={<IconClock />} label="Updated" value={formatDateTime(movie.updatedAt)} />
        </DetailGrid>
      ),
    },
    {
      value: "remove", label: "Remove", icon: <IconTrash />,
      content: (
        <div className="flex flex-col items-start gap-3">
          <p className="text-body-sm text-muted">The movie leaves the catalogue. Its file stays on disk and shows up as unmatched on the next scan.</p>
          <Button variant="danger-soft" onPress={() => setConfirmRemove(true)}><IconTrash size="md" /> Remove from library</Button>
        </div>
      ),
    },
  ];
  const tabs = isAdmin ? allTabs : allTabs.filter((item) => READ_ONLY_TABS.includes(item.value as MovieSettingsTab));

  return (
    <>
      <EntitySettingsDialog
        open={open}
        onOpenChange={onOpenChange}
        form={form}
        size="xl"
        identity={movieIdentityProps(movie, { status: contentStatusMeta(statuses, movie.id), actions: headerActions })}
        tabs={tabs}
        tab={tab}
        onTabChange={(next) => onTabChange(next as MovieSettingsTab)}
        saveDisabledReason={isAdmin ? undefined : "Only an admin can change a movie"}
      />
      <ConfirmDialog isOpen={confirmRemove} onOpenChange={setConfirmRemove} title={`Remove ${movie.title}?`} description="The movie leaves the catalogue. Its file stays on disk and shows up as unmatched on the next scan." confirmLabel="Remove" destructive isPending={removing} onConfirm={onRemove} />
      {dialogs}
    </>
  );
}
