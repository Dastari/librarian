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
import { withArtworkLocks } from "@/features/artwork/slots";
import { Button, ConfirmDialog } from "@/components/ui";
import { FilesBrowser } from "@/features/libraries/browser/FilesBrowser";
import { AudiobookDetailDocument, EntityAudiobookDeleteDocument, EntityAudiobookUpdateDocument, type AudiobookDetailQuery } from "@/graphql/generated/graphql";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatBytes, formatDateTime, formatRuntime } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { withoutAction } from "@/lib/actions/withoutAction";

import { audiobookIdentityProps } from "./AudiobookIdentity";
import { AutoDownloadField, DownloadSettingsFields } from "./entity/DownloadSettings";
import { changedFields } from "./entity/metadataInput";
import { MetadataTagsField, MetadataTextareaField, MetadataTextField } from "./entity/MetadataFields";
import { useTitleFiles } from "./entity/useTitleFiles";
import { useAudiobookActions, type AudiobookSettingsTab } from "./useAudiobookActions";

type Book = NonNullable<AudiobookDetailQuery["audiobook"]>;

const READ_ONLY_TABS: readonly AudiobookSettingsTab[] = ["files", "info"];

const schema = z.object({
  title: z.string().trim().min(1, "Give the book a title"),
  publishedDate: z.string().trim().regex(/^(\d{4}(-\d{2}(-\d{2})?)?)?$/, "As YYYY or YYYY-MM-DD"),
});

const valuesOf = (book: Book) => ({
  title: book.title,
  sortTitle: book.sortTitle ?? "",
  authorName: book.authorName ?? "",
  narrators: book.narrators.length > 0 ? book.narrators : book.narratorName ? [book.narratorName] : [],
  seriesName: book.seriesName ?? "",
  seriesPosition: book.seriesPosition ?? "",
  publisher: book.publisher ?? "",
  publishedDate: book.publishedDate ?? "",
  language: book.language ?? "",
  description: book.description ?? "",
  lockedFields: book.lockedFields ?? [],
  autoDownloadMode: book.autoDownloadMode,
  qualityProfileId: book.qualityProfileId ?? "",
});

/** A book's details and how it's fetched, with its identity and actions in the header. */
export function AudiobookSettingsDialog({ audiobookId, open, onOpenChange, tab, onTabChange, onRemoved }: {
  audiobookId: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  tab: AudiobookSettingsTab;
  onTabChange: (tab: AudiobookSettingsTab) => void;
  onRemoved?: () => void;
}) {
  const { data, refetch } = useQuery(AudiobookDetailDocument, { variables: { id: audiobookId }, skip: !open });
  const book = data?.audiobook;
  if (!book || book.id !== audiobookId) return null;
  return <LoadedAudiobookSettings book={book} open={open} onOpenChange={onOpenChange} chosenTab={tab} onTabChange={onTabChange} onRemoved={onRemoved} onChanged={() => void refetch()} />;
}

