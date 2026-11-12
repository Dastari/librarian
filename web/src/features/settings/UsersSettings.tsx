import { LoadedDataTable } from "@/components/ui/LoadedDataTable";
import { copyText } from "glass-ux/copy-button";
import { useMutation } from "@apollo/client/react";
import { toast } from "glass-ux/toast";
import { zodResolver } from "@hookform/resolvers/zod";
import { defineActions } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, RelativeTime, TextCell } from "glass-ux/data-table";
import { IconCopy, IconPlus, IconTrash } from "glass-ux/icons";
import { MeterCell } from "glass-ux/meter-cell";
import { useCallback, useMemo, useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { Button, Dialog, FieldGroup, FormNumberField, FormSelectField } from "@/components/ui";
import {
  EntityInviteTokenCreateDocument,
  EntityInviteTokenDeleteDocument,
  EntityInviteTokenListDocument,
  EntityUserListDocument,
  EntityLibraryListDocument,
  type InviteTokenFieldsFragment,
  type UserFieldsFragment,
} from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { parseTimestamp } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

import { TableBulkActions } from "@/components/ui/TableBulkActions";
import { UserIdentity } from "@/features/users/UserIdentity";
import { UserSettingsDialog } from "@/features/users/UserSettingsDialog";
import { useUserActions, type UserSettingsTab } from "@/features/users/useUserActions";
import { runSettingsSelection } from "./runSettingsSelection";

import { SettingsLoadError } from "./SettingsLoadError";
import { SettingsPage, SettingsSection } from "./SettingsSection";
import { useSettingsConnection } from "./useSettingsConnection";

const inviteSchema = z.object({ role: z.enum(["member", "admin"]), accessLevel: z.string(), maxUses: z.number().int().min(1).max(100), expiresInDays: z.number().int().min(1).max(365) });
type InviteValues = z.infer<typeof inviteSchema>;

const userColumn = createDataTableColumns<UserFieldsFragment>();
const inviteColumn = createDataTableColumns<InviteTokenFieldsFragment>();
const inviteColumns = inviteColumn.columns([
  inviteColumn.accessor("id", { header: "Invite", width: 22, priority: 1, cardRole: "title", cell: ({ row }) => <TextCell identifier>{row.original.id}</TextCell> }),
  inviteColumn.accessor("role", { header: "Role", width: 8, priority: 2, cell: ({ row }) => <Badge size="sm" tone={row.original.role === "admin" ? "accent" : "neutral"}>{row.original.role === "admin" ? "Admin" : "Member"}</Badge> }),
  inviteColumn.accessor((token) => token.useCount, {
    id: "uses", header: "Uses", width: 10, priority: 2,
    cell: ({ row }) => (row.original.maxUses ? <MeterCell label="Uses" value={row.original.useCount / row.original.maxUses} max={1} tone="data" warningAt={2} dangerAt={2} valueText={`${row.original.useCount} of ${row.original.maxUses}`} /> : <TextCell muted>{`${row.original.useCount} · unlimited`}</TextCell>),
  }),
  inviteColumn.accessor((token) => token.expiresAt ?? "", { id: "expires", header: "Expires", width: 9, priority: 2, sortable: true, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.expiresAt)} fallback="Never" /> }),
]);

