import { useMutation, useQuery } from "@apollo/client/react";
import { toast } from "@heroui/react";
import { zodResolver } from "@hookform/resolvers/zod";
import { IconPlus, IconStar, IconStarFilled, IconTags, IconTrash } from "@tabler/icons-react";
import { useEffect, useState, type ReactNode } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { Button, ConfirmDialog, DataTable, type DataTableColumn, type DataTableRowAction, Dialog, EmptyState, FieldGroup, FormNumberField, FormSelectField, FormSwitchField, FormTextField, Panel, StatusChip } from "@/components/ui";
import {
  EntityQualityProfileCreateDocument,
  EntityQualityProfileDeleteDocument,
  EntityQualityProfileUpdateDocument,
  QualityProfilesListDocument,
  type QualityProfileFieldsFragment,
} from "@/graphql/generated/graphql";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { LANGUAGES, languageName } from "@/lib/languages";

import { FormOrderedChips } from "./OrderedChips";

const list = z.string();
const splitList = (value: string) => value.split(/[,\n]/).map((item) => item.trim()).filter(Boolean);
const optionalInt = z.number().int().min(0).nullable();
const schema = z.object({
  name: z.string().trim().min(1, "Name the profile"),
  mediaKind: z.enum(["VIDEO", "AUDIO"]),
  allowedResolutions: list,
  allowedVideoCodecs: list,
  allowedAudioFormats: list,
  allowedHdrTypes: list,
  allowedSources: list,
  releaseGroupWhitelist: list,
  releaseGroupBlacklist: list,
  requireHdr: z.boolean(),
  cutoffResolution: z.string().trim(),
  upgradeUntilCutoff: z.boolean(),
  isDefault: z.boolean(),
  preferredLanguages: z.array(z.string()),
  requireLanguageMatch: z.boolean(),
  minSizeMb: optionalInt,
  maxSizeMb: optionalInt,
  minSeeders: z.number().int().min(0),
  maxReleaseAgeDays: optionalInt,
  preferredReleaseGroups: z.array(z.string()),
  allowSeasonPacks: z.boolean(),
  preferProperRepack: z.boolean(),
  resolutionPreference: z.array(z.string()),
});
type Input = z.infer<typeof schema>;

const toInput = (profile?: QualityProfileFieldsFragment | null): Input => ({
  name: profile?.name ?? "",
  mediaKind: profile?.mediaKind ?? "VIDEO",
  allowedResolutions: profile?.allowedResolutions.join(", ") ?? "2160p, 1080p, 720p",
  allowedVideoCodecs: profile?.allowedVideoCodecs.join(", ") ?? "hevc, h264, av1",
  allowedAudioFormats: profile?.allowedAudioFormats.join(", ") ?? "",
  allowedHdrTypes: profile?.allowedHdrTypes.join(", ") ?? "",
  allowedSources: profile?.allowedSources.join(", ") ?? "bluray, web",
  releaseGroupWhitelist: profile?.releaseGroupWhitelist.join(", ") ?? "",
  releaseGroupBlacklist: profile?.releaseGroupBlacklist.join(", ") ?? "",
  requireHdr: profile?.requireHdr ?? false,
  cutoffResolution: profile?.cutoffResolution ?? "",
  upgradeUntilCutoff: profile?.upgradeUntilCutoff ?? false,
  isDefault: profile?.isDefault ?? false,
  preferredLanguages: profile?.preferredLanguages ?? [],
  requireLanguageMatch: profile?.requireLanguageMatch ?? false,
  minSizeMb: profile?.minSizeMb ?? null,
  maxSizeMb: profile?.maxSizeMb ?? null,
  minSeeders: profile?.minSeeders ?? 1,
  maxReleaseAgeDays: profile?.maxReleaseAgeDays ?? null,
  preferredReleaseGroups: profile?.preferredReleaseGroups ?? [],
  allowSeasonPacks: profile?.allowSeasonPacks ?? true,
  preferProperRepack: profile?.preferProperRepack ?? true,
  resolutionPreference: profile?.resolutionPreference ?? [],
});

