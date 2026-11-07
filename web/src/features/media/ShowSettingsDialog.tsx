import { useMutation } from "@apollo/client/react";
import { useNavigate } from "@tanstack/react-router";
import { EntitySettingsDialog, type EntitySettingsTab } from "glass-ux/entity-settings";
import { formBehaviour, useAppForm } from "glass-ux/form";
import { IconClock, IconDownload, IconFile, IconInfo, IconSettings, IconTrash } from "glass-ux/icons";
import { toast } from "glass-ux/toast";
import { useEffect, useMemo, useState } from "react";
import { z } from "zod";

import { DetailGrid, DetailTile } from "@/components/entities/EntityDetailDialog";
import { Button, ConfirmDialog } from "@/components/ui";
import { FilesBrowser } from "@/features/libraries/browser/FilesBrowser";
import { EntityShowDeleteDocument, EntityShowUpdateDocument, ShowDetailDocument, type ShowDetailQuery } from "@/graphql/generated/graphql";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatDateTime, formatRuntime } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { withoutAction } from "@/lib/actions/withoutAction";

import { AutoDownloadField, DownloadSettingsFields } from "./entity/DownloadSettings";
import { changedFields, yearValue } from "./entity/metadataInput";
import { MetadataTagsField, MetadataTextareaField, MetadataTextField } from "./entity/MetadataFields";
import { useTitleFiles } from "./entity/useTitleFiles";
import { showIdentityProps } from "./ShowIdentity";
import { useShowActions, type ShowSettingsTab } from "./useShowActions";

type Show = NonNullable<ShowDetailQuery["show"]>;

const READ_ONLY_TABS: readonly ShowSettingsTab[] = ["files", "info"];

const schema = z.object({
  name: z.string().trim().min(1, "Give the show a name"),
  year: z.string().trim().regex(/^(\d{4})?$/, "A four-digit year"),
});

const valuesOf = (show: Show) => ({
  name: show.name,
  sortName: show.sortName ?? "",
  year: show.year ? String(show.year) : "",
  contentRating: show.contentRating ?? "",
  network: show.network ?? "",
  overview: show.overview ?? "",
  genres: show.genres,
  lockedFields: show.lockedFields ?? [],
  autoDownloadMode: show.autoDownloadMode,
  qualityProfileId: show.qualityProfileId ?? "",
});

/** A show's details and how it's fetched, with its identity and actions in the header. */
export function ShowSettingsDialog({ showId, open, onOpenChange, tab, onTabChange, onRemoved }: {
  showId: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  tab: ShowSettingsTab;
  onTabChange: (tab: ShowSettingsTab) => void;
  onRemoved?: () => void;
}) {
  const { data } = useQuery(ShowDetailDocument, { variables: { id: showId }, skip: !open });
  const show = data?.show;
  if (!show || show.id !== showId) return null;
  return <LoadedShowSettings show={show} open={open} onOpenChange={onOpenChange} chosenTab={tab} onTabChange={onTabChange} onRemoved={onRemoved} />;
}

