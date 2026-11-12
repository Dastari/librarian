import { useMutation } from "@apollo/client/react";
import { useNavigate } from "@tanstack/react-router";
import { EntitySettingsDialog, type EntitySettingsTab } from "glass-ux/entity-settings";
import { formBehaviour, useAppForm } from "glass-ux/form";
import { IconClock, IconDownload, IconFile, IconImage, IconInfo, IconSettings, IconTrash } from "glass-ux/icons";
import { toast } from "glass-ux/toast";
import { useEffect, useMemo, useState } from "react";
import { z } from "zod";

import { DetailGrid, DetailTile } from "@/components/entities/EntityDetailDialog";
import { ArtworkSettings } from "@/features/artwork/ArtworkSettings";
import { Button, ConfirmDialog } from "@/components/ui";
import { FilesBrowser } from "@/features/libraries/browser/FilesBrowser";
import { AlbumDetailDocument, EntityAlbumDeleteDocument, EntityAlbumUpdateDocument, type AlbumDetailQuery } from "@/graphql/generated/graphql";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatBytes, formatDateTime, formatRuntime } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { withoutAction } from "@/lib/actions/withoutAction";

import { albumIdentityProps } from "./AlbumIdentity";
import { AutoDownloadField, DownloadSettingsFields } from "./entity/DownloadSettings";
import { changedFields, yearValue } from "./entity/metadataInput";
import { MetadataTagsField, MetadataTextField } from "./entity/MetadataFields";
import { useTitleFiles } from "./entity/useTitleFiles";
import { useAlbumActions, type AlbumSettingsTab } from "./useAlbumActions";

type Album = NonNullable<AlbumDetailQuery["album"]>;

const READ_ONLY_TABS: readonly AlbumSettingsTab[] = ["files", "info"];

const schema = z.object({
  name: z.string().trim().min(1, "Give the album a name"),
  year: z.string().trim().regex(/^(\d{4})?$/, "A four-digit year"),
  releaseDate: z.string().trim().regex(/^(\d{4}(-\d{2}(-\d{2})?)?)?$/, "As YYYY or YYYY-MM-DD"),
});

// No lockedFields here: nothing refreshes an album's details yet. Its cover's lock is on the Artwork tab.
const valuesOf = (album: Album) => ({
  name: album.name,
  sortName: album.sortName ?? "",
  year: album.year ? String(album.year) : "",
  releaseDate: album.releaseDate ?? "",
  albumType: album.albumType ?? "",
  label: album.label ?? "",
  country: album.country ?? "",
  genres: album.genres,
  autoDownloadMode: album.autoDownloadMode,
  qualityProfileId: album.qualityProfileId ?? "",
});

/** An album's details and how it's fetched, with its identity and actions in the header. */
export function AlbumSettingsDialog({ albumId, open, onOpenChange, tab, onTabChange, onRemoved }: {
  albumId: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  tab: AlbumSettingsTab;
  onTabChange: (tab: AlbumSettingsTab) => void;
  onRemoved?: () => void;
}) {
  const { data } = useQuery(AlbumDetailDocument, { variables: { id: albumId }, skip: !open });
  const album = data?.album;
  if (!album || album.id !== albumId) return null;
  return <LoadedAlbumSettings album={album} open={open} onOpenChange={onOpenChange} chosenTab={tab} onTabChange={onTabChange} onRemoved={onRemoved} />;
}

