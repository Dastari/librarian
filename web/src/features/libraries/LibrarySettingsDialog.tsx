import { useMutation } from "@apollo/client/react";
import { useNavigate } from "@tanstack/react-router";
import { ActionBar } from "glass-ux/actions";
import { EntitySettingsDialog, type EntitySettingsTab } from "glass-ux/entity-settings";
import { formBehaviour, useAppForm } from "glass-ux/form";
import { IconClock, IconFile, IconFolder, IconHistory, IconSettings, IconTrash, IconWarning } from "glass-ux/icons";
import { Surface } from "glass-ux/surface";
import { toast } from "glass-ux/toast";
import { useEffect, useMemo, useState } from "react";
import { z } from "zod";

import { Button, ConfirmDialog } from "@/components/ui";
import { useMediaQuery } from "@/hooks/useMediaQuery";
import {
  EntityLibraryDeleteDocument,
  EntityLibraryUpdateDocument,
  NamingPatternsForTypeDocument,
  QualityProfilesListDocument,
  type LibraryFieldsFragment,
} from "@/graphql/generated/graphql";
import { useIsAdmin } from "@/lib/auth/useSession";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

import { FilesBrowser } from "./browser/FilesBrowser";
import { FolderField } from "./FolderField";
import { LibraryIdentity } from "./LibraryIdentity";
import { previewNamingPattern } from "./namingPreview";
import { ScansPanel } from "./ScansPanel";
import { useLibraryActions } from "./useLibraryActions";

export const LIBRARY_SETTINGS_TABS = ["general", "scanning", "organization", "files", "issues", "history", "remove"] as const;
export type LibrarySettingsTab = (typeof LIBRARY_SETTINGS_TABS)[number];
export const isLibrarySettingsTab = (value: string | null): value is LibrarySettingsTab => LIBRARY_SETTINGS_TABS.includes(value as LibrarySettingsTab);
/** What someone who can't change a library can still look at. */
const READ_ONLY_TABS: readonly LibrarySettingsTab[] = ["files", "issues", "history"];

const schema = z.object({
  name: z.string().trim().min(1, "Give the library a name").max(80),
  path: z.string().trim().min(1, "Choose the folder that holds the media"),
  autoScan: z.boolean(),
  scanIntervalMinutes: z.number().int().min(5, "At least 5 minutes").max(10080, "At most a week"),
  watchForChanges: z.boolean(),
  autoOrganize: z.boolean(),
  namingPattern: z.string(),
  qualityProfileId: z.string(),
});

const valuesOf = (library: LibraryFieldsFragment) => ({
  name: library.name,
  path: library.path,
  autoScan: library.autoScan,
  scanIntervalMinutes: library.scanIntervalMinutes,
  watchForChanges: library.watchForChanges,
  autoOrganize: library.autoOrganize,
  namingPattern: library.namingPattern || "",
  qualityProfileId: library.qualityProfileId ?? "",
});

