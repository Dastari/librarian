import { useMutation } from "@apollo/client/react";
import { Field } from "glass-ux/field";
import { Input } from "glass-ux/input";
import { Segmented } from "glass-ux/segmented";
import { Select } from "glass-ux/select";
import { SettingRow } from "glass-ux/setting-row";
import { Switch } from "glass-ux/switch";
import { toast } from "glass-ux/toast";
import { UrlInput } from "glass-ux/url-field";
import { useEffect, useMemo, useState } from "react";

import { Button, Dialog, FieldGroup } from "@/components/ui";
import {
  AvailableSourceDefinitionsDocument,
  EntityLibraryListDocument,
  EntityRssFeedCreateDocument,
  EntityRssFeedUpdateDocument,
  EntitySourceCreateDocument,
  EntitySourceUpdateDocument,
  SourceSettingDefinitionsDocument,
  type EntityRssFeedListQuery,
  type EntitySourceListQuery,
} from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

type Source = EntitySourceListQuery["sources"]["edges"][number]["node"];
type Feed = EntityRssFeedListQuery["rssFeeds"]["edges"][number]["node"];

/** What the dialog edits: an indexer, an RSS feed, or (null) a new one of either. */
export type SourceTarget = { kind: "source"; source: Source } | { kind: "feed"; feed: Feed };

