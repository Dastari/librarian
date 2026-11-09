import { useMutation } from "@apollo/client/react";
import { Button as GlassButton } from "glass-ux/button";
import { Field } from "glass-ux/field";
import { useBoundField } from "glass-ux/form";
import { IconFolder, IconPlus } from "glass-ux/icons";
import { Input } from "glass-ux/input";
import { Select } from "glass-ux/select";
import { Tooltip } from "glass-ux/tooltip";
import { useEffect, useMemo, useState } from "react";

import { Button } from "@/components/ui";
import { GlassSegmented } from "@/components/ui/GlassControls";
import { FolderPickerDialog } from "@/features/files/FolderPickerDialog";
import { FolderField } from "@/features/libraries/FolderField";
import { composeShareAddress, isShareAddress, parseShareAddress, splitHostPort } from "@/features/libraries/shareLocation";
import {
  EntityNetworkServerListDocument,
  EntityNetworkServerUpdateDocument,
  NetworkServerSharesDocument,
  type NetworkServerFieldsFragment,
} from "@/graphql/generated/graphql";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

import { AddNetworkServerDialog } from "./NetworkServerDialogs";
import { serverAddress, signInLabel } from "./NetworkServerIdentity";

type Mode = "local" | "share";
type Server = NetworkServerFieldsFragment;

const ADD = "__add__";
const OTHER = "__other__";

/** The saved server a `host[:port]` belongs to. */
export function serverFor(servers: readonly Server[], host: string): Server | null {
  const { host: name, port } = splitHostPort(host);
  const wanted = name.trim().toLowerCase();
  return servers.find((server) => server.host === wanted && (server.port ?? 445) === (port ?? 445)) ?? null;
}

/**
 * Where files are: a folder on this server (the default), or a folder on a share of a saved file
 * server, which Librarian reads itself with no mount. The server is chosen from the saved ones
 * (or added on the spot with "Add network share…"), the share from what the server offers, and
 * the folder typed or browsed. Goes in a form's `path` field; the field keeps the one address the
 * server stores (`smb://host[:port]/share/folder`), and the sign-in is the saved server's.
 */
