import { useApolloClient, useMutation } from "@apollo/client/react";
import { Button as GlassButton } from "glass-ux/button";
import { Field } from "glass-ux/field";
import { useBoundField } from "glass-ux/form";
import { IconFolder } from "glass-ux/icons";
import { Input } from "glass-ux/input";
import { PasswordInput } from "glass-ux/password-field";
import { Tooltip } from "glass-ux/tooltip";
import { useEffect, useRef, useState } from "react";

import { Button } from "@/components/ui";
import { GlassSegmented } from "@/components/ui/GlassControls";
import { FolderPickerDialog } from "@/features/files/FolderPickerDialog";
import { ConnectShareDocument, ShareSignInDocument } from "@/graphql/generated/graphql";
import { errorMessage } from "@/lib/graphql/errors";

import { FolderField } from "./FolderField";
import { composeShareAddress, isShareAddress, parseShareAddress, shareRootAddress, splitHostPort } from "./shareLocation";

type Mode = "local" | "share";

/** Whether the share can be browsed: it answered a connection test, or a sign-in is saved for it. */
type Link = { state: "idle" } | { state: "testing" } | { state: "connected"; folders: number | null } | { state: "failed"; message: string };

/**
 * Where a library's files are: a folder on the server (the default), or a folder on a network
 * share Librarian reads itself, with no mount. A share takes the server, the share name and a
 * sign-in (saved once for the share, encrypted), and its folder is browsed over the share. Goes in
 * a form's `path` field; the field keeps the one address the server stores.
 */