export function UsersSettings() {
  const { user: me } = useSession();
  const users = useSettingsConnection(EntityUserListDocument, { orderBy: [{ username: "ASC" }] }, (data) => data.users);
  const invites = useSettingsConnection(EntityInviteTokenListDocument, { where: { isActive: { eq: true } }, orderBy: [{ createdAt: "DESC" }] }, (data) => data.inviteTokens);
  const libraries = useSettingsConnection(EntityLibraryListDocument, { orderBy: [{ name: "ASC" }] }, (data) => data.libraries);
  const userRows = users.rows;
  const inviteRows = invites.rows;

  const [createInvite, { loading: creatingInvite }] = useMutation(EntityInviteTokenCreateDocument);
  const [deleteInvite] = useMutation(EntityInviteTokenDeleteDocument);
  const [inviting, setInviting] = useState(false);
  const inviteForm = useForm<InviteValues>({ resolver: zodResolver(inviteSchema), defaultValues: { role: "member", accessLevel: "full", maxUses: 1, expiresInDays: 7 } });

  const inviteLink = (token: InviteTokenFieldsFragment) => `${window.location.origin}/register?invite=${encodeURIComponent(token.id)}`;
  const copy = async (token: InviteTokenFieldsFragment) => {
    await copyText(inviteLink(token));
    toast.success("Invite link copied");
  };

  const submitInvite = inviteForm.handleSubmit(async (values) => {
    if (!me || libraries.loading || libraries.error) return;
    try {
      const expiresAt = new Date(Date.now() + values.expiresInDays * 86400_000).toISOString();
      const { data } = await createInvite({ variables: { input: { createdBy: me.id, libraryIds: libraries.rows.map((library) => library.id), role: values.role, accessLevel: values.accessLevel, maxUses: values.maxUses, useCount: 0, applyRestrictions: false, isActive: true, expiresAt } } });
      const token = assertSuccess(data?.createInviteToken, "Could not create invite").inviteToken;
      if (token) await copy(token);
      setInviting(false);
      void invites.refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  });

  const userColumns = useMemo(() => userColumn.columns([
    userColumn.accessor((user) => user.displayName ?? user.username, {
      id: "name", header: "User", width: 26, priority: 1, cardRole: "title", sortable: true,
      cell: ({ row }) => <UserIdentity user={row.original} self={row.original.id === me?.id} />,
    }),
    userColumn.accessor("role", { header: "Role", width: 8, priority: 2, sortable: true, cell: ({ row }) => <Badge size="sm" tone={row.original.role === "admin" ? "accent" : "neutral"}>{row.original.role === "admin" ? "Admin" : "Member"}</Badge> }),
    userColumn.accessor((user) => (user.isActive ? "active" : "disabled"), { id: "active", header: "Status", width: 8, priority: 2, cardRole: "status", cell: ({ row }) => <Badge size="sm" marker="dot" tone={row.original.isActive ? (row.original.mustChangePassword ? "warning" : "success") : "danger"}>{row.original.isActive ? (row.original.mustChangePassword ? "Temporary password" : "Active") : "Disabled"}</Badge> }),
    userColumn.accessor((user) => (user.twoFactorEnabledAt ? "on" : "off"), { id: "twoFactor", header: "Two-factor", width: 8, priority: 3, cell: ({ row }) => (row.original.twoFactorEnabledAt ? <Badge size="sm" marker="dot" tone="success">On</Badge> : <TextCell muted>Off</TextCell>) }),
    userColumn.accessor((user) => user.lastLoginAt ?? "", { id: "last", header: "Last sign-in", width: 9, priority: 3, sortable: true, sortDescFirst: true, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.lastLoginAt)} fallback="Never" /> }),
    userColumn.accessor("createdAt", { header: "Joined", width: 9, priority: 4, sortable: true, sortDescFirst: true, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.createdAt)} /> }),
  ]), [me?.id]);

  const refreshUsers = users.refetch;
  const [editing, setEditing] = useState<{ id: string; tab: UserSettingsTab } | null>(null);
  const openUser = useCallback((user: UserFieldsFragment, tab?: UserSettingsTab) => setEditing({ id: user.id, tab: tab ?? "profile" }), []);
  const refreshAfterChange = useCallback(() => void refreshUsers(), [refreshUsers]);
  const { actions: userActions, dialogs: userDialogs } = useUserActions<UserFieldsFragment>({ settings: openUser, onChanged: refreshAfterChange });

  const refreshInvites = invites.refetch;
  const inviteActions = useMemo(() => defineActions<InviteTokenFieldsFragment>("invite", [
    { id: "copy", label: (rows) => rows.length === 1 ? "Copy link" : "Copy invite links", icon: IconCopy, group: "open", priority: 1, bulk: true, run: (rows) => copyText(rows.map(inviteLink).join("\n")), successMessage: (rows) => rows.length === 1 ? "Invite link copied" : "Invite links copied" },
    {
      id: "revoke", label: (rows) => rows.length === 1 ? "Revoke" : `Revoke ${rows.length} invites`, icon: IconTrash, group: "danger", bulk: true,
      confirm: (rows) => ({ title: rows.length === 1 ? "Revoke this invite?" : `Revoke ${rows.length} invites?`, body: "The links stop working immediately. Accounts already created with them are kept.", confirmLabel: "Revoke", tone: "danger" }),
      run: (rows) => runSettingsSelection(rows, "Revoke", (row) => row.id, async (row) => assertSuccess((await deleteInvite({ variables: { id: row.id } })).data?.deleteInviteToken, "Could not revoke invite"), refreshInvites),
      successMessage: (rows) => rows.length === 1 ? "Invite revoked" : `${rows.length} invites revoked`,
    },
  ]), [deleteInvite, refreshInvites]);

  return (
    <SettingsPage>
      <SettingsSection title="Users" actions={<Button size="sm" variant="primary" onPress={() => setInviting(true)}><IconPlus /> Invite</Button>}>
        <SettingsLoadError error={users.error} retry={users.refetch} hasRows={userRows.length > 0} />
        <LoadedDataTable<UserFieldsFragment> complete={!users.loading && !users.error} height="auto" selectable queryKey={me?.id} id={`librarian.${me?.id ?? "session"}.settings.users`} label="Users" columns={userColumns} defaultHiddenColumns={["twoFactor"]} rows={userRows} getRowId={(user) => user.id} getRowLabel={(user) => user.displayName ?? user.username} loading={users.loading && userRows.length === 0} actions={userActions} renderBulkActions={(rows) => <TableBulkActions set={userActions} rows={rows} noun={{ one: "user", other: "users" }} />} onRowOpen={(user) => openUser(user)} rowOpen="click" noun={{ one: "user", other: "users" }} emptyMessage="No users" />
      </SettingsSection>
      <SettingsSection title="Open invites" description="Share a link; it stops working after the last use or when it expires.">
        <SettingsLoadError error={invites.error} retry={invites.refetch} hasRows={inviteRows.length > 0} />
        <LoadedDataTable<InviteTokenFieldsFragment> complete={!invites.loading && !invites.error} height="auto" selectable queryKey={me?.id} id={`librarian.${me?.id ?? "session"}.settings.invites`} label="Open invites" columns={inviteColumns} rows={inviteRows} getRowId={(token) => token.id} getRowLabel={(token) => `Invite ${token.id}`} loading={invites.loading && inviteRows.length === 0} actions={inviteActions} renderBulkActions={(rows) => <TableBulkActions set={inviteActions} rows={rows} noun={{ one: "invite", other: "invites" }} />} noun={{ one: "invite", other: "invites" }} emptyMessage="No open invites" />
      </SettingsSection>

      {userDialogs}
      {editing ? (
        <UserSettingsDialog
          userId={editing.id}
          open
          onOpenChange={(next) => !next && setEditing(null)}
          tab={editing.tab}
          onTabChange={(tab) => setEditing((state) => (state ? { ...state, tab } : state))}
          onChanged={refreshAfterChange}
        />
      ) : null}
      <Dialog isOpen={inviting} onOpenChange={setInviting} title="Invite someone" size="sm" footer={<><Button variant="ghost" onPress={() => setInviting(false)}>Cancel</Button><Button variant="primary" onPress={() => void submitInvite()} isPending={creatingInvite} isDisabled={libraries.loading || Boolean(libraries.error)}>Create and copy link</Button></>}>
        <SettingsLoadError error={libraries.error} retry={libraries.refetch} />
        <form onSubmit={submitInvite} noValidate>
          <FieldGroup>
            <FormSelectField control={inviteForm.control} name="role" label="Role" options={[{ key: "member", label: "Member", description: "Browse and play" }, { key: "admin", label: "Admin", description: "Manage libraries, sources and users" }]} />
            <FormSelectField control={inviteForm.control} name="accessLevel" label="Library access" options={[{ key: "full", label: "All libraries" }, { key: "restricted", label: "Restricted" }]} />
            <FormNumberField control={inviteForm.control} name="maxUses" label="Maximum uses" min={1} max={100} suffix="uses" />
            <FormNumberField control={inviteForm.control} name="expiresInDays" label="Expires in" min={1} max={365} suffix="days" />
          </FieldGroup>
        </form>
      </Dialog>
    </SettingsPage>
  );
}
