import { useMutation } from "@apollo/client/react";
import { toast } from "glass-ux/toast";
import { zodResolver } from "@hookform/resolvers/zod";
import { defineActions } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, DataTable, IdentityCell, RelativeTime, TextCell } from "glass-ux/data-table";
import { IconCopy, IconPlus, IconTrash } from "glass-ux/icons";
import { MeterCell } from "glass-ux/meter-cell";
import { IconShieldCheck, IconUserOff } from "@tabler/icons-react";
import { useMemo, useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { Button, Dialog, FieldGroup, FormNumberField, FormSelectField } from "@/components/ui";
import {
  EntityInviteTokenCreateDocument,
  EntityInviteTokenDeleteDocument,
  EntityInviteTokenListDocument,
  EntityUserDeleteDocument,
  EntityUserListDocument,
  EntityUserUpdateDocument,
  EntityLibraryListDocument,
  type InviteTokenFieldsFragment,
  type UserFieldsFragment,
} from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { ENTITY_KINDS, glyph } from "@/lib/entities/kinds";
import { parseTimestamp } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

import { SettingsLoadError } from "./SettingsLoadError";
import { SettingsSection } from "./SettingsSection";
import { useSettingsConnection } from "./useSettingsConnection";

const inviteSchema = z.object({ role: z.enum(["member", "admin"]), accessLevel: z.string(), maxUses: z.number().int().min(1).max(100), expiresInDays: z.number().int().min(1).max(365) });
type InviteValues = z.infer<typeof inviteSchema>;

const UserGlyph = ENTITY_KINDS.user.icon;
const RoleIcon = glyph(IconShieldCheck, "shield-check");
const SignInIcon = glyph(IconUserOff, "user-off");
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

  const [updateUser] = useMutation(EntityUserUpdateDocument);
  const [deleteUser] = useMutation(EntityUserDeleteDocument);
  const [createInvite, { loading: creatingInvite }] = useMutation(EntityInviteTokenCreateDocument);
  const [deleteInvite] = useMutation(EntityInviteTokenDeleteDocument);
  const [inviting, setInviting] = useState(false);
  const inviteForm = useForm<InviteValues>({ resolver: zodResolver(inviteSchema), defaultValues: { role: "member", accessLevel: "full", maxUses: 1, expiresInDays: 7 } });

  const inviteLink = (token: InviteTokenFieldsFragment) => `${window.location.origin}/register?invite=${encodeURIComponent(token.id)}`;
  const copy = async (token: InviteTokenFieldsFragment) => {
    await navigator.clipboard.writeText(inviteLink(token));
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
      id: "name", header: "User", width: 24, priority: 1, cardRole: "title", sortable: true,
      cell: ({ row }) => <IdentityCell icon={<UserGlyph />} title={`${row.original.displayName ?? row.original.username}${row.original.id === me?.id ? " (you)" : ""}`} subtitle={[row.original.username, row.original.email].filter(Boolean).join(" · ")} status={row.original.isActive ? "online" : "offline"} statusLabel={row.original.isActive ? "Active" : "Disabled"} />,
    }),
    userColumn.accessor("role", { header: "Role", width: 8, priority: 2, sortable: true, cell: ({ row }) => <Badge size="sm" tone={row.original.role === "admin" ? "accent" : "neutral"}>{row.original.role === "admin" ? "Admin" : "Member"}</Badge> }),
    userColumn.accessor((user) => (user.isActive ? "active" : "disabled"), { id: "active", header: "Status", width: 8, priority: 2, cardRole: "status", cell: ({ row }) => <Badge size="sm" marker="dot" tone={row.original.isActive ? "success" : "danger"}>{row.original.isActive ? "Active" : "Disabled"}</Badge> }),
    userColumn.accessor((user) => user.lastLoginAt ?? "", { id: "last", header: "Last sign-in", width: 9, priority: 3, sortable: true, sortDescFirst: true, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.lastLoginAt)} fallback="Never" /> }),
    userColumn.accessor("createdAt", { header: "Joined", width: 9, priority: 4, sortable: true, sortDescFirst: true, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.createdAt)} /> }),
  ]), [me?.id]);

  const refreshUsers = users.refetch;
  const userActions = useMemo(() => defineActions<UserFieldsFragment>("user", [
    {
      id: "role", label: (rows) => (rows[0]!.role === "admin" ? "Make member" : "Make admin"), icon: RoleIcon, group: "manage", priority: 1,
      visible: (rows) => rows.length === 1 && rows[0]!.id !== me?.id,
      run: async (rows) => { const user = rows[0]!; assertSuccess((await updateUser({ variables: { id: user.id, input: { role: user.role === "admin" ? "member" : "admin" } } })).data?.updateUser, "Could not update"); void refreshUsers(); },
      successMessage: (rows) => (rows[0]!.role === "admin" ? "Now a member" : "Now an admin"),
    },
    {
      id: "active", label: (rows) => (rows[0]!.isActive ? "Disable sign-in" : "Enable sign-in"), icon: SignInIcon, group: "manage", priority: 2,
      visible: (rows) => rows.length === 1 && rows[0]!.id !== me?.id,
      confirm: (rows) => (rows[0]!.isActive ? { title: `Disable ${rows[0]!.displayName ?? rows[0]!.username}?`, body: "They are signed out and cannot sign in until enabled again. Their history is kept.", confirmLabel: "Disable", tone: "warning" } : null),
      run: async (rows) => { const user = rows[0]!; assertSuccess((await updateUser({ variables: { id: user.id, input: { isActive: !user.isActive } } })).data?.updateUser, "Could not update"); void refreshUsers(); },
    },
    {
      id: "delete", label: "Delete user", icon: IconTrash, group: "danger",
      visible: (rows) => rows.length === 1 && rows[0]!.id !== me?.id,
      confirm: (rows) => ({ title: `Delete ${rows[0]!.displayName ?? rows[0]!.username}?`, body: "Their playback history is removed. Libraries and media are kept.", confirmLabel: "Delete user", tone: "danger" }),
      run: async (rows) => { assertSuccess((await deleteUser({ variables: { id: rows[0]!.id } })).data?.deleteUser, "Could not delete"); void refreshUsers(); },
      successMessage: () => "User deleted",
    },
  ]), [deleteUser, me?.id, refreshUsers, updateUser]);

  const refreshInvites = invites.refetch;
  const inviteActions = useMemo(() => defineActions<InviteTokenFieldsFragment>("invite", [
    { id: "copy", label: "Copy link", icon: IconCopy, group: "open", priority: 1, visible: (rows) => rows.length === 1, run: (rows) => copy(rows[0]!) },
    {
      id: "revoke", label: "Revoke", icon: IconTrash, group: "danger",
      confirm: () => ({ title: "Revoke this invite?", body: "The link stops working immediately. Accounts already created with it are kept.", confirmLabel: "Revoke", tone: "danger" }),
      run: async (rows) => { for (const token of rows) await deleteInvite({ variables: { id: token.id } }); void refreshInvites(); },
      successMessage: () => "Invite revoked",
    },
  ]), [deleteInvite, refreshInvites]);

  return (
    <div className="flex flex-col gap-8">
      <SettingsSection title="Users" actions={<Button size="sm" variant="primary" onPress={() => setInviting(true)}><IconPlus /> Invite</Button>}>
        <SettingsLoadError error={users.error} retry={users.refetch} hasRows={userRows.length > 0} />
        <DataTable<UserFieldsFragment> height="auto" id={`librarian.${me?.id ?? "session"}.settings.users`} label="Users" columns={userColumns} rows={userRows} getRowId={(user) => user.id} getRowLabel={(user) => user.displayName ?? user.username} loading={users.loading && userRows.length === 0} actions={userActions} noun={{ one: "user", other: "users" }} emptyMessage="No users" />
      </SettingsSection>
      <SettingsSection title="Open invites" description="Share a link; it stops working after the last use or when it expires.">
        <SettingsLoadError error={invites.error} retry={invites.refetch} hasRows={inviteRows.length > 0} />
        <DataTable<InviteTokenFieldsFragment> height="auto" id={`librarian.${me?.id ?? "session"}.settings.invites`} label="Open invites" columns={inviteColumns} rows={inviteRows} getRowId={(token) => token.id} getRowLabel={(token) => `Invite ${token.id}`} loading={invites.loading && inviteRows.length === 0} actions={inviteActions} density="compact" noun={{ one: "invite", other: "invites" }} emptyMessage="No open invites" />
      </SettingsSection>

      <Dialog isOpen={inviting} onOpenChange={setInviting} title="Invite someone" size="sm" footer={<><Button variant="ghost" onPress={() => setInviting(false)}>Cancel</Button><Button variant="primary" onPress={() => void submitInvite()} isPending={creatingInvite} isDisabled={libraries.loading || Boolean(libraries.error)}>Create and copy link</Button></>}>
        <SettingsLoadError error={libraries.error} retry={libraries.refetch} />
        <form onSubmit={submitInvite} noValidate>
          <FieldGroup>
            <FormSelectField control={inviteForm.control} name="role" label="Role" options={[{ key: "member", label: "Member", description: "Browse and play" }, { key: "admin", label: "Admin", description: "Manage libraries, sources and users" }]} />
            <FormSelectField control={inviteForm.control} name="accessLevel" label="Library access" options={[{ key: "full", label: "All libraries" }, { key: "restricted", label: "Restricted" }]} />
            <FormNumberField control={inviteForm.control} name="maxUses" label="Maximum uses" min={1} max={100} />
            <FormNumberField control={inviteForm.control} name="expiresInDays" label="Expires in (days)" min={1} max={365} />
          </FieldGroup>
        </form>
      </Dialog>
    </div>
  );
}
