import { useMutation } from "@apollo/client/react";
import { toast } from "glass-ux/toast";
import { zodResolver } from "@hookform/resolvers/zod";
import { defineActions } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, DataTable, filters, IdentityCell, TextCell } from "glass-ux/data-table";
import { IconEdit, IconPlus, IconStar, IconTrash } from "glass-ux/icons";
import { IconTags } from "@tabler/icons-react";
import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { Button, Dialog, FieldGroup, FormNumberField, FormSelectField, FormSwitchField, FormTextField } from "@/components/ui";
import {
  EntityQualityProfileCreateDocument,
  EntityQualityProfileDeleteDocument,
  EntityQualityProfileUpdateDocument,
  EntityQualityProfileListDocument,
  type QualityProfileFieldsFragment,
} from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { glyph } from "@/lib/entities/kinds";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { LANGUAGES, languageName } from "@/lib/languages";

import { FormOrderedChips } from "./OrderedChips";

import { SettingsLoadError } from "./SettingsLoadError";
import { SettingsSection } from "./SettingsSection";
import { useSettingsConnection } from "./useSettingsConnection";

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

type Profile = QualityProfileFieldsFragment;
const ProfileGlyph = glyph(IconTags, "tags");
const column = createDataTableColumns<Profile>();
const columns = column.columns([
  column.accessor("name", {
    header: "Profile", width: 26, priority: 1, cardRole: "title", sortable: true,
    cell: ({ row }) => { const profile = row.original; return <IdentityCell icon={<ProfileGlyph />} title={profile.name} subtitle={profile.mediaKind === "VIDEO" ? [profile.allowedResolutions.join("/"), profile.allowedVideoCodecs.join("/"), profile.requireHdr ? "HDR required" : null].filter(Boolean).join(" · ") : profile.allowedAudioFormats.join("/")} />; },
  }),
  column.accessor("mediaKind", {
    header: "Kind", width: 7, priority: 2, sortable: true,
    filter: filters.options({ field: "mediaKind", nullable: false, operators: ["in"], options: [{ value: "VIDEO", label: "Video" }, { value: "AUDIO", label: "Audio" }] }),
    cell: ({ row }) => <Badge size="sm" tone="info">{row.original.mediaKind === "VIDEO" ? "Video" : "Audio"}</Badge>,
  }),
  column.display({ id: "default", header: "Use", width: 7, priority: 2, cardRole: "status", cell: ({ row }) => (row.original.isDefault ? <Badge size="sm" marker="dot" tone="accent">Default</Badge> : <TextCell muted>—</TextCell>) }),
  column.display({ id: "cutoff", header: "Upgrade until", width: 9, priority: 3, cell: ({ row }) => <TextCell muted>{row.original.upgradeUntilCutoff ? (row.original.cutoffResolution ?? "Best available") : "Off"}</TextCell> }),
  column.display({ id: "sources", header: "Sources", width: 12, priority: 4, cell: ({ row }) => <TextCell muted>{row.original.allowedSources.join(", ") || "Any"}</TextCell> }),
]);

export function QualitySettings() {
  const { user } = useSession();
  const list = useSettingsConnection(EntityQualityProfileListDocument, { orderBy: [{ name: "ASC" }] }, (data) => data.qualityProfiles);
  const { rows: profiles, loading, refetch } = list;
  const [editing, setEditing] = useState<QualityProfileFieldsFragment | null | "new">(null);
  const [create, { loading: creating }] = useMutation(EntityQualityProfileCreateDocument);
  const [update, { loading: updating }] = useMutation(EntityQualityProfileUpdateDocument);
  const [remove] = useMutation(EntityQualityProfileDeleteDocument);
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

  const actions = useMemo(() => defineActions<Profile>("quality-profile", [
    { id: "edit", label: "Edit", icon: IconEdit, group: "open", priority: 1, visible: (targets) => targets.length === 1, run: (targets) => setEditing(targets[0]!) },
    { id: "default", label: "Make default", icon: IconStar, group: "manage", priority: 2, visible: (targets) => targets.length === 1 && !targets[0]!.isDefault, run: (targets) => makeDefault(targets[0]!) },
    {
      id: "delete", label: "Delete", icon: IconTrash, group: "danger",
      visible: (targets) => targets.length === 1 && !targets[0]!.isDefault,
      confirm: (targets) => ({ title: `Delete ${targets[0]!.name}?`, body: "Libraries using this profile fall back to the default.", confirmLabel: "Delete", tone: "danger" }),
      run: async (targets) => { assertSuccess((await remove({ variables: { id: targets[0]!.id } })).data?.deleteQualityProfile, "Could not delete"); void refetch(); },
      successMessage: () => "Profile deleted",
    },
  ]), [makeDefault, refetch, remove]);

  const kind = form.watch("mediaKind");
  const resolutions = splitList(form.watch("allowedResolutions"));

  return (
    <div className="flex flex-col gap-6">
      <SettingsSection title="Quality profiles" description="Decide which releases are acceptable and when a better file should replace an existing one." actions={<Button variant="primary" size="sm" onPress={() => setEditing("new")}><IconPlus /> New profile</Button>}>
        <SettingsLoadError error={list.error} retry={list.refetch} hasRows={profiles.length > 0} />
        <DataTable<Profile> id={`librarian.${user?.id ?? "session"}.settings.quality-profiles`} label="Quality profiles" columns={columns} rows={profiles} getRowId={(profile) => profile.id} getRowLabel={(profile) => profile.name} loading={loading && profiles.length === 0} actions={actions} onRowOpen={(profile) => setEditing(profile)} rowOpen="click" noun={{ one: "profile", other: "profiles" }} emptyMessage="No profiles yet" />
      </SettingsSection>

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
    </div>
  );
}

/** Small heading that splits the profile form into readable groups. */
function GroupLabel({ children }: { children: ReactNode }) {
  return <p className="text-overline mt-2 text-muted sm:col-span-2">{children}</p>;
}