export function QualitySettings() {
  const { data, previousData, loading, refetch } = useQuery(QualityProfilesListDocument);
  const profiles = (data ?? previousData)?.qualityProfiles.edges.map((edge) => edge.node) ?? [];
  const [editing, setEditing] = useState<QualityProfileFieldsFragment | null | "new">(null);
  const [removing, setRemoving] = useState<QualityProfileFieldsFragment | null>(null);
  const [create, { loading: creating }] = useMutation(EntityQualityProfileCreateDocument);
  const [update, { loading: updating }] = useMutation(EntityQualityProfileUpdateDocument);
  const [remove, { loading: deleting }] = useMutation(EntityQualityProfileDeleteDocument);
  const form = useForm<Input>({ resolver: zodResolver(schema), defaultValues: toInput() });

  useEffect(() => {
    if (editing !== null) form.reset(toInput(editing === "new" ? null : editing));
  }, [editing, form]);

  const submit = form.handleSubmit(async (values) => {
    const input = {
      ...values,
      allowedResolutions: splitList(values.allowedResolutions),
      allowedVideoCodecs: splitList(values.allowedVideoCodecs),
      allowedAudioFormats: splitList(values.allowedAudioFormats),
      allowedHdrTypes: splitList(values.allowedHdrTypes),
      allowedSources: splitList(values.allowedSources),
      releaseGroupWhitelist: splitList(values.releaseGroupWhitelist),
      releaseGroupBlacklist: splitList(values.releaseGroupBlacklist),
      cutoffResolution: values.cutoffResolution || null,
      resolutionPreference: values.resolutionPreference.filter((resolution) => splitList(values.allowedResolutions).includes(resolution)),
    };
    try {
      if (editing && editing !== "new") assertSuccess((await update({ variables: { id: editing.id, input } })).data?.updateQualityProfile, "Could not save");
      else assertSuccess((await create({ variables: { input } })).data?.createQualityProfile, "Could not create");
      toast.success("Profile saved");
      setEditing(null);
      void refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  });

  const makeDefault = async (profile: QualityProfileFieldsFragment) => {
    try {
      for (const other of profiles.filter((item) => item.isDefault && item.mediaKind === profile.mediaKind && item.id !== profile.id)) {
        await update({ variables: { id: other.id, input: { isDefault: false } } });
      }
      assertSuccess((await update({ variables: { id: profile.id, input: { isDefault: true } } })).data?.updateQualityProfile, "Could not update");
      void refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const columns: Array<DataTableColumn<QualityProfileFieldsFragment>> = [
    {
      id: "name",
      header: "Profile",
      cell: (profile) => (
        <span className="flex items-center gap-2">
          {profile.isDefault ? <IconStarFilled size={14} className="text-brand" /> : null}
          <span className="min-w-0">
            <span className="block truncate text-body-sm text-foreground">{profile.name}</span>
            <span className="block truncate text-label-sm text-muted">{profile.mediaKind === "VIDEO" ? [profile.allowedResolutions.join("/"), profile.allowedVideoCodecs.join("/"), profile.requireHdr ? "HDR required" : null].filter(Boolean).join(" · ") : profile.allowedAudioFormats.join("/")}</span>
          </span>
        </span>
      ),
    },
    { id: "kind", header: "Kind", size: 100, cell: (profile) => <StatusChip status={{ label: profile.mediaKind === "VIDEO" ? "Video" : "Audio", tone: "accent", dot: "bg-info" }} /> },
    { id: "cutoff", header: "Upgrade until", size: 140, hideBelow: "md", cell: (profile) => <span className="text-muted">{profile.upgradeUntilCutoff ? (profile.cutoffResolution ?? "best available") : "Off"}</span> },
    { id: "sources", header: "Sources", size: 180, hideBelow: "lg", cell: (profile) => <span className="text-muted">{profile.allowedSources.join(", ") || "Any"}</span> },
  ];
  const actions: Array<DataTableRowAction<QualityProfileFieldsFragment>> = [
    { key: "default", label: "Make default", icon: <IconStar size={16} />, hidden: (profile) => profile.isDefault, onAction: makeDefault },
    { key: "delete", label: "Delete", icon: <IconTrash size={16} />, destructive: true, hidden: (profile) => profile.isDefault, onAction: (profile) => setRemoving(profile) },
  ];

  const kind = form.watch("mediaKind");
  const resolutions = splitList(form.watch("allowedResolutions"));

  return (
    <div className="flex flex-col gap-6">
      <Panel title="Quality profiles" description="Decide which releases are acceptable and when a better file should replace an existing one." flush actions={<Button variant="primary" size="sm" onPress={() => setEditing("new")}><IconPlus size={16} /> New profile</Button>}>
        <DataTable<QualityProfileFieldsFragment> className="px-4 pb-4" frame={false} columns={columns} rows={profiles} getRowId={(profile) => profile.id} isLoading={loading && profiles.length === 0} rowActions={actions} onRowClick={(profile) => setEditing(profile)} noun="profiles" emptyState={<EmptyState compact icon={IconTags} title="No profiles yet" />} />
      </Panel>

      <Dialog isOpen={editing !== null} onOpenChange={(open) => !open && setEditing(null)} title={editing === "new" || !editing ? "New quality profile" : `Edit ${editing.name}`} size="xl" footer={<><Button variant="ghost" onPress={() => setEditing(null)}>Cancel</Button><Button variant="primary" onPress={() => void submit()} isPending={creating || updating}>Save</Button></>}>
        <form onSubmit={submit} noValidate>
          <FieldGroup columns={2}>
            <FormTextField control={form.control} name="name" label="Name" isRequired autoFocus />
            <FormSelectField control={form.control} name="mediaKind" label="Media kind" options={[{ key: "VIDEO", label: "Video" }, { key: "AUDIO", label: "Audio" }]} />

            <GroupLabel>Quality</GroupLabel>
            {kind === "VIDEO" ? (
              <>
                <FormTextField control={form.control} name="allowedResolutions" label="Resolutions" description="Comma separated: 2160p, 1080p, 720p" />
                <FormTextField control={form.control} name="allowedVideoCodecs" label="Video codecs" description="hevc, h264, av1" />
                <FormOrderedChips
                  control={form.control}
                  name="resolutionPreference"
                  label="Resolution preference"
                  description="Best first. Without an order the highest allowed resolution wins."
                  options={resolutions.map((resolution) => ({ key: resolution, label: resolution }))}
                  placeholder="Add a resolution"
                  className="sm:col-span-2"
                />
                <FormTextField control={form.control} name="allowedHdrTypes" label="HDR types" description="hdr10, dolby_vision, hlg" />
                <FormSwitchField control={form.control} name="requireHdr" label="Require HDR" />
              </>
            ) : (
              <FormTextField control={form.control} name="allowedAudioFormats" label="Audio formats" description="flac, alac, mp3, aac" className="sm:col-span-2" />
            )}
            <FormTextField control={form.control} name="allowedSources" label="Sources" description="bluray, web, remux, hdtv" />
            <FormTextField control={form.control} name="cutoffResolution" label="Upgrade until" description="Stop looking for upgrades at this resolution" />
            <FormSwitchField control={form.control} name="upgradeUntilCutoff" label="Keep upgrading until the cutoff" className="sm:col-span-2" />

            <GroupLabel>Languages</GroupLabel>
            <FormOrderedChips
              control={form.control}
              name="preferredLanguages"
              label="Preferred languages"
              description="Most preferred first. Empty accepts any language."
              options={LANGUAGES.map((language) => ({ key: language.code, label: language.name }))}
              renderLabel={languageName}
              placeholder="Add a language"
              className="sm:col-span-2"
            />
            <FormSwitchField control={form.control} name="requireLanguageMatch" label="Only accept these languages" description="An untagged release counts as English." className="sm:col-span-2" />

            <GroupLabel>Release filters</GroupLabel>
            <FormNumberField control={form.control} name="minSizeMb" label="Minimum size (MB)" min={0} description="Per episode for season packs. Empty = no minimum." />
            <FormNumberField control={form.control} name="maxSizeMb" label="Maximum size (MB)" min={0} description="Per episode for season packs. Empty = no maximum." />
            <FormNumberField control={form.control} name="minSeeders" label="Minimum seeders" min={0} />
            <FormNumberField control={form.control} name="maxReleaseAgeDays" label="Maximum age (days)" min={0} description="Empty = any age." />
            <FormSwitchField control={form.control} name="allowSeasonPacks" label="Allow season packs" className="sm:col-span-2" />
            <FormSwitchField control={form.control} name="preferProperRepack" label="Prefer PROPER and REPACK releases" description="Also treats one as an upgrade over the file you already have." className="sm:col-span-2" />

            <GroupLabel>Release groups</GroupLabel>
            <FormOrderedChips
              control={form.control}
              name="preferredReleaseGroups"
              label="Preferred release groups"
              description="Ranking only, best first. Use the lists below to accept or reject outright."
              placeholder="Add a release group"
              className="sm:col-span-2"
            />
            <FormTextField control={form.control} name="releaseGroupWhitelist" label="Only these release groups" />
            <FormTextField control={form.control} name="releaseGroupBlacklist" label="Blocked release groups" />

            <FormSwitchField control={form.control} name="isDefault" label="Default for new libraries of this kind" className="sm:col-span-2" />
          </FieldGroup>
        </form>
      </Dialog>
      <ConfirmDialog isOpen={Boolean(removing)} onOpenChange={(open) => !open && setRemoving(null)} title={`Delete ${removing?.name}?`} description="Libraries using this profile fall back to the default." confirmLabel="Delete" destructive isPending={deleting} onConfirm={async () => { if (!removing) return; try { assertSuccess((await remove({ variables: { id: removing.id } })).data?.deleteQualityProfile, "Could not delete"); setRemoving(null); void refetch(); } catch (error) { toast.danger(errorMessage(error)); } }} />
    </div>
  );
}

/** Small heading that splits the profile form into readable groups. */
function GroupLabel({ children }: { children: ReactNode }) {
  return <p className="text-overline mt-2 text-muted sm:col-span-2">{children}</p>;
}