export function LibraryLocationField({ required, localPlaceholder }: { required?: boolean; localPlaceholder?: string }) {
  const { field, root, error } = useBoundField<string>();
  const value = field.state.value;
  const parsed = parseShareAddress(value);
  const [mode, setMode] = useState<Mode>(isShareAddress(value) ? "share" : "local");
  const [host, setHost] = useState(parsed?.host ?? "");
  const [share, setShare] = useState(parsed?.share ?? "");
  const [folder, setFolder] = useState(parsed?.folder ?? "");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [savedUser, setSavedUser] = useState<string | null>(null);
  const [link, setLink] = useState<Link>({ state: "idle" });
  const [browsing, setBrowsing] = useState(false);
  const client = useApolloClient();
  const [connect] = useMutation(ConnectShareDocument);

  // The address follows the parts: change a box and the stored value changes with it.
  const publish = (next: { host: string; share: string; folder: string }) => field.handleChange(composeShareAddress(next));

  // A sign-in already saved for this share: show whose it is, and let it be used as it is.
  const rootAddress = host.trim() && share.trim() ? shareRootAddress({ host, share }) : null;
  const lookedUp = useRef<string | null>(null);
  const latestRoot = useRef(rootAddress);
  latestRoot.current = rootAddress;
  useEffect(() => {
    if (mode !== "share" || !rootAddress || lookedUp.current === rootAddress) return;
    lookedUp.current = rootAddress;
    // An answer for a share that is no longer the one typed is ignored.
    const stillTyped = () => latestRoot.current === rootAddress;
    void client
      .query({ query: ShareSignInDocument, variables: { share: rootAddress }, fetchPolicy: "network-only" })
      .then(({ data }) => {
        if (!stillTyped()) return;
        if (!data?.shareSignIn.saved) {
          setSavedUser(null);
          return;
        }
        setSavedUser(data.shareSignIn.username ?? "");
        setUsername((typed) => typed || data.shareSignIn.username || "");
        // A share with a saved sign-in is known to have worked once.
        setLink((state) => (state.state === "idle" ? { state: "connected", folders: null } : state));
      })
      .catch(() => { if (stillTyped()) setSavedUser(null); });
  }, [client, mode, rootAddress]);

  const changeMode = (next: Mode) => {
    if (next === mode) return;
    setMode(next);
    field.handleChange("");
    setLink({ state: "idle" });
  };

  const testConnection = async () => {
    const { host: name, port } = splitHostPort(host);
    setLink({ state: "testing" });
    try {
      const { data } = await connect({ variables: { input: { host: name, port, share: share.trim(), username: username.trim(), password, save: true } } });
      const result = data?.connectShare;
      if (!result?.success) {
        setLink({ state: "failed", message: result?.error ?? "The connection failed" });
        return;
      }
      setLink({ state: "connected", folders: result.folders });
      setSavedUser(username.trim() || savedUser || "");
      setPassword("");
      const normalised = result.share ? parseShareAddress(result.share) : null;
      if (normalised) {
        setHost(normalised.host);
        setShare(normalised.share);
        publish({ ...normalised, folder });
      }
    } catch (failure) {
      setLink({ state: "failed", message: errorMessage(failure) });
    }
  };

  const connected = link.state === "connected";
  const canTest = host.trim() !== "" && share.trim() !== "" && link.state !== "testing";

  return (
    <div className="flex min-w-0 flex-col gap-3">
      <div className="flex flex-col gap-1.5">
        <p className="text-label-sm text-muted">Location</p>
        <GlassSegmented
          ariaLabel="Where the library's files are"
          value={mode}
          onChange={changeMode}
          items={[{ key: "local", label: "This server" }, { key: "share", label: "Network share" }]}
        />
      </div>

      {mode === "local" ? <FolderField label="Folder" placeholder={localPlaceholder} required={required} /> : (
        <>
          <p className="text-body-sm text-muted">
            Librarian reads the share itself, so nothing has to be mounted. Use the server&rsquo;s IP address if its name doesn&rsquo;t resolve from here.
          </p>
          <div className="@container min-w-0">
            <div className="grid min-w-0 grid-cols-1 gap-3 @min-[32rem]:grid-cols-2">
            <Field label="Server" required>
              <Input
                value={host}
                onValueChange={(next) => { setHost(next); setLink({ state: "idle" }); publish({ host: next, share, folder }); }}
                placeholder="10.0.1.190 or nas.local"
                spellCheck={false}
                className="font-mono"
              />
            </Field>
            <Field label="Share" required>
              <Input
                value={share}
                onValueChange={(next) => { setShare(next); setLink({ state: "idle" }); publish({ host, share: next, folder }); }}
                placeholder="media"
                spellCheck={false}
                className="font-mono"
              />
            </Field>
            <Field label="User name" hint={savedUser ? `A sign-in for ${savedUser || "a guest"} is saved for this share.` : "Leave blank for a share that allows guests."}>
              <Input
                value={username}
                onValueChange={(next) => { setUsername(next); setLink({ state: "idle" }); }}
                autoComplete="off"
                spellCheck={false}
              />
            </Field>
            <Field label="Password" hint={savedUser ? "Leave blank to keep the saved password." : "Saved encrypted, once for this share."}>
              <PasswordInput
                value={password}
                onValueChange={(next) => { setPassword(next); setLink({ state: "idle" }); }}
                // "new" keeps a browser from filling in the account password; no strength bar, it is
                // the file server's password, not one being chosen.
                purpose="new"
                strength={false}
              />
            </Field>
            </div>
          </div>
          <div className="flex min-w-0 flex-wrap items-center gap-3">
            <Button variant="secondary" size="sm" isPending={link.state === "testing"} isDisabled={!canTest} onPress={() => void testConnection()}>
              Test connection
            </Button>
            <p role="status" className={link.state === "failed" ? "min-w-0 text-body-sm text-danger [overflow-wrap:anywhere]" : "min-w-0 text-body-sm text-muted"}>
              {link.state === "failed" && link.message}
              {link.state === "connected" && (link.folders === null ? "Connected with the saved sign-in." : `Connected. ${link.folders} ${link.folders === 1 ? "folder" : "folders"} at the top of the share. The sign-in is saved.`)}
              {link.state === "testing" && "Connecting…"}
            </p>
          </div>
          <Field {...root} label="Folder in the share" hint="Leave blank to use the whole share." required={required}>
            <Input
              value={folder}
              onValueChange={(next) => { setFolder(next); publish({ host, share, folder: next }); }}
              onBlur={field.handleBlur}
              placeholder="Movies"
              spellCheck={false}
              className="font-mono"
              aria-invalid={error ? true : undefined}
              trailing={(
                <Tooltip content={connected ? "Browse the share" : "Test the connection first"}>
                  <GlassButton variant="ghost" size="xs" icon={<IconFolder />} aria-label="Browse the share" disabled={!connected || !rootAddress} onClick={() => setBrowsing(true)} />
                </Tooltip>
              )}
            />
          </Field>
          <FolderPickerDialog
            isOpen={browsing}
            onOpenChange={setBrowsing}
            initialPath={composeShareAddress({ host, share, folder }) || undefined}
            onPick={(path) => {
              const picked = parseShareAddress(path);
              if (!picked) return;
              setFolder(picked.folder);
              publish({ host, share, folder: picked.folder });
            }}
          />
        </>
      )}
    </div>
  );
}