interface SourceDialogProps {
  target: SourceTarget | null;
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

/** How often a feed is read. The server never reads one faster than every 15 minutes. */
export const FEED_INTERVALS = [15, 30, 60] as const;
export const MIN_FEED_MINUTES = 15;

type Kind = "indexer" | "feed";

/** The two kinds of source, chosen first when adding one. */
function KindRow({ kind, onChange }: { kind: Kind; onChange: (kind: Kind) => void }) {
  return (
    <SettingRow
      label="Kind"
      description={kind === "feed" ? "A feed Librarian reads on a schedule. Every release it lists is kept as a catalogue of what is available." : "A tracker or indexer Librarian searches when it looks for a release."}
      control={<Segmented<Kind> aria-label="Kind of source" value={kind} onValueChange={onChange} options={[{ value: "indexer", label: "Indexer" }, { value: "feed", label: "RSS feed" }]} />}
    />
  );
}

/**
 * Add or edit a source: a torrent indexer, or an RSS feed. Credentials and feed addresses are sent
 * once, encrypted by the server and never read back.
 */
export function SourceDialog({ target, isOpen, onOpenChange, onSaved }: SourceDialogProps) {
  const [kind, setKind] = useState<Kind>(target?.kind === "feed" ? "feed" : "indexer");
  useEffect(() => {
    if (isOpen) setKind(target?.kind === "feed" ? "feed" : "indexer");
  }, [isOpen, target]);
  const adding = target === null;
  const kindRow = adding ? <KindRow kind={kind} onChange={setKind} /> : null;
  return kind === "feed" ? (
    <FeedDialog feed={target?.kind === "feed" ? target.feed : null} isOpen={isOpen} onOpenChange={onOpenChange} onSaved={onSaved} kindRow={kindRow} />
  ) : (
    <IndexerDialog source={target?.kind === "source" ? target.source : null} isOpen={isOpen} onOpenChange={onOpenChange} onSaved={onSaved} kindRow={kindRow} />
  );
}

// ---------------------------------------------------------------------------
// Indexers
// ---------------------------------------------------------------------------

interface IndexerDialogProps {
  source: Source | null;
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  onSaved?: () => void;
  kindRow: React.ReactNode;
}

type AuthMethod = "password" | "cookie";

/** IPTorrents signs in with a username and password (the default), or with a cookie from a browser. */
export function iptorrentsCredentials(method: AuthMethod, values: { username: string; password: string; cookie: string; userAgent: string }): Record<string, string> | null {
  if (method === "password") {
    return values.username.trim() && values.password ? { AuthMethod: "password", Username: values.username.trim(), Password: values.password } : null;
  }
  return values.cookie.trim() ? { AuthMethod: "cookie", Cookie: values.cookie.trim(), ...(values.userAgent.trim() ? { UserAgent: values.userAgent.trim() } : {}) } : null;
}

function IndexerDialog({ source, isOpen, onOpenChange, onSaved, kindRow }: IndexerDialogProps) {
  const definitions = useQuery(AvailableSourceDefinitionsDocument, { skip: !isOpen });
  const [definitionId, setDefinitionId] = useState<string | null>(source?.definitionId ?? null);
  const definition = definitions.data?.availableSourceDefinitions.find((item) => item.id === definitionId);
  const settingDefs = useQuery(SourceSettingDefinitionsDocument, { variables: { definitionId: definitionId ?? "" }, skip: !definitionId });

  const [name, setName] = useState(source?.name ?? "");
  const [siteUrl, setSiteUrl] = useState(source?.siteUrl ?? "");
  const [enabled, setEnabled] = useState(source?.enabled ?? true);
  const [mediaTypes, setMediaTypes] = useState<string[]>(source ? source.mediaTypes.split(",").filter(Boolean) : ["movies", "tv"]);
  const [credentials, setCredentials] = useState<Record<string, string>>({});
  const [method, setMethod] = useState<AuthMethod>("password");
  const [login, setLogin] = useState({ username: "", password: "", cookie: "", userAgent: "" });
  const [settings, setSettings] = useState<Record<string, string>>({});
  const [errors, setErrors] = useState<Record<string, string>>({});
  const isIptorrents = definitionId === "iptorrents";

  useEffect(() => {
    if (!isOpen) return;
    setDefinitionId(source?.definitionId ?? null);
    setName(source?.name ?? "");
    setSiteUrl(source?.siteUrl ?? "");
    setEnabled(source?.enabled ?? true);
    setMediaTypes(source ? source.mediaTypes.split(",").filter(Boolean) : ["movies", "tv"]);
    setCredentials({});
    setMethod("password");
    setLogin({ username: "", password: "", cookie: "", userAgent: "" });
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

  const requiredCredentials = useMemo(() => (isIptorrents ? [] : definition?.requiredCredentials ?? []), [definition, isIptorrents]);

  const save = async () => {
    const nextErrors: Record<string, string> = {};
    if (!definitionId) nextErrors.definition = "Choose a source type";
    if (!name.trim()) nextErrors.name = "Give the source a name";
    if (!source) for (const key of requiredCredentials) if (!credentials[key]?.trim()) nextErrors[key] = "Required";
    const iptorrents = isIptorrents ? iptorrentsCredentials(method, login) : null;
    if (isIptorrents && !source && !iptorrents) {
      if (method === "password") {
        if (!login.username.trim()) nextErrors.username = "Required";
        if (!login.password) nextErrors.password = "Required";
      } else nextErrors.cookie = "Required";
    }
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
    const sent = isIptorrents ? iptorrents : Object.values(credentials).some((value) => value.trim()) ? credentials : null;
    try {
      if (source) {
        assertSuccess((await updateSource({ variables: { id: source.id, input: { ...common, ...(sent ? { credentials: JSON.stringify(sent) } : {}) } } })).data?.updateSource, "Could not save");
        toast.success("Source saved");
      } else {
        assertSuccess(
          (await createSource({ variables: { input: { ...common, definitionId: definitionId!, sourceType: definition?.sourceType ?? "torrent_indexer", priority: 100, credentials: JSON.stringify(sent ?? {}), errorCount: 0 } } })).data?.createSource,
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
        {kindRow}
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
          <UrlInput value={siteUrl} onValueChange={setSiteUrl} className="font-mono" />
        </Field>
        {isIptorrents ? (
          <>
            <SettingRow
              label="Sign in with"
              description={method === "password" ? "Librarian signs in for you and signs in again when the session expires. It never tries more than once an hour, so the account cannot be locked." : "Paste the cookie from a signed-in browser. Use this if the site asks for a CAPTCHA, a Cloudflare check or a two-factor code."}
              control={<Segmented<AuthMethod> aria-label="How to sign in" value={method} onValueChange={setMethod} options={[{ value: "password", label: "Username and password" }, { value: "cookie", label: "Cookie (advanced)" }]} />}
            />
            {method === "password" ? (
              <>
                <Field label="Username" error={errors.username} required={!source}>
                  <Input value={login.username} onValueChange={(username) => setLogin({ ...login, username })} autoComplete="off" spellCheck={false} required={!source} placeholder={source ? "Leave empty to keep the stored login" : undefined} />
                </Field>
                <Field label="Password" error={errors.password} required={!source}>
                  <Input value={login.password} onValueChange={(password) => setLogin({ ...login, password })} type="password" autoComplete="off" spellCheck={false} required={!source} placeholder={source ? "Leave empty to keep the stored login" : undefined} />
                </Field>
              </>
            ) : (
              <>
                <Field label="Cookie" error={errors.cookie} hint="Stored encrypted and never shown again." required={!source}>
                  <Input value={login.cookie} onValueChange={(cookie) => setLogin({ ...login, cookie })} type="password" autoComplete="off" spellCheck={false} required={!source} placeholder={source ? "Leave empty to keep the stored cookie" : undefined} className="font-mono" />
                </Field>
                <Field label="User agent" hint="The browser's user agent, so the site sees the same browser.">
                  <Input value={login.userAgent} onValueChange={(userAgent) => setLogin({ ...login, userAgent })} autoComplete="off" spellCheck={false} className="font-mono" />
                </Field>
              </>
            )}
          </>
        ) : null}
        {requiredCredentials.map((key) => (
          <Field key={key} label={key.replace(/_/g, " ")} error={errors[key]} required={!source}>
            <Input value={credentials[key] ?? ""} onValueChange={(value) => setCredentials({ ...credentials, [key]: value })} type="password" autoComplete="off" spellCheck={false} required={!source} placeholder={source ? "Leave empty to keep the stored value" : undefined} className="font-mono" />
          </Field>
        ))}
        {(settingDefs.data?.sourceSettingDefinitions ?? []).map((setting) =>
          setting.options?.length ? (
            <Field key={setting.key} label={setting.label} nativeLabel={false}>
              <Select value={settings[setting.key] ?? setting.defaultValue ?? null} onValueChange={(value) => value !== null && setSettings({ ...settings, [setting.key]: value })} options={setting.options.map((option) => ({ value: option.value, label: option.label }))} />
            </Field>
          ) : setting.settingType === "Checkbox" || setting.settingType === "boolean" ? (
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

// ---------------------------------------------------------------------------
// RSS feeds
// ---------------------------------------------------------------------------

interface FeedDialogProps {
  feed: Feed | null;
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  onSaved?: () => void;
  kindRow: React.ReactNode;
}

/** The intervals offered: the three the app uses, and the feed's own if it is something else. */
export function feedIntervalOptions(current: number | undefined): number[] {
  return current && !(FEED_INTERVALS as readonly number[]).includes(current) ? [...FEED_INTERVALS, current].sort((a, b) => a - b) : [...FEED_INTERVALS];
}

/** Add or edit a feed: name, address, how often it is read, an optional library, and whether it is on. */
function FeedDialog({ feed, isOpen, onOpenChange, onSaved, kindRow }: FeedDialogProps) {
  const { user } = useSession();
  const libraries = useQuery(EntityLibraryListDocument, { variables: { orderBy: [{ name: "ASC" }] }, skip: !isOpen });
  const [name, setName] = useState("");
  const [url, setUrl] = useState("");
  const [minutes, setMinutes] = useState<number>(30);
  const [libraryId, setLibraryId] = useState<string>("");
  const [enabled, setEnabled] = useState(true);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [create, { loading: creating }] = useMutation(EntityRssFeedCreateDocument);
  const [update, { loading: updating }] = useMutation(EntityRssFeedUpdateDocument);

  useEffect(() => {
    if (!isOpen) return;
    setName(feed?.name ?? "");
    setUrl("");
    setMinutes(feed?.pollIntervalMinutes ?? 30);
    setLibraryId(feed?.libraryId ?? "");
    setEnabled(feed?.enabled ?? true);
    setErrors({});
  }, [isOpen, feed]);

  const save = async () => {
    const next: Record<string, string> = {};
    if (!name.trim()) next.name = "Give the feed a name";
    // The address of a saved feed is never read back, so editing one leaves it unchanged when empty.
    if ((!feed || url.trim()) && !/^https?:\/\/\S+$/i.test(url.trim())) next.url = "Enter an http or https address";
    if (!Number.isInteger(minutes) || minutes < MIN_FEED_MINUTES) next.minutes = `At least ${MIN_FEED_MINUTES} minutes`;
    setErrors(next);
    if (Object.keys(next).length) return;
    const common = { name: name.trim(), pollIntervalMinutes: minutes, enabled, libraryId: libraryId || null };
    try {
      if (feed) {
        assertSuccess((await update({ variables: { id: feed.id, input: { ...common, ...(url.trim() ? { url: url.trim() } : {}) } } })).data?.updateRssFeed, "Could not save the feed");
        toast.success("Feed saved");
      } else {
        if (!user) throw new Error("Sign in to add a feed");
        assertSuccess((await create({ variables: { input: { ...common, url: url.trim(), userId: user.id } } })).data?.createRssFeed, "Could not add the feed");
        toast.success("Feed added");
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
      title={feed ? `Edit ${feed.name}` : "Add source"}
      size="md"
      footer={
        <>
          <Button variant="ghost" onPress={() => onOpenChange(false)}>Cancel</Button>
          <Button variant="primary" onPress={() => void save()} isPending={creating || updating}>{feed ? "Save" : "Add"}</Button>
        </>
      }
    >
      <FieldGroup>
        {kindRow}
        <Field label="Name" error={errors.name} required>
          <Input value={name} onValueChange={setName} required />
        </Field>
        <Field
          label="Feed address"
          error={errors.url}
          hint={feed ? `Currently ${feed.urlDisplay ?? "set"}. Leave empty to keep it; the address carries your tracker key, so it is stored encrypted and never shown.` : "An RSS 2.0 feed of releases. Torznab feeds work too. The address carries your tracker key, so it is stored encrypted and never shown."}
          required={!feed}
        >
          <UrlInput value={url} onValueChange={setUrl} openable={false} className="font-mono" required={!feed} placeholder={feed ? "Leave empty to keep the stored address" : "https://"} />
        </Field>
        <SettingRow
          label="Read every"
          description={errors.minutes ?? `Never faster than every ${MIN_FEED_MINUTES} minutes, and slower while the feed keeps failing.`}
          control={<Segmented<string> aria-label="Read the feed every" value={String(minutes)} onValueChange={(value) => setMinutes(Number(value))} options={feedIntervalOptions(feed?.pollIntervalMinutes).map((value) => ({ value: String(value), label: `${value} min` }))} />}
        />
        <Field label="Library" hint="Wanted titles from this feed are only grabbed for this library. Leave on all to use every library you own." nativeLabel={false}>
          <Select
            value={libraryId}
            onValueChange={(value) => setLibraryId(value ?? "")}
            options={[{ value: "", label: "All my libraries" }, ...(libraries.data?.libraries.edges.map((edge) => ({ value: edge.node.id, label: edge.node.name })) ?? [])]}
          />
        </Field>
        <SettingRow label="Enabled" description="A disabled feed is not read." control={<Switch checked={enabled} onCheckedChange={setEnabled} />} />
      </FieldGroup>
    </Dialog>
  );
}