function LoadedAudiobookSettings({ book, open, onOpenChange, chosenTab, onTabChange, onRemoved, onChanged }: {
  book: Book;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  chosenTab: AudiobookSettingsTab;
  onTabChange: (tab: AudiobookSettingsTab) => void;
  onRemoved?: () => void;
  onChanged: () => void;
}) {
  const navigate = useNavigate();
  const isAdmin = useIsAdmin();
  const tab = isAdmin || READ_ONLY_TABS.includes(chosenTab) ? chosenTab : READ_ONLY_TABS[0]!;
  const files = useTitleFiles("audiobook", book.id, !open);
  const [update] = useMutation(EntityAudiobookUpdateDocument);
  const [remove, { loading: removing }] = useMutation(EntityAudiobookDeleteDocument);
  const [confirmRemove, setConfirmRemove] = useState(false);

  const form = useAppForm({
    ...formBehaviour,
    defaultValues: valuesOf(book),
    validators: { onDynamic: schema },
    onSubmit: async ({ value, formApi }) => {
      const { autoDownloadMode, narrators, ...input } = changedFields(valuesOf(book), value);
      const { data } = await update({
        variables: {
          id: book.id,
          input: {
            ...input,
            ...(narrators ? { narrators, narratorName: narrators[0] ?? null } : {}),
            ...(autoDownloadMode ? { autoDownloadMode, autoDownload: autoDownloadMode !== "NONE" } : {}),
            // A picture chosen while the form was open keeps its lock.
            ...(input.lockedFields !== undefined ? { lockedFields: withArtworkLocks(value.lockedFields, book.lockedFields) } : {}),
          },
        },
      });
      assertSuccess(data?.updateAudiobook, "Could not save the audiobook");
      formApi.reset(value);
    },
  });
  useEffect(() => { if (open) form.reset(valuesOf(book)); }, [open, book.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const { actions, dialogs } = useAudiobookActions<Book>({ settings: (_, next) => onTabChange(next ?? "general"), open: false, onChanged });
  const headerActions = useMemo(() => withoutAction(actions, "settings", "remove"), [actions]);
  const onRemove = async () => {
    try {
      assertSuccess((await remove({ variables: { id: book.id } })).data?.deleteAudiobook, "Could not remove the audiobook");
      toast.success("Audiobook removed");
      onOpenChange(false);
      if (onRemoved) onRemoved();
      else await navigate({ to: "/libraries/$libraryId/audiobooks", params: { libraryId: book.libraryId } });
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const allTabs: EntitySettingsTab[] = [
    {
      value: "general", label: "General", icon: <IconSettings />, fields: ["title", "sortTitle", "authorName", "narrators", "seriesName", "seriesPosition", "publisher", "publishedDate", "language", "description"],
      content: (
        <div className="flex flex-col gap-3">
          <form.AppField name="title">{() => <MetadataTextField label="Title" required />}</form.AppField>
          <form.AppField name="sortTitle">{() => <MetadataTextField label="Sort title" hint="Leave blank to sort by the title without a leading The, A or An." />}</form.AppField>
          <form.AppField name="authorName">{() => <MetadataTextField label="Author" />}</form.AppField>
          <form.AppField name="narrators">{() => <MetadataTagsField label="Narrators" placeholder="Add a narrator" />}</form.AppField>
          <div className="grid gap-3 sm:grid-cols-[1fr_8rem]">
            <form.AppField name="seriesName">{() => <MetadataTextField label="Series" />}</form.AppField>
            <form.AppField name="seriesPosition">{() => <MetadataTextField label="Number" />}</form.AppField>
          </div>
          <div className="grid gap-3 sm:grid-cols-3">
            <form.AppField name="publisher">{() => <MetadataTextField label="Publisher" />}</form.AppField>
            <form.AppField name="publishedDate">{() => <MetadataTextField label="Published" placeholder="YYYY-MM-DD" />}</form.AppField>
            <form.AppField name="language">{() => <MetadataTextField label="Language" />}</form.AppField>
          </div>
          <form.AppField name="description">{() => <MetadataTextareaField label="Summary" minRows={5} />}</form.AppField>
        </div>
      ),
    },
    {
      value: "artwork", label: "Artwork", icon: <IconImage />,
      content: <ArtworkSettings target={{ entityType: "AUDIOBOOK", entityId: book.id, title: book.title }} active={open && tab === "artwork"} />,
    },
    {
      value: "downloads", label: "Downloads", icon: <IconDownload />, fields: ["autoDownloadMode", "qualityProfileId"],
      content: (
        <DownloadSettingsFields libraryId={book.libraryId} scope={{ audiobookId: book.id }} active={open && tab === "downloads"}
          mode={<form.AppField name="autoDownloadMode">{() => <AutoDownloadField />}</form.AppField>}
          profile={(options) => <form.AppField name="qualityProfileId">{(field) => <field.SelectField label="Quality profile" options={options} />}</form.AppField>}
        />
      ),
    },
    {
      value: "files", label: "Files", icon: <IconFile />, count: files.count,
      content: tab === "files" && files.scope ? <div className="entity-tab-table flex min-h-0 flex-1 flex-col overflow-hidden"><FilesBrowser libraryId={book.libraryId} scope={files.scope} filterPrefix="file" /></div> : null,
    },
    {
      value: "info", label: "Info", icon: <IconInfo />,
      content: (
        <DetailGrid label="About this book">
          <DetailTile icon={<IconClock />} label="Length" numeric value={book.totalDurationSecs ? formatRuntime(book.totalDurationSecs) : "Unknown"} />
          <DetailTile icon={<IconInfo />} label="Chapters" numeric value={book.chapterCount ?? "Unknown"} detail={book.chapterSource ?? undefined} />
          <DetailTile icon={<IconFile />} label="Size" numeric value={book.sizeBytes ? formatBytes(book.sizeBytes) : "Unknown"} detail={book.fileShape ?? undefined} />
          <DetailTile icon={<IconInfo />} label="ASIN" value={book.asin ?? "None"} />
          <DetailTile icon={<IconInfo />} label="ISBN" value={book.isbn ?? "None"} />
          <DetailTile icon={<IconClock />} label="Added" value={formatDateTime(book.createdAt)} />
          <DetailTile icon={<IconClock />} label="Updated" value={formatDateTime(book.updatedAt)} />
        </DetailGrid>
      ),
    },
    {
      value: "remove", label: "Remove", icon: <IconTrash />,
      content: (
        <div className="flex flex-col items-start gap-3">
          <p className="text-body-sm text-muted">The audiobook and its chapters leave the catalogue. Files stay on disk.</p>
          <Button variant="danger-soft" onPress={() => setConfirmRemove(true)}><IconTrash size="md" /> Remove from library</Button>
        </div>
      ),
    },
  ];
  const tabs = isAdmin ? allTabs : allTabs.filter((item) => READ_ONLY_TABS.includes(item.value as AudiobookSettingsTab));

  return (
    <>
      <EntitySettingsDialog
        open={open}
        onOpenChange={onOpenChange}
        form={form}
        size="xl"
        identity={audiobookIdentityProps(book, { actions: headerActions })}
        tabs={tabs}
        tab={tab}
        onTabChange={(next) => onTabChange(next as AudiobookSettingsTab)}
        saveDisabledReason={isAdmin ? undefined : "Only an admin can change an audiobook"}
      />
      <ConfirmDialog isOpen={confirmRemove} onOpenChange={setConfirmRemove} title={`Remove ${book.title}?`} description="The audiobook and its chapters leave the catalogue. Files stay on disk." confirmLabel="Remove" destructive isPending={removing} onConfirm={onRemove} />
      {dialogs}
    </>
  );
}