/** A library's settings and its scanning record, with its identity and actions in the header. */
export function LibrarySettingsDialog({ library, open, onOpenChange, tab: chosenTab, onTabChange, items, issues = 0, fileCount }: {
  library: LibraryFieldsFragment;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  tab: LibrarySettingsTab;
  onTabChange: (tab: LibrarySettingsTab) => void;
  items?: number | null;
  issues?: number;
  fileCount?: number | null;
}) {
  const navigate = useNavigate();
  const isAdmin = useIsAdmin();
  const narrow = useMediaQuery("(max-width: 39.99rem)");
  const tab = isAdmin || READ_ONLY_TABS.includes(chosenTab) ? chosenTab : READ_ONLY_TABS[0]!;
  const [update] = useMutation(EntityLibraryUpdateDocument);
  const [remove, { loading: deleting }] = useMutation(EntityLibraryDeleteDocument);
  const patterns = useQuery(NamingPatternsForTypeDocument, { variables: { libraryType: library.libraryType.toLowerCase() } });
  const profiles = useQuery(QualityProfilesListDocument);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [scanRunId, setScanRunId] = useState<string | null>(null);

  const form = useAppForm({
    ...formBehaviour,
    defaultValues: valuesOf(library),
    validators: { onDynamic: schema },
    onSubmit: async ({ value, formApi }) => {
      const { data } = await update({ variables: { id: library.id, input: { ...value, qualityProfileId: value.qualityProfileId || null } } });
      assertSuccess(data?.updateLibrary, "Could not save the library");
      formApi.reset(value);
    },
  });
  // Another library, or this one reopened: start from what's saved.
  useEffect(() => { if (open) form.reset(valuesOf(library)); }, [open, library.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const actions = useLibraryActions<LibraryFieldsFragment>({ settings: (_, next) => onTabChange(next ?? "general"), open: false });
  const onDelete = async () => {
    try {
      const { data } = await remove({ variables: { id: library.id } });
      assertSuccess(data?.deleteLibrary, "Could not delete the library");
      toast.success("Library removed. Files on disk were left untouched.");
      onOpenChange(false);
      await navigate({ to: "/libraries" });
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const defaultPattern = useMemo(() => (patterns.data?.namingPatterns.edges ?? []).find(({ node }) => node.isDefault)?.node.pattern ?? "", [patterns.data]);
  const previewOf = (pattern: string) => previewNamingPattern(pattern || defaultPattern, library.libraryType);
  const patternOptions = [{ value: "", label: "Librarian default", description: previewOf("") }, ...(patterns.data?.namingPatterns.edges ?? []).map(({ node }) => ({ value: node.pattern, label: node.name, description: previewOf(node.pattern) }))];
  const profileOptions = useMemo(() => [{ value: "", label: "Use the default profile" }, ...(profiles.data?.qualityProfiles.edges ?? []).map(({ node }) => ({ value: node.id, label: node.name, description: node.isDefault ? "Default" : undefined }))], [profiles.data]);

  const allTabs: EntitySettingsTab[] = [
    {
      value: "general", label: "General", icon: <IconSettings />, fields: ["name", "path", "qualityProfileId"],
      content: (
        <div className="flex flex-col gap-3">
          <form.AppField name="name">{(field) => <field.TextField label="Name" required />}</form.AppField>
          <form.AppField name="path">{() => <FolderField required />}</form.AppField>
          <form.AppField name="qualityProfileId">{(field) => <field.SelectField label="Quality profile" options={profileOptions} />}</form.AppField>
        </div>
      ),
    },
    {
      value: "scanning", label: "Scanning", icon: <IconClock />, fields: ["autoScan", "scanIntervalMinutes", "watchForChanges"],
      content: (
        <div className="flex flex-col gap-3">
          <form.AppField name="autoScan">{(field) => <field.SwitchRow label="Scan on a schedule" description="Look for new, changed and missing files automatically." />}</form.AppField>
          <form.AppField name="scanIntervalMinutes">{(field) => <field.NumberField label="Scan every (minutes)" min={5} max={10080} step={5} />}</form.AppField>
          <form.AppField name="watchForChanges">{(field) => <field.SwitchRow label="Watch the folder" description="React to file changes between scans." />}</form.AppField>
        </div>
      ),
    },
    {
      value: "organization", label: "Organization", icon: <IconFolder />, fields: ["autoOrganize", "namingPattern"],
      content: (
        <div className="flex flex-col gap-3">
          <form.AppField name="autoOrganize">{(field) => <field.SwitchRow label="Organize files" description="Rename and move matched files into the naming pattern." />}</form.AppField>
          <form.AppField name="namingPattern">{(field) => <field.SelectField label="Naming pattern" options={patternOptions} />}</form.AppField>
          <form.Subscribe selector={(state) => state.values.namingPattern}>{(pattern) => (
            <Surface material="inset" padding="tile" radius="tile" className="flex min-w-0 flex-col gap-1">
              <p className="text-label-sm text-muted">A file would go here</p>
              <p className="font-mono text-label-sm text-foreground [overflow-wrap:anywhere]">{previewOf(pattern)}</p>
            </Surface>
          )}</form.Subscribe>
        </div>
      ),
    },
    {
      value: "files", label: "Files", icon: <IconFile />, count: fileCount ?? undefined,
      content: tab === "files" ? <div className="entity-tab-table flex min-h-0 flex-1 flex-col overflow-hidden"><FilesBrowser libraryId={library.id} filterPrefix="file" /></div> : null,
    },
    {
      value: "issues", label: "Issues", icon: <IconWarning />, count: issues > 0 ? issues : undefined,
      content: tab === "issues" ? <ScansPanel libraryId={library.id} section="issues" scanRunId={scanRunId} onScanRunChange={setScanRunId} onShowIssues={setScanRunId} /> : null,
    },
    {
      value: "history", label: "Scan history", icon: <IconHistory />,
      content: tab === "history" ? <ScansPanel libraryId={library.id} section="history" scanRunId={null} onScanRunChange={setScanRunId} onShowIssues={(id) => { setScanRunId(id); onTabChange("issues"); }} /> : null,
    },
    {
      value: "remove", label: "Remove", icon: <IconTrash />,
      content: (
        <div className="flex flex-col items-start gap-3">
          <p className="text-body-sm text-muted">Removing the library deletes its catalogue from Librarian. Files on disk are never touched.</p>
          <Button variant="danger-soft" onPress={() => setConfirmDelete(true)}><IconTrash size="md" /> Remove library</Button>
        </div>
      ),
    },
  ];
  const tabs = isAdmin ? allTabs : allTabs.filter((item) => READ_ONLY_TABS.includes(item.value as LibrarySettingsTab));

  return (
    <>
      <EntitySettingsDialog
        open={open}
        onOpenChange={onOpenChange}
        form={form}
        size="xl"
        entityName={library.name}
        identity={(
          <LibraryIdentity
            tier={narrow ? "compact" : "header"}
            heading="h2"
            library={library}
            items={items}
            issues={issues}
            actions={actions}
            headerActions={narrow ? undefined : <ActionBar set={actions} targets={[library]} label={`Actions for ${library.name}`} maxButtons={1} />}
          />
        )}
        tabs={tabs}
        tab={tab}
        onTabChange={(next) => onTabChange(next as LibrarySettingsTab)}
        saveDisabledReason={isAdmin ? undefined : "Only an admin can change a library"}
      />
      <ConfirmDialog
        isOpen={confirmDelete}
        onOpenChange={setConfirmDelete}
        title={`Remove ${library.name}?`}
        description="The catalogue, playback history and match records for this library are deleted. Media files stay where they are."
        confirmLabel="Remove library"
        destructive
        isPending={deleting}
        onConfirm={onDelete}
      />
    </>
  );
}
