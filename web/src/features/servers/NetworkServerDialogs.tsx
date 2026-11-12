import { useMutation } from "@apollo/client/react";
import { Badge } from "glass-ux/badge";
import { EntitySettingsDialog, type EntitySettingsTab } from "glass-ux/entity-settings";
import { Field } from "glass-ux/field";
import { formBehaviour, useAppForm, useBoundField } from "glass-ux/form";
import { IconPlus, IconSettings, IconTrash } from "glass-ux/icons";
import { Input } from "glass-ux/input";
import { PasswordInput } from "glass-ux/password-field";
import { toast } from "glass-ux/toast";
import { IconFolderShare } from "@tabler/icons-react";
import { useEffect, useMemo, useState } from "react";
import { z } from "zod";

import { Button, ConfirmDialog, Dialog } from "@/components/ui";
import {
  EntityNetworkServerCreateDocument,
  EntityNetworkServerDeleteDocument,
  EntityNetworkServerUpdateDocument,
  NetworkServerSharesDocument,
  TestNetworkServerDocument,
  type NetworkServerFieldsFragment,
  type TestNetworkServerMutation,
} from "@/graphql/generated/graphql";
import { glyph } from "@/lib/entities/kinds";
import { pluralize } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { withoutAction } from "@/lib/actions/withoutAction";

import { networkServerIdentityProps } from "./NetworkServerIdentity";
import { testSummary, useNetworkServerActions, type NetworkServerSettingsTab } from "./useNetworkServerActions";

const SharesIcon = glyph(IconFolderShare, "folder-share");

const schema = z.object({
  name: z.string().trim().max(80),
  host: z.string().trim().min(1, "Enter the server's name or IP address").refine((value) => !/[/\\\s]/.test(value), "Only the server: choose the share separately"),
  port: z.number().int().min(1).max(65535).nullable(),
  username: z.string().trim(),
  password: z.string(),
});
type Values = z.input<typeof schema>;

const EMPTY: Values = { name: "", host: "", port: null, username: "", password: "" };
const valuesOf = (server: NetworkServerFieldsFragment): Values => ({ name: server.name, host: server.host, port: server.port ?? null, username: server.username ?? "", password: "" });

type TestResult = TestNetworkServerMutation["testNetworkServer"];

/** The password box, bound to the form. Blank keeps what is saved. */
function PasswordBox({ saved }: { saved: boolean }) {
  const { field, root } = useBoundField<string>();
  return (
    <Field {...root} label="Password" hint={saved ? "Leave blank to keep the saved password." : "Saved encrypted. Leave blank for a guest share."}>
      <PasswordInput
        value={field.state.value}
        onValueChange={(value) => field.handleChange(value)}
        onBlur={field.handleBlur}
        // "new" keeps a browser from filling in the account password; no strength bar, it is the
        // file server's password, not one being chosen.
        purpose="new"
        strength={false}
      />
    </Field>
  );
}

/** The sign-in boxes every server form has. */
function ConnectionFields({ form, saved }: { form: ReturnType<typeof useServerForm>; saved: boolean }) {
  return (
    <div className="@container min-w-0">
      <div className="grid min-w-0 grid-cols-1 gap-3 @min-[32rem]:grid-cols-[minmax(0,1fr)_8rem]">
        <form.AppField name="host">{(field) => <field.TextField label="Server" placeholder="10.0.1.190 or nas.local" required className="font-mono" />}</form.AppField>
        <form.AppField name="port">{(field) => <field.PortField label="Port" placeholder="445" />}</form.AppField>
      </div>
      <div className="mt-3 grid min-w-0 grid-cols-1 gap-3 @min-[32rem]:grid-cols-2">
        <form.AppField name="username">{(field) => <field.TextField label="User name" autoComplete="off" hint="Leave blank for a guest sign-in." />}</form.AppField>
        <form.AppField name="password">{() => <PasswordBox saved={saved} />}</form.AppField>
      </div>
      <div className="mt-3">
        <form.AppField name="name">{(field) => <field.TextField label="Name" placeholder="Uses the server's address when left blank" hint="What Librarian calls it, such as TrueNAS." />}</form.AppField>
      </div>
    </div>
  );
}

function useServerForm(initial: Values, onSubmit: (values: Values) => Promise<void>) {
  return useAppForm({
    ...formBehaviour,
    defaultValues: initial,
    validators: { onDynamic: schema },
    onSubmit: async ({ value, formApi }) => {
      await onSubmit(schema.parse(value) as Values);
      formApi.reset({ ...value, password: "" });
    },
  });
}