function LoadedAlbumSettings({ album, open, onOpenChange, chosenTab, onTabChange, onRemoved }: {
  album: Album;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  chosenTab: AlbumSettingsTab;
  onTabChange: (tab: AlbumSettingsTab) => void;
  onRemoved?: () => void;
}) {
  const navigate = useNavigate();
  const isAdmin = useIsAdmin();
  const tab = isAdmin || READ_ONLY_TABS.includes(chosenTab) ? chosenTab : READ_ONLY_TABS[0]!;
  const files = useTitleFiles("album", album.id, !open);
  const [update] = useMutation(EntityAlbumUpdateDocument);
  const [remove, { loading: removing }] = useMutation(EntityAlbumDeleteDocument);
  const [confirmRemove, setConfirmRemove] = useState(false);

  const form = useAppForm({
    ...formBehaviour,
    defaultValues: valuesOf(album),
    validators: { onDynamic: schema },
    onSubmit: async ({ value, formApi }) => {
      const { year, autoDownloadMode, ...input } = changedFields(valuesOf(album), value);
      const { data } = await update({
        variables: {
          id: album.id,
          input: {
            ...input,
            ...(year !== undefined ? { year: yearValue(value.year) } : {}),
            ...(autoDownloadMode ? { autoDownloadMode, autoDownload: autoDownloadMode !== "NONE" } : {}),
          },
        },
      });
      assertSuccess(data?.updateAlbum, "Could not save the album");
      formApi.reset(value);
    },
  });
  useEffect(() => { if (open) form.reset(valuesOf(album)); }, [open, album.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const { actions, dialogs } = useAlbumActions<Album>({ settings: (_, next) => onTabChange(next ?? "general"), open: false });
  const headerActions = useMemo(() => withoutAction(actions, "settings", "remove"), [actions]);
  const onRemove = async () => {
    try {
      assertSuccess((await remove({ variables: { id: album.id } })).data?.deleteAlbum, "Could not remove the album");
      toast.success("Album removed from the library");
      onOpenChange(false);
      if (onRemoved) onRemoved();
      else await navigate({ to: "/libraries/$libraryId/albums", params: { libraryId: album.libraryId } });
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const allTabs: EntitySettingsTab[] = [
    {
      value: "general", label: "General", icon: <IconSettings />, fields: ["name", "sortName", "year", "releaseDate", "albumType", "label", "country", "genres"],
      content: (
        <div className="flex flex-col gap-3">
          <form.AppField name="name">{() => <MetadataTextField label="Name" required />}</form.AppField>
          <form.AppField name="sortName">{() => <MetadataTextField label="Sort name" hint="Leave blank to sort by the name without a leading The, A or An." />}</form.AppField>
          <div className="grid gap-3 sm:grid-cols-3">
            <form.AppField name="year">{() => <MetadataTextField label="Year" inputMode="numeric" />}</form.AppField>
            <form.AppField name="releaseDate">{() => <MetadataTextField label="Released" placeholder="YYYY-MM-DD" />}</form.AppField>
            <form.AppField name="albumType">{() => <MetadataTextField label="Type" />}</form.AppField>
          </div>
          <div className="grid gap-3 sm:grid-cols-2">
            <form.AppField name="label">{() => <MetadataTextField label="Label" />}</form.AppField>
            <form.AppField name="country">{() => <MetadataTextField label="Country" />}</form.AppField>
          </div>
          <form.AppField name="genres">{() => <MetadataTagsField label="Genres" placeholder="Add a genre" />}</form.AppField>
        </div>
      ),
    },
    {
      value: "artwork", label: "Artwork", icon: <IconImage />,
      content: <ArtworkSettings target={{ entityType: "ALBUM", entityId: album.id, title: album.name }} active={open && tab === "artwork"} />,
    },
    {
      value: "downloads", label: "Downloads", icon: <IconDownload />, fields: ["autoDownloadMode", "qualityProfileId"],
      content: (
        <DownloadSettingsFields libraryId={album.libraryId} scope={{ albumId: album.id }} active={open && tab === "downloads"}
          mode={<form.AppField name="autoDownloadMode">{() => <AutoDownloadField />}</form.AppField>}
          profile={(options) => <form.AppField name="qualityProfileId">{(field) => <field.SelectField label="Quality profile" options={options} />}</form.AppField>}
        />
      ),
    },
    {
      value: "files", label: "Files", icon: <IconFile />, count: files.count,
      content: tab === "files" && files.scope ? <div className="entity-tab-table flex min-h-0 flex-1 flex-col overflow-hidden"><FilesBrowser libraryId={album.libraryId} scope={files.scope} filterPrefix="file" /></div> : null,
    },
    {
      value: "info", label: "Info", icon: <IconInfo />,
      content: (
        <DetailGrid label="About this album">
          <DetailTile icon={<IconClock />} label="Length" numeric value={album.totalDurationSecs ? formatRuntime(album.totalDurationSecs) : "Unknown"} />
          <DetailTile icon={<IconInfo />} label="Tracks" numeric value={album.trackCount ?? "Unknown"} detail={album.discCount && album.discCount > 1 ? `${album.discCount} discs` : undefined} />
          <DetailTile icon={<IconFile />} label="Size" numeric value={album.sizeBytes ? formatBytes(album.sizeBytes) : "Unknown"} />
          <DetailTile icon={<IconInfo />} label="MusicBrainz" value={album.musicbrainzId ? <a className="nav-focus rounded text-brand hover:underline" href={`https://musicbrainz.org/release-group/${album.musicbrainzId}`} target="_blank" rel="noreferrer">Open</a> : "None"} />
          <DetailTile icon={<IconClock />} label="Added" value={formatDateTime(album.createdAt)} />
          <DetailTile icon={<IconClock />} label="Updated" value={formatDateTime(album.updatedAt)} />
        </DetailGrid>
      ),
    },
    {
      value: "remove", label: "Remove", icon: <IconTrash />,
      content: (
        <div className="flex flex-col items-start gap-3">
          <p className="text-body-sm text-muted">The album and its tracks leave the catalogue. Files stay on disk.</p>
          <Button variant="danger-soft" onPress={() => setConfirmRemove(true)}><IconTrash size="md" /> Remove from library</Button>
        </div>
      ),
    },
  ];
  const tabs = isAdmin ? allTabs : allTabs.filter((item) => READ_ONLY_TABS.includes(item.value as AlbumSettingsTab));

  return (
    <>
      <EntitySettingsDialog
        open={open}
        onOpenChange={onOpenChange}
        form={form}
        size="xl"
        identity={albumIdentityProps(album, { actions: headerActions })}
        tabs={tabs}
        tab={tab}
        onTabChange={(next) => onTabChange(next as AlbumSettingsTab)}
        saveDisabledReason={isAdmin ? undefined : "Only an admin can change an album"}
      />
      <ConfirmDialog isOpen={confirmRemove} onOpenChange={setConfirmRemove} title={`Remove ${album.name}?`} description="The album and its tracks leave the catalogue. Files stay on disk." confirmLabel="Remove" destructive isPending={removing} onConfirm={onRemove} />
      {dialogs}
    </>
  );
}