function LoadedShowSettings({ show, open, onOpenChange, chosenTab, onTabChange, onRemoved }: {
  show: Show;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  chosenTab: ShowSettingsTab;
  onTabChange: (tab: ShowSettingsTab) => void;
  onRemoved?: () => void;
}) {
  const navigate = useNavigate();
  const isAdmin = useIsAdmin();
  const tab = isAdmin || READ_ONLY_TABS.includes(chosenTab) ? chosenTab : READ_ONLY_TABS[0]!;
  const files = useTitleFiles("show", show.id, !open);
  const [update] = useMutation(EntityShowUpdateDocument);
  const [remove, { loading: removing }] = useMutation(EntityShowDeleteDocument);
  const [confirmRemove, setConfirmRemove] = useState(false);

  const form = useAppForm({
    ...formBehaviour,
    defaultValues: valuesOf(show),
    validators: { onDynamic: schema },
    onSubmit: async ({ value, formApi }) => {
      const { year, autoDownloadMode, ...input } = changedFields(valuesOf(show), value);
      const { data } = await update({
        variables: {
          id: show.id,
          input: {
            ...input,
            ...(year !== undefined ? { year: yearValue(value.year) } : {}),
            ...(autoDownloadMode ? { autoDownloadMode, autoDownload: autoDownloadMode !== "NONE" } : {}),
          },
        },
      });
      assertSuccess(data?.updateShow, "Could not save the show");
      formApi.reset(value);
    },
  });
  useEffect(() => { if (open) form.reset(valuesOf(show)); }, [open, show.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const { actions, dialogs } = useShowActions<Show>({ settings: (_, next) => onTabChange(next ?? "general"), open: false });
  const headerActions = useMemo(() => withoutAction(actions, "settings", "remove"), [actions]);
  const onRemove = async () => {
    try {
      assertSuccess((await remove({ variables: { id: show.id } })).data?.deleteShow, "Could not remove the show");
      toast.success("Show removed from the library");
      onOpenChange(false);
      if (onRemoved) onRemoved();
      else await navigate({ to: "/libraries/$libraryId/shows", params: { libraryId: show.libraryId } });
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const allTabs: EntitySettingsTab[] = [
    {
      value: "general", label: "General", icon: <IconSettings />, fields: ["name", "sortName", "year", "contentRating", "network", "overview", "genres"],
      content: (
        <div className="flex flex-col gap-3">
          <form.AppField name="name">{() => <MetadataTextField label="Name" required />}</form.AppField>
          <form.AppField name="sortName">{() => <MetadataTextField label="Sort name" hint="Leave blank to sort by the name without a leading The, A or An." />}</form.AppField>
          <div className="grid gap-3 sm:grid-cols-3">
            <form.AppField name="year">{() => <MetadataTextField label="Year" inputMode="numeric" />}</form.AppField>
            <form.AppField name="contentRating">{() => <MetadataTextField label="Content rating" />}</form.AppField>
            <form.AppField name="network">{() => <MetadataTextField label="Network" />}</form.AppField>
          </div>
          <form.AppField name="overview">{() => <MetadataTextareaField label="Summary" minRows={5} />}</form.AppField>
          <form.AppField name="genres">{() => <MetadataTagsField label="Genres" placeholder="Add a genre" />}</form.AppField>
        </div>
      ),
    },
    {
      value: "downloads", label: "Downloads", icon: <IconDownload />, fields: ["autoDownloadMode", "qualityProfileId"],
      content: (
        <DownloadSettingsFields libraryId={show.libraryId} scope={{ showId: show.id }} active={open && tab === "downloads"}
          mode={<form.AppField name="autoDownloadMode">{() => <AutoDownloadField />}</form.AppField>}
          profile={(options) => <form.AppField name="qualityProfileId">{(field) => <field.SelectField label="Quality profile" options={options} />}</form.AppField>}
        />
      ),
    },
    {
      value: "files", label: "Files", icon: <IconFile />, count: files.count,
      content: tab === "files" && files.scope ? <div className="entity-tab-table flex min-h-0 flex-1 flex-col overflow-hidden"><FilesBrowser libraryId={show.libraryId} scope={files.scope} filterPrefix="file" /></div> : null,
    },
    {
      value: "info", label: "Info", icon: <IconInfo />,
      content: (
        <DetailGrid label="About this show">
          <DetailTile icon={<IconClock />} label="Episode length" numeric value={show.runtime ? formatRuntime(show.runtime, "minutes") : "Unknown"} />
          <DetailTile icon={<IconInfo />} label="Status" value={show.status ?? "Unknown"} />
          <DetailTile icon={<IconInfo />} label="TVmaze" value={show.tvmazeId ? <a className="nav-focus rounded text-brand hover:underline" href={`https://www.tvmaze.com/shows/${show.tvmazeId}`} target="_blank" rel="noreferrer">{show.tvmazeId}</a> : "None"} />
          <DetailTile icon={<IconInfo />} label="IMDb" value={show.imdbId ? <a className="nav-focus rounded text-brand hover:underline" href={`https://www.imdb.com/title/${show.imdbId}`} target="_blank" rel="noreferrer">{show.imdbId}</a> : "None"} />
          <DetailTile icon={<IconClock />} label="Added" value={formatDateTime(show.createdAt)} />
          <DetailTile icon={<IconClock />} label="Updated" value={formatDateTime(show.updatedAt)} />
        </DetailGrid>
      ),
    },
    {
      value: "remove", label: "Remove", icon: <IconTrash />,
      content: (
        <div className="flex flex-col items-start gap-3">
          <p className="text-body-sm text-muted">The show and its episodes leave the catalogue. Files stay on disk.</p>
          <Button variant="danger-soft" onPress={() => setConfirmRemove(true)}><IconTrash size="md" /> Remove from library</Button>
        </div>
      ),
    },
  ];
  const tabs = isAdmin ? allTabs : allTabs.filter((item) => READ_ONLY_TABS.includes(item.value as ShowSettingsTab));

  return (
    <>
      <EntitySettingsDialog
        open={open}
        onOpenChange={onOpenChange}
        form={form}
        size="xl"
        identity={showIdentityProps(show, { actions: headerActions })}
        tabs={tabs}
        tab={tab}
        onTabChange={(next) => onTabChange(next as ShowSettingsTab)}
        saveDisabledReason={isAdmin ? undefined : "Only an admin can change a show"}
      />
      <ConfirmDialog isOpen={confirmRemove} onOpenChange={setConfirmRemove} title={`Remove ${show.name}?`} description="The show and its episodes leave the catalogue. Files stay on disk." confirmLabel="Remove" destructive isPending={removing} onConfirm={onRemove} />
      {dialogs}
    </>
  );
}