/** Test with what is typed (and, for a saved server, what is saved), and show the outcome. */
function TestRow({ form, serverId, onResult }: { form: ReturnType<typeof useServerForm>; serverId?: string; onResult?: (result: TestResult) => void }) {
  const [test, { loading }] = useMutation(TestNetworkServerDocument);
  const [result, setResult] = useState<TestResult | null>(null);
  const run = async () => {
    const values = form.state.values;
    try {
      const { data } = await test({ variables: { input: { id: serverId ?? null, host: values.host.trim(), port: values.port ?? null, username: values.username.trim(), password: values.password } } });
      const outcome = data?.testNetworkServer ?? null;
      setResult(outcome);
      if (outcome) onResult?.(outcome);
    } catch (error) {
      setResult({ success: false, error: errorMessage(error), shares: [], sharesError: null, folders: null });
    }
  };
  return (
    <form.Subscribe selector={(state) => state.values.host}>
      {(host) => (
        <div className="flex min-w-0 flex-wrap items-center gap-3">
          <Button variant="secondary" size="sm" isPending={loading} isDisabled={!host.trim()} onPress={() => void run()}>Test connection</Button>
          <p role="status" className={result && !result.success ? "min-w-0 text-body-sm text-danger [overflow-wrap:anywhere]" : "min-w-0 text-body-sm text-muted [overflow-wrap:anywhere]"}>
            {loading ? "Connecting…" : result ? testSummary(result) : "Librarian signs in itself, so nothing has to be mounted. Use the IP address if the name doesn’t resolve from here."}
          </p>
        </div>
      )}
    </form.Subscribe>
  );
}

/**
 * Save a new file server: where it is and who to sign in as. Its shares are listed once it
 * answers. Calls `onCreated` with the saved server, so a location picker can choose it at once.
 */
export function AddNetworkServerDialog({ open, onOpenChange, onCreated }: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreated?: (server: NetworkServerFieldsFragment) => void;
}) {
  const [create] = useMutation(EntityNetworkServerCreateDocument);
  const [shares, setShares] = useState<string[]>([]);
  const form = useServerForm(EMPTY, async (values) => {
    try {
      const { data } = await create({
        variables: {
          input: {
            name: values.name.trim(), host: values.host.trim(), port: values.port ?? null, username: values.username.trim() || null, password: values.password,
            hasPassword: false, knownShares: shares, status: null, lastError: null, lastCheckedAt: null,
          },
        },
      });
      const result = assertSuccess(data?.createNetworkServer, "The server could not be saved");
      toast.success(`${result.networkServer?.name ?? values.host} saved`);
      if (result.networkServer) onCreated?.(result.networkServer);
      onOpenChange(false);
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  });
  useEffect(() => { if (open) { form.reset(EMPTY); setShares([]); } }, [open]); // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <Dialog
      isOpen={open}
      onOpenChange={onOpenChange}
      title="Add network share"
      size="md"
      footer={(
        <>
          <Button variant="ghost" onPress={() => onOpenChange(false)}>Cancel</Button>
          <form.Subscribe selector={(state) => state.isSubmitting}>
            {(saving) => <Button variant="primary" isPending={saving} onPress={() => void form.handleSubmit()}>Save server</Button>}
          </form.Subscribe>
        </>
      )}
    >
      <form.AppForm>
        <form noValidate className="flex flex-col gap-3" onSubmit={(event) => { event.preventDefault(); void form.handleSubmit(); }}>
          <p className="text-body-sm text-muted">Save the file server once, with its sign-in. Libraries and the download folder then choose a share on it without asking again.</p>
          <ConnectionFields form={form} saved={false} />
          <TestRow form={form} onResult={(result) => { if (result.success) setShares(result.shares.map((share) => share.name)); }} />
          {shares.length > 0 && <p className="text-label-sm text-muted">Shares: <span className="font-mono text-foreground">{shares.join(", ")}</span></p>}
        </form>
      </form.AppForm>
    </Dialog>
  );
}

