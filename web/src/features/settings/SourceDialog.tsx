import { useMutation, useQuery } from "@apollo/client/react";
import { Description, FieldError, Input, Label, ListBox, ListBoxItem, Select, Switch, TextField, toast } from "@heroui/react";
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
        <Select aria-label="Source type" selectedKey={definitionId} onSelectionChange={(key) => setDefinitionId(key === null ? null : String(key))} isDisabled={Boolean(source)} isInvalid={Boolean(errors.definition)} fullWidth placeholder="Choose a source type">
          <Label>Type</Label>
          <Select.Trigger>
            <Select.Value />
            <Select.Indicator />
          </Select.Trigger>
          {definition ? <Description>{definition.description}</Description> : null}
          <FieldError>{errors.definition}</FieldError>
          <Select.Popover>
            <ListBox>
              {(definitions.data?.availableSourceDefinitions ?? []).map((item) => (
                <ListBoxItem key={item.id} id={item.id} textValue={item.name}>
                  <span className="flex flex-col">
                    <span>{item.name}</span>
                    <span className="text-label-sm text-muted">{item.trackerType} · {item.language}</span>
                  </span>
                </ListBoxItem>
              ))}
            </ListBox>
          </Select.Popover>
        </Select>
        <TextField value={name} onChange={setName} isInvalid={Boolean(errors.name)} fullWidth isRequired>
          <Label>Name</Label>
          <Input />
          <FieldError>{errors.name}</FieldError>
        </TextField>
        <TextField value={siteUrl} onChange={setSiteUrl} fullWidth type="url">
          <Label>Site URL</Label>
          <Input className="font-mono text-label" />
        </TextField>
        {requiredCredentials.map((key) => (
          <TextField key={key} value={credentials[key] ?? ""} onChange={(value) => setCredentials({ ...credentials, [key]: value })} isInvalid={Boolean(errors[key])} fullWidth type="password" autoComplete="off">
            <Label className="capitalize">{key.replace(/_/g, " ")}</Label>
            <Input className="font-mono text-label" placeholder={source ? "Leave empty to keep the stored value" : undefined} />
            <FieldError>{errors[key]}</FieldError>
          </TextField>
        ))}
        {(settingDefs.data?.sourceSettingDefinitions ?? []).map((setting) =>
          setting.options?.length ? (
            <Select key={setting.key} aria-label={setting.label} selectedKey={settings[setting.key] ?? setting.defaultValue ?? null} onSelectionChange={(key) => setSettings({ ...settings, [setting.key]: String(key) })} fullWidth>
              <Label>{setting.label}</Label>
              <Select.Trigger>
                <Select.Value />
                <Select.Indicator />
              </Select.Trigger>
              <Select.Popover>
                <ListBox>
                  {setting.options.map((option) => (
                    <ListBoxItem key={option.value} id={option.value} textValue={option.label}>
                      {option.label}
                    </ListBoxItem>
                  ))}
                </ListBox>
              </Select.Popover>
            </Select>
          ) : setting.settingType === "boolean" ? (
            <Switch key={setting.key} isSelected={(settings[setting.key] ?? setting.defaultValue) === "true"} onChange={(value) => setSettings({ ...settings, [setting.key]: String(value) })}>
              <Switch.Control>
                <Switch.Thumb />
              </Switch.Control>
              <Switch.Content>{setting.label}</Switch.Content>
            </Switch>
          ) : (
            <TextField key={setting.key} value={settings[setting.key] ?? setting.defaultValue ?? ""} onChange={(value) => setSettings({ ...settings, [setting.key]: value })} fullWidth>
              <Label>{setting.label}</Label>
              <Input className="font-mono text-label" />
            </TextField>
          ),
        )}
        <div>
          <p className="text-label text-foreground">Search for</p>
          <div className="mt-2 flex flex-wrap gap-2">
            {MEDIA_TYPES.map((type) => {
              const active = mediaTypes.includes(type.key);
              return (
                <Button key={type.key} size="sm" variant={active ? "primary" : "secondary"} onPress={() => setMediaTypes(active ? mediaTypes.filter((item) => item !== type.key) : [...mediaTypes, type.key])}>
                  {type.label}
                </Button>
              );
            })}
          </div>
        </div>
        <Switch isSelected={enabled} onChange={setEnabled}>
          <Switch.Control>
            <Switch.Thumb />
          </Switch.Control>
          <Switch.Content>Enabled</Switch.Content>
        </Switch>
      </FieldGroup>
    </Dialog>
  );
}