export function StorageLocationField({ required, localPlaceholder, folderLabel = "Folder in the share", allowWholeShare = true }: {
  required?: boolean;
  localPlaceholder?: string;
  folderLabel?: string;
  /** Whether a blank folder (the share itself) is allowed. */
  allowWholeShare?: boolean;
}) {
  const { field, root, error } = useBoundField<string>();
  const value = field.state.value;
  const parsed = parseShareAddress(value);
  const [mode, setMode] = useState<Mode>(isShareAddress(value) ? "share" : "local");
  const servers = useQuery(EntityNetworkServerListDocument, { variables: { orderBy: [{ name: "ASC" }] }, skip: mode !== "share" });
  const saved = useMemo(() => (servers.data?.networkServers.edges ?? []).map(({ node }) => node), [servers.data]);
  const [serverId, setServerId] = useState<string | null>(null);
  const [share, setShare] = useState(parsed?.share ?? "");
  const [typingShare, setTypingShare] = useState(false);
  const [folder, setFolder] = useState(parsed?.folder ?? "");
  const [adding, setAdding] = useState(false);
  const [browsing, setBrowsing] = useState(false);
  const [remember] = useMutation(EntityNetworkServerUpdateDocument);

  // A saved address names its server by host: choose that server once the list is in.
  useEffect(() => {
    if (serverId || !parsed?.host) return;
    const found = serverFor(saved, parsed.host);
    if (found) setServerId(found.id);
  }, [saved]); // eslint-disable-line react-hooks/exhaustive-deps

  const server = saved.find((item) => item.id === serverId) ?? null;
  const shares = useQuery(NetworkServerSharesDocument, { variables: { id: serverId ?? "" }, skip: !serverId });
  const shareNames = useMemo(() => {
    const listed = shares.data?.networkServerShares.shares.map((item) => item.name) ?? server?.knownShares ?? [];
    return share && !listed.some((name) => name.toLowerCase() === share.toLowerCase()) ? [...listed, share] : listed;
  }, [shares.data, server, share]);

  const hostOf = (item: Server | null) => (item ? serverAddress(item) : parsed?.host ?? "");
  const publish = (next: { server?: Server | null; share?: string; folder?: string }) => {
    const host = hostOf(next.server === undefined ? server : next.server);
    field.handleChange(composeShareAddress({ host, share: next.share ?? share, folder: next.folder ?? folder }));
  };

  const changeMode = (next: Mode) => {
    if (next === mode) return;
    setMode(next);
    field.handleChange("");
  };
  const chooseServer = (id: string | null) => {
    if (id === ADD) {
      setAdding(true);
      return;
    }
    const next = saved.find((item) => item.id === id) ?? null;
    setServerId(next?.id ?? null);
    setShare("");
    setTypingShare(false);
    publish({ server: next, share: "" });
  };
  const chooseShare = (name: string | null) => {
    if (name === OTHER) {
      setTypingShare(true);
      setShare("");
      publish({ share: "" });
      return;
    }
    setTypingShare(false);
    setShare(name ?? "");
    publish({ share: name ?? "" });
  };
  // A share typed by hand is remembered on the server, so it is offered next time.
  const rememberTyped = () => {
    const name = share.trim().toLowerCase();
    if (!server || !name || server.knownShares.includes(name)) return;
    void remember({ variables: { id: server.id, input: { knownShares: [...server.knownShares, name] } } }).catch(() => undefined);
  };

  const serverOptions = [
    ...saved.map((item) => ({ value: item.id, label: item.name, description: `${serverAddress(item)} · ${signInLabel(item)}` })),
    { value: ADD, label: "Add network share…", icon: <IconPlus /> },
  ];
  const shareOptions = [
    ...shareNames.map((name) => ({ value: name, label: name })),
    { value: OTHER, label: "Another share…" },
  ];
  const unsavedHost = parsed?.host && !server && servers.data ? parsed.host : null;

  return (
    <div className="flex min-w-0 flex-col gap-3">
      <div className="flex flex-col gap-1.5">
        <p className="text-label-sm text-muted">Location</p>
        <GlassSegmented
          ariaLabel="Where the files are"
          value={mode}
          onChange={changeMode}
          items={[{ key: "local", label: "This server" }, { key: "share", label: "Network share" }]}
        />
      </div>

      {mode === "local" ? <FolderField label="Folder" placeholder={localPlaceholder} required={required} /> : (
        <>
          {saved.length === 0 && servers.data ? (
            <div className="flex min-w-0 flex-wrap items-center gap-3">
              <p className="min-w-0 flex-1 text-body-sm text-muted">No file server is saved yet. Save one with its sign-in; it is offered here from then on.</p>
              <Button variant="secondary" size="sm" onPress={() => setAdding(true)}><IconPlus /> Add network share…</Button>
            </div>
          ) : (
            <div className="@container min-w-0">
              <div className="grid min-w-0 grid-cols-1 gap-3 @min-[32rem]:grid-cols-2">
                <Field label="Server" required hint={unsavedHost ? `${unsavedHost} is not saved. Add it to sign in.` : server ? signInLabel(server) : undefined}>
                  <Select<string> value={serverId} onValueChange={chooseServer} options={serverOptions} placeholder="Choose a server" aria-label="Server" />
                </Field>
                {typingShare ? (
                  <Field label="Share" required hint="Remembered on the server once used.">
                    <Input value={share} onValueChange={(next) => { setShare(next); publish({ share: next }); }} onBlur={rememberTyped} placeholder="media" spellCheck={false} className="font-mono" />
                  </Field>
                ) : (
                  <Field label="Share" required hint={shares.data && !shares.data.networkServerShares.success ? `Not listed now: ${shares.data.networkServerShares.error ?? "the server did not answer"}` : undefined}>
                    <Select<string> value={share || null} onValueChange={chooseShare} options={shareOptions} placeholder={serverId ? "Choose a share" : "Choose a server first"} disabled={!serverId} aria-label="Share" />
                  </Field>
                )}
              </div>
            </div>
          )}
          <Field {...root} label={folderLabel} hint={allowWholeShare ? "Leave blank to use the whole share." : undefined} required={required}>
            <Input
              value={folder}
              onValueChange={(next) => { setFolder(next); publish({ folder: next }); }}
              onBlur={field.handleBlur}
              placeholder="Movies"
              spellCheck={false}
              className="font-mono"
              aria-invalid={error ? true : undefined}
              trailing={(
                <Tooltip content={server && share ? "Browse the share" : "Choose the server and share first"}>
                  <GlassButton variant="ghost" size="xs" icon={<IconFolder />} aria-label="Browse the share" disabled={!server || !share} onClick={() => setBrowsing(true)} />
                </Tooltip>
              )}
            />
          </Field>
          <FolderPickerDialog
            isOpen={browsing}
            onOpenChange={setBrowsing}
            initialPath={composeShareAddress({ host: hostOf(server), share, folder }) || undefined}
            onPick={(path) => {
              const picked = parseShareAddress(path);
              if (!picked) return;
              setFolder(picked.folder);
              publish({ folder: picked.folder });
            }}
          />
          <AddNetworkServerDialog
            open={adding}
            onOpenChange={setAdding}
            onCreated={(created) => {
              void servers.refetch();
              setServerId(created.id);
              const first = created.knownShares[0] ?? "";
              setShare(first);
              setTypingShare(false);
              publish({ server: created, share: first });
            }}
          />
        </>
      )}
    </div>
  );
}
