import { useMutation } from "@apollo/client/react";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { Field } from "glass-ux/field";
import { Input } from "glass-ux/input";
import { Select } from "glass-ux/select";
import { toast } from "glass-ux/toast";
import { SettingRow } from "glass-ux/setting-row";
import { Switch } from "glass-ux/switch";
import { useEffect, useMemo, useState } from "react";

import { Button, Dialog, FieldGroup } from "@/components/ui";
import {
  AvailableSourceDefinitionsDocument,
  EntitySourceCreateDocument,
  EntitySourceUpdateDocument,
  SourceSettingDefinitionsDocument,
  type EntitySourceListQuery,
} from "@/graphql/generated/graphql";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

type Source = EntitySourceListQuery["sources"]["edges"][number]["node"];

interface SourceDialogProps {
  source: Source | null;
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  onSaved?: () => void;
}

const MEDIA_TYPES = [
  { key: "movies", label: "Movies" },
  { key: "tv", label: "TV" },
  { key: "music", label: "Music" },
  { key: "audiobooks", label: "Audiobooks" },
];

/**
 * Add or edit a source. Definitions drive which credentials and settings appear; credentials are
 * sent once, encrypted server-side and never read back.
 */
export function SourceDialog({ source, isOpen, onOpenChange, onSaved }: SourceDialogProps) {
  const definitions = useQuery(AvailableSourceDefinitionsDocument, { skip: !isOpen });
  const [definitionId, setDefinitionId] = useState<string | null>(source?.definitionId ?? null);
  const definition = definitions.data?.availableSourceDefinitions.find((item) => item.id === definitionId);
  const settingDefs = useQuery(SourceSettingDefinitionsDocument, { variables: { definitionId: definitionId ?? "" }, skip: !definitionId });

  const [name, setName] = useState(source?.name ?? "");
  const [siteUrl, setSiteUrl] = useState(source?.siteUrl ?? "");
  const [enabled, setEnabled] = useState(source?.enabled ?? true);
  const [mediaTypes, setMediaTypes] = useState<string[]>(source ? source.mediaTypes.split(",").filter(Boolean) : ["movies", "tv"]);
  const [credentials, setCredentials] = useState<Record<string, string>>({});
  const [settings, setSettings] = useState<Record<string, string>>({});
  const [errors, setErrors] = useState<Record<string, string>>({});

  useEffect(() => {
    if (!isOpen) return;
    setDefinitionId(source?.definitionId ?? null);
    setName(source?.name ?? "");
    setSiteUrl(source?.siteUrl ?? "");
    setEnabled(source?.enabled ?? true);
    setMediaTypes(source ? source.mediaTypes.split(",").filter(Boolean) : ["movies", "tv"]);
    setCredentials({});
    setErrors({});
    try {
      setSettings(source?.settings ? (JSON.parse(source.settings) as Record<string, string>) : {});
    } catch {
      setSettings({});
    }
  }, [isOpen, source]);

  useEffect(() => {
    if (definition && !source) {
      if (!name) setName(definition.name);
      if (!siteUrl) setSiteUrl(definition.siteLink);
    }
  }, [definition, source, name, siteUrl]);

  const [createSource, { loading: creating }] = useMutation(EntitySourceCreateDocument);
  const [updateSource, { loading: updating }] = useMutation(EntitySourceUpdateDocument);

  const requiredCredentials = useMemo(() => definition?.requiredCredentials ?? [], [definition]);

  const save = async () => {
    const nextErrors: Record<string, string> = {};
    if (!definitionId) nextErrors.definition = "Choose a source type";
    if (!name.trim()) nextErrors.name = "Give the source a name";
    if (!source) for (const key of requiredCredentials) if (!credentials[key]?.trim()) nextErrors[key] = "Required";
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length) return;

    const common = {
      name: name.trim(),
      siteUrl: siteUrl.trim() || null,
      enabled,
      mediaTypes: mediaTypes.join(","),
      settings: Object.keys(settings).length ? JSON.stringify(settings) : null,
      supportsSearch: true,
      supportsMovieSearch: mediaTypes.includes("movies"),
      supportsTvSearch: mediaTypes.includes("tv"),
      supportsMusicSearch: mediaTypes.includes("music"),
      supportsBookSearch: mediaTypes.includes("audiobooks"),
    };
    try {
      if (source) {
        const hasNewCredentials = Object.values(credentials).some((value) => value.trim());
        assertSuccess((await updateSource({ variables: { id: source.id, input: { ...common, ...(hasNewCredentials ? { credentials: JSON.stringify(credentials) } : {}) } } })).data?.updateSource, "Could not save");
        toast.success("Source saved");
      } else {
        assertSuccess(
          (await createSource({ variables: { input: { ...common, definitionId: definitionId!, sourceType: definition?.sourceType ?? "torrent_indexer", priority: 100, credentials: JSON.stringify(credentials), errorCount: 0 } } })).data?.createSource,
          "Could not add the source",
        );
        toast.success("Source added");
      }
      onSaved?.();
      onOpenChange(false);
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      title={source ? `Edit ${source.name}` : "Add source"}
      size="md"
      footer={
        <>
          <Button variant="ghost" onPress={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button variant="primary" onPress={() => void save()} isPending={creating || updating}>
            {source ? "Save" : "Add"}
          </Button>
        </>
      }
    >
      <FieldGroup>
        <Field label="Type" hint={definition?.description} error={errors.definition} disabled={Boolean(source)} nativeLabel={false}>
          <Select
            value={definitionId}
            onValueChange={setDefinitionId}
            disabled={Boolean(source)}
            placeholder="Choose a source type"
            options={(definitions.data?.availableSourceDefinitions ?? []).map((item) => ({ value: item.id, label: item.name, detail: [item.trackerType, item.language].filter(Boolean).join(" · ") }))}
          />
        </Field>
        <Field label="Name" error={errors.name} required>
          <Input value={name} onValueChange={setName} required />
        </Field>
        <Field label="Site URL">
          <Input value={siteUrl} onValueChange={setSiteUrl} type="url" className="font-mono" />
        </Field>
        {requiredCredentials.map((key) => (
          <Field key={key} label={key.replace(/_/g, " ")} error={errors[key]} required={!source}>
            <Input value={credentials[key] ?? ""} onValueChange={(value) => setCredentials({ ...credentials, [key]: value })} type="password" autoComplete="off" required={!source} placeholder={source ? "Leave empty to keep the stored value" : undefined} className="font-mono" />
          </Field>
        ))}
        {(settingDefs.data?.sourceSettingDefinitions ?? []).map((setting) =>
          setting.options?.length ? (
            <Field key={setting.key} label={setting.label} nativeLabel={false}>
              <Select value={settings[setting.key] ?? setting.defaultValue ?? null} onValueChange={(value) => value !== null && setSettings({ ...settings, [setting.key]: value })} options={setting.options.map((option) => ({ value: option.value, label: option.label }))} />
            </Field>
          ) : setting.settingType === "boolean" ? (
            <SettingRow key={setting.key} label={setting.label} control={<Switch checked={(settings[setting.key] ?? setting.defaultValue) === "true"} onCheckedChange={(value) => setSettings({ ...settings, [setting.key]: String(value) })} />} />
          ) : (
            <Field key={setting.key} label={setting.label}>
              <Input value={settings[setting.key] ?? setting.defaultValue ?? ""} onValueChange={(value) => setSettings({ ...settings, [setting.key]: value })} className="font-mono" />
            </Field>
          ),
        )}
        <div>
          <p className="text-label text-foreground">Search for</p>
          <div className="mt-2 flex flex-wrap gap-2">
            {MEDIA_TYPES.map((type) => {
              const active = mediaTypes.includes(type.key);
              return (
                <Button key={type.key} aria-pressed={active} size="sm" variant={active ? "primary" : "secondary"} onPress={() => setMediaTypes(active ? mediaTypes.filter((item) => item !== type.key) : [...mediaTypes, type.key])}>
                  {type.label}
                </Button>
              );
            })}
          </div>
        </div>
        <SettingRow label="Enabled" control={<Switch checked={enabled} onCheckedChange={setEnabled} />} />
      </FieldGroup>
    </Dialog>
  );
}
