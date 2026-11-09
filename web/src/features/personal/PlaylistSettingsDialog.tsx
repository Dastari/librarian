import { useMutation } from "@apollo/client/react";
import { useNavigate } from "@tanstack/react-router";
import { EntitySettingsDialog, type EntitySettingsTab } from "glass-ux/entity-settings";
import { formBehaviour, useAppForm } from "glass-ux/form";
import { IconClock, IconEdit, IconImage, IconInfo, IconSettings, IconTrash } from "glass-ux/icons";
import { toast } from "glass-ux/toast";
import { useEffect, useMemo, useState } from "react";
import { z } from "zod";

import { DetailGrid, DetailTile } from "@/components/entities/EntityDetailDialog";
import { ArtworkSettings } from "@/features/artwork/ArtworkSettings";
import { Button, ConfirmDialog } from "@/components/ui";
import {
  EntityPersonalListDeleteDocument, EntityPersonalListUpdateDocument, EntitySavedViewGetDocument, PlaylistGetDocument,
  type PlaylistGetQuery,
} from "@/graphql/generated/graphql";
import { formatDateTime } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { withoutAction } from "@/lib/actions/withoutAction";

import { ListEditor } from "./ListEditor";
import { playlistIdentityProps, playlistKindLabel } from "./PlaylistIdentity";
import { usePlaylistActions } from "./usePlaylistActions";
import { useHomeShelves } from "./usePersonalLibrary";

type Playlist = NonNullable<PlaylistGetQuery["personalList"]>;

export const PLAYLIST_SETTINGS_TABS = ["general", "rules", "artwork", "info", "remove"] as const;
export type PlaylistSettingsTab = (typeof PLAYLIST_SETTINGS_TABS)[number];

const schema = z.object({
  name: z.string().trim().min(1, "Give the playlist a name").max(120),
  description: z.string().max(1000),
});
const valuesOf = (list: Playlist) => ({ name: list.name, description: list.description ?? "" });

/** A playlist's name and description, rules and facts, with its identity and actions in the header. */
export function PlaylistSettingsDialog({ listId, open, onOpenChange, tab, onTabChange, onRemoved }: {
  listId: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  tab: PlaylistSettingsTab;
  onTabChange: (tab: PlaylistSettingsTab) => void;
  onRemoved?: () => void;
}) {
  const { data } = useQuery(PlaylistGetDocument, { variables: { id: listId }, skip: !open });
  const list = data?.personalList;
  if (!list || list.id !== listId) return null;
  return <LoadedPlaylistSettings list={list} open={open} onOpenChange={onOpenChange} chosenTab={tab} onTabChange={onTabChange} onRemoved={onRemoved} />;
}