/** The shares a saved server offers, and those typed in by hand where it would not list them. */
function SharesPanel({ server }: { server: NetworkServerFieldsFragment }) {
  const listing = useQuery(NetworkServerSharesDocument, { variables: { id: server.id }, fetchPolicy: "network-only" });
  const [update, { loading: saving }] = useMutation(EntityNetworkServerUpdateDocument);
  const [typed, setTyped] = useState("");
  const result = listing.data?.networkServerShares;
  const shares = result?.shares ?? server.knownShares.map((name) => ({ __typename: "NetworkShare" as const, name, comment: null, listed: false }));
  const save = async (next: string[]) => {
    try {
      assertSuccess((await update({ variables: { id: server.id, input: { knownShares: next } } })).data?.updateNetworkServer, "The shares could not be saved");
      void listing.refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };
  const add = async () => {
    const name = typed.trim().replace(/^[/\\]+|[/\\]+$/g, "");
    if (!name) return;
    await save([...server.knownShares, name]);
    setTyped("");
  };
  return (
    <div className="flex min-w-0 flex-col gap-4">
      <p className="text-body-sm text-muted">
        {listing.loading && !result ? "Asking the server for its shares…" : result?.success ? `${server.name} offers ${pluralize(shares.filter((share) => share.listed).length, "share")}.` : `The server did not list its shares${result?.error ? `: ${result.error}` : ""}. The shares below are the ones remembered.`}
      </p>
      {shares.length > 0 ? (
        <ul className="flex min-w-0 flex-col divide-y divide-separator">
          {shares.map((share) => (
            <li key={share.name} className="flex min-w-0 items-center gap-3 py-2">
              <span className="min-w-0 flex-1">
                <span className="block font-mono text-body-sm text-foreground [overflow-wrap:anywhere]">{share.name}</span>
                {share.comment && <span className="block text-label-sm text-muted">{share.comment}</span>}
              </span>
              <Badge size="sm" tone={share.listed ? "success" : "neutral"}>{share.listed ? "Offered" : "Remembered"}</Badge>
              {!share.listed && server.knownShares.includes(share.name.toLowerCase()) && (
                <Button variant="ghost" size="sm" isDisabled={saving} onPress={() => void save(server.knownShares.filter((name) => name !== share.name.toLowerCase()))}>Forget</Button>
              )}
            </li>
          ))}
        </ul>
      ) : null}
      <div className="flex min-w-0 flex-wrap items-end gap-3">
        <Field label="Add a share by name" hint="For a share the server will not list. It is remembered, so it is never typed again." className="min-w-0 flex-1">
          <Input value={typed} onValueChange={setTyped} placeholder="media" spellCheck={false} className="font-mono" />
        </Field>
        <Button variant="secondary" isDisabled={!typed.trim() || saving} onPress={() => void add()}><IconPlus size="md" /> Add share</Button>
      </div>
    </div>
  );
}

/** A saved server's settings, with its identity and actions in the header. */
export function NetworkServerSettingsDialog({ server, open, onOpenChange, tab, onTabChange, onChanged }: {
  server: NetworkServerFieldsFragment;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  tab: NetworkServerSettingsTab;
  onTabChange: (tab: NetworkServerSettingsTab) => void;
  onChanged?: () => void;
}) {
  const [update] = useMutation(EntityNetworkServerUpdateDocument);
  const form = useServerForm(valuesOf(server), async (values) => {
    const input: Record<string, unknown> = { name: values.name.trim() || values.host.trim(), host: values.host.trim(), port: values.port ?? null, username: values.username.trim() || null };
    // A blank password keeps the saved one; a guest sign-in has none.
    if (values.password) input.password = values.password;
    else if (!values.username.trim() && server.hasPassword) input.password = "";
    assertSuccess((await update({ variables: { id: server.id, input } })).data?.updateNetworkServer, "The server could not be saved");
    onChanged?.();
  });
  useEffect(() => { if (open) form.reset(valuesOf(server)); }, [open, server.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const actions = useNetworkServerActions<NetworkServerFieldsFragment>({ settings: (_, next) => onTabChange(next ?? "connection"), onChanged });
  const headerActions = useMemo(() => withoutAction(actions, "settings", "remove"), [actions]);
  const [remove, { loading: removing }] = useMutation(EntityNetworkServerDeleteDocument);
  const [confirmRemove, setConfirmRemove] = useState(false);
  const onRemove = async () => {
    try {
      assertSuccess((await remove({ variables: { id: server.id } })).data?.deleteNetworkServer, `${server.name} could not be removed`);
      toast.success(`${server.name} removed`);
      setConfirmRemove(false);
      onOpenChange(false);
      onChanged?.();
    } catch (error) {
      setConfirmRemove(false);
      toast.danger(errorMessage(error));
    }
  };

  const tabs: EntitySettingsTab[] = [
    {
      value: "connection", label: "Connection", icon: <IconSettings />, fields: ["name", "host", "port", "username", "password"],
      content: (
        <div className="flex flex-col gap-3">
          <ConnectionFields form={form} saved={server.hasPassword} />
          <TestRow form={form} serverId={server.id} onResult={() => onChanged?.()} />
        </div>
      ),
    },
    {
      value: "shares", label: "Shares", icon: <SharesIcon />, count: server.knownShares.length || undefined,
      content: tab === "shares" ? <SharesPanel server={server} /> : null,
    },
    {
      value: "remove", label: "Remove", icon: <IconTrash />,
      content: (
        <div className="flex flex-col items-start gap-3">
          <p className="text-body-sm text-muted">Removing the server deletes its saved sign-in. While a library, the download folder or a stored torrent uses one of its shares, it cannot be removed: move them first.</p>
          <Button variant="danger-soft" onPress={() => setConfirmRemove(true)}>
            <IconTrash size="md" /> Remove server
          </Button>
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
        identity={networkServerIdentityProps(server, { actions: headerActions })}
        tabs={tabs}
        tab={tab}
        onTabChange={(next) => onTabChange(next as NetworkServerSettingsTab)}
        onSaveError={(error) => toast.danger(errorMessage(error, "Could not save the server"))}
      />
      <ConfirmDialog
        isOpen={confirmRemove}
        onOpenChange={setConfirmRemove}
        title={`Remove ${server.name}?`}
        description="Its saved sign-in is deleted. Nothing on the server is touched."
        confirmLabel="Remove server"
        destructive
        isPending={removing}
        onConfirm={onRemove}
      />
    </>
  );
}