function LoadedPlaylistSettings({ list, open, onOpenChange, chosenTab, onTabChange, onRemoved }: {
  list: Playlist;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  chosenTab: PlaylistSettingsTab;
  onTabChange: (tab: PlaylistSettingsTab) => void;
  onRemoved?: () => void;
}) {
  const navigate = useNavigate();
  const smart = list.kind === "SMART";
  const tab = chosenTab === "rules" && !smart ? "general" : chosenTab;
  const [update] = useMutation(EntityPersonalListUpdateDocument);
  const [remove, { loading: removing }] = useMutation(EntityPersonalListDeleteDocument);
  const view = useQuery(EntitySavedViewGetDocument, { variables: { id: list.savedViewId ?? "" }, skip: !open || !list.savedViewId });
  const home = useHomeShelves();
  const [confirmRemove, setConfirmRemove] = useState(false);
  const [editingRules, setEditingRules] = useState(false);

  const form = useAppForm({
    ...formBehaviour,
    defaultValues: valuesOf(list),
    validators: { onDynamic: schema },
    onSubmit: async ({ value, formApi }) => {
      const input = { ...(list.kind === "WATCH_LATER" ? {} : { name: value.name.trim() }), description: value.description.trim() || null };
      const { data } = await update({ variables: { id: list.id, input } });
      assertSuccess(data?.updatePersonalList, "Could not save the playlist");
      formApi.reset(value);
    },
  });
  useEffect(() => { if (open) form.reset(valuesOf(list)); }, [open, list.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const actions = usePlaylistActions<Playlist>({ settings: (_, next) => onTabChange(next ?? "general"), open: false });
  const headerActions = useMemo(() => withoutAction(actions, "settings", "remove", "rename", "rules"), [actions]);
  const onRemove = async () => {
    try {
      assertSuccess((await remove({ variables: { id: list.id } })).data?.deletePersonalList, "Could not delete the playlist");
      toast.success("Playlist deleted");
      onOpenChange(false);
      if (onRemoved) onRemoved();
      else await navigate({ to: "/lists" });
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const count = list.entries.pageInfo.totalCount ?? 0;
  const pinned = home.rows.some((shelf) => shelf.listId === list.id && shelf.visible);
  const allTabs: EntitySettingsTab[] = [
    {
      value: "general", label: "General", icon: <IconSettings />, fields: ["name", "description"],
      content: (
        <div className="flex flex-col gap-3">
          <form.AppField name="name">{(field) => <field.TextField label="Name" required disabled={list.kind === "WATCH_LATER"} hint={list.kind === "WATCH_LATER" ? "Watch Later keeps its name." : undefined} />}</form.AppField>
          <form.AppField name="description">{(field) => <field.TextField label="Description" optional />}</form.AppField>
        </div>
      ),
    },
    ...(smart ? [{
      value: "rules", label: "Rules", icon: <IconEdit />,
      content: (
        <div className="flex flex-col items-start gap-3">
          <p className="text-body-sm text-muted">{view.data?.savedView ? `${view.data.savedView.contentType.toLowerCase()} titles${view.data.savedView.titleContains ? ` containing “${view.data.savedView.titleContains}”` : ""}, ${view.data.savedView.availability === "ANY" ? "with or without files" : view.data.savedView.availability === "AVAILABLE" ? "with a file" : "missing files"}.` : "Loading the rules…"}</p>
          <Button isDisabled={!view.data?.savedView} onPress={() => setEditingRules(true)}><IconEdit size="md" /> Edit rules</Button>
        </div>
      ),
    }] : []),
    {
      value: "artwork", label: "Cover", icon: <IconImage />,
      content: <ArtworkSettings target={{ entityType: "PLAYLIST", entityId: list.id, title: list.name }} active={open && tab === "artwork"} />,
    },
    {
      value: "info", label: "Info", icon: <IconInfo />,
      content: (
        <DetailGrid label="About this playlist">
          <DetailTile icon={<IconInfo />} label="Kind" value={playlistKindLabel(list.kind)} />
          {smart ? null : <DetailTile icon={<IconInfo />} label="Items" numeric value={count} />}
          <DetailTile icon={<IconInfo />} label="On Home" value={pinned ? "Yes" : "No"} />
          <DetailTile icon={<IconClock />} label="Created" value={formatDateTime(list.createdAt)} />
          <DetailTile icon={<IconClock />} label="Updated" value={formatDateTime(list.updatedAt)} />
        </DetailGrid>
      ),
    },
    {
      value: "remove", label: "Delete", icon: <IconTrash />,
      content: (
        <div className="flex flex-col items-start gap-3">
          <p className="text-body-sm text-muted">The playlist is deleted. The media in it, and every download setting, stay as they are.</p>
          <Button variant="danger-soft" onPress={() => setConfirmRemove(true)}><IconTrash size="md" /> Delete playlist</Button>
        </div>
      ),
    },
  ];

  return (
    <>
      <EntitySettingsDialog
        open={open}
        onOpenChange={onOpenChange}
        form={form}
        size="xl"
        identity={playlistIdentityProps(list, { actions: headerActions, pinned })}
        tabs={allTabs}
        tab={tab}
        onTabChange={(next) => onTabChange(next as PlaylistSettingsTab)}
      />
      <ConfirmDialog isOpen={confirmRemove} onOpenChange={setConfirmRemove} title={`Delete ${list.name}?`} description="The playlist is removed. Media and download settings stay as they are." confirmLabel="Delete playlist" destructive isPending={removing} onConfirm={onRemove} />
      {editingRules && view.data?.savedView ? <ListEditor open onOpenChange={setEditingRules} initial={view.data.savedView} personalListId={list.id} /> : null}
    </>
  );
}
