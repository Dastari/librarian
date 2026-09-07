import { useMutation, useQuery } from "@apollo/client/react";
import { toast } from "@heroui/react";
import { zodResolver } from "@hookform/resolvers/zod";
import { IconCopy, IconPlus, IconTicket, IconTrash, IconUsers } from "@tabler/icons-react";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { Button, ConfirmDialog, DataTable, type DataTableColumn, type DataTableRowAction, Dialog, EmptyState, FieldGroup, FormNumberField, FormSelectField, Panel, StatusChip } from "@/components/ui";
import {
  EntityInviteTokenCreateDocument,
  EntityInviteTokenDeleteDocument,
  EntityInviteTokenListDocument,
  EntityUserDeleteDocument,
  EntityUserListDocument,
  EntityUserUpdateDocument,
  NavLibrariesDocument,
  type InviteTokenFieldsFragment,
  type UserFieldsFragment,
} from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { formatDate, formatRelative } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

const inviteSchema = z.object({ role: z.enum(["member", "admin"]), accessLevel: z.string(), maxUses: z.number().int().min(1).max(100), expiresInDays: z.number().int().min(1).max(365) });
type InviteValues = z.infer<typeof inviteSchema>;

export function UsersSettings() {
  const { user: me } = useSession();
  const users = useQuery(EntityUserListDocument, { variables: { orderBy: [{ username: "ASC" }], page: { limit: 100, offset: 0 } } });
  const invites = useQuery(EntityInviteTokenListDocument, { variables: { where: { isActive: { eq: true } }, orderBy: [{ createdAt: "DESC" }], page: { limit: 100, offset: 0 } } });
  const libraries = useQuery(NavLibrariesDocument);
  const userRows = (users.data ?? users.previousData)?.users.edges.map((edge) => edge.node) ?? [];
  const inviteRows = (invites.data ?? invites.previousData)?.inviteTokens.edges.map((edge) => edge.node) ?? [];

  const [updateUser] = useMutation(EntityUserUpdateDocument);
  const [deleteUser, { loading: deletingUser }] = useMutation(EntityUserDeleteDocument);
  const [createInvite, { loading: creatingInvite }] = useMutation(EntityInviteTokenCreateDocument);
  const [deleteInvite] = useMutation(EntityInviteTokenDeleteDocument);
  const [removing, setRemoving] = useState<UserFieldsFragment | null>(null);
  const [inviting, setInviting] = useState(false);
  const inviteForm = useForm<InviteValues>({ resolver: zodResolver(inviteSchema), defaultValues: { role: "member", accessLevel: "full", maxUses: 1, expiresInDays: 7 } });

  const inviteLink = (token: InviteTokenFieldsFragment) => `${window.location.origin}/register?invite=${encodeURIComponent(token.id)}`;
  const copy = async (token: InviteTokenFieldsFragment) => {
    await navigator.clipboard.writeText(inviteLink(token));
    toast.success("Invite link copied");
  };

  const submitInvite = inviteForm.handleSubmit(async (values) => {
    if (!me) return;
    try {
      const expiresAt = new Date(Date.now() + values.expiresInDays * 86400_000).toISOString();
      const { data } = await createInvite({ variables: { input: { createdBy: me.id, libraryIds: (libraries.data?.libraries.edges ?? []).map((edge) => edge.node.id), role: values.role, accessLevel: values.accessLevel, maxUses: values.maxUses, useCount: 0, applyRestrictions: false, isActive: true, expiresAt } } });
      const token = assertSuccess(data?.createInviteToken, "Could not create invite").inviteToken;
      if (token) await copy(token);
      setInviting(false);
      void invites.refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  });

  const userColumns: Array<DataTableColumn<UserFieldsFragment>> = [
    { id: "name", header: "User", cell: (user) => (<span className="min-w-0"><span className="block truncate text-body-sm text-foreground">{user.displayName ?? user.username}{user.id === me?.id ? <span className="ml-2 text-label-sm text-muted">(you)</span> : null}</span><span className="block truncate text-label-sm text-muted">{[user.username, user.email].filter(Boolean).join(" · ")}</span></span>) },
    { id: "role", header: "Role", size: 110, cell: (user) => <StatusChip status={{ label: user.role === "admin" ? "Admin" : "Member", tone: user.role === "admin" ? "accent" : "default", dot: user.role === "admin" ? "bg-info" : "bg-muted" }} /> },
    { id: "active", header: "Status", size: 110, cell: (user) => <StatusChip minimal status={user.isActive ? { label: "Active", tone: "success", dot: "bg-success" } : { label: "Disabled", tone: "danger", dot: "bg-danger" }} /> },
    { id: "last", header: "Last sign-in", size: 140, hideBelow: "md", cell: (user) => <span className="text-muted">{user.lastLoginAt ? formatRelative(user.lastLoginAt) : "Never"}</span> },
    { id: "created", header: "Joined", size: 130, hideBelow: "lg", cell: (user) => <span className="text-muted">{formatDate(user.createdAt)}</span> },
  ];
  const userActions: Array<DataTableRowAction<UserFieldsFragment>> = [
    { key: "role", label: (user) => (user.role === "admin" ? "Make member" : "Make admin"), hidden: (user) => user.id === me?.id, onAction: async (user) => { try { assertSuccess((await updateUser({ variables: { id: user.id, input: { role: user.role === "admin" ? "member" : "admin" } } })).data?.updateUser, "Could not update"); } catch (error) { toast.danger(errorMessage(error)); } } },
    { key: "active", label: (user) => (user.isActive ? "Disable" : "Enable"), hidden: (user) => user.id === me?.id, onAction: async (user) => { try { assertSuccess((await updateUser({ variables: { id: user.id, input: { isActive: !user.isActive } } })).data?.updateUser, "Could not update"); } catch (error) { toast.danger(errorMessage(error)); } } },
    { key: "delete", label: "Delete", icon: <IconTrash size={16} />, destructive: true, hidden: (user) => user.id === me?.id, onAction: (user) => setRemoving(user) },
  ];
  const inviteColumns: Array<DataTableColumn<InviteTokenFieldsFragment>> = [
    { id: "code", header: "Invite", cell: (token) => <span className="font-mono text-label text-foreground">{token.id}</span> },
    { id: "role", header: "Role", size: 100, cell: (token) => <span className="capitalize text-muted">{token.role}</span> },
    { id: "uses", header: "Uses", size: 90, align: "end", numeric: true, cell: (token) => `${token.useCount}/${token.maxUses ?? "∞"}` },
    { id: "expires", header: "Expires", size: 140, cell: (token) => <span className="text-muted">{token.expiresAt ? formatRelative(token.expiresAt) : "Never"}</span> },
  ];
  const inviteActions: Array<DataTableRowAction<InviteTokenFieldsFragment>> = [
    { key: "copy", label: "Copy link", icon: <IconCopy size={16} />, onAction: copy },
    { key: "revoke", label: "Revoke", icon: <IconTrash size={16} />, destructive: true, onAction: async (token) => { try { await deleteInvite({ variables: { id: token.id } }); void invites.refetch(); } catch (error) { toast.danger(errorMessage(error)); } } },
  ];

  return (
    <div className="flex flex-col gap-6">
      <Panel title="Users" flush actions={<Button size="sm" variant="primary" onPress={() => setInviting(true)}><IconPlus size={16} /> Invite</Button>}>
        <DataTable<UserFieldsFragment> className="px-4 pb-4" frame={false} columns={userColumns} rows={userRows} getRowId={(user) => user.id} isLoading={users.loading && userRows.length === 0} rowActions={userActions} noun="users" emptyState={<EmptyState compact icon={IconUsers} title="No users" />} />
      </Panel>
      <Panel title="Open invites" description="Share a link; it stops working after the last use or when it expires." flush>
        <DataTable<InviteTokenFieldsFragment> className="px-4 pb-4" frame={false} columns={inviteColumns} rows={inviteRows} getRowId={(token) => token.id} isLoading={invites.loading && inviteRows.length === 0} rowActions={inviteActions} density="compact" noun="invites" emptyState={<EmptyState compact icon={IconTicket} title="No open invites" />} />
      </Panel>

      <Dialog isOpen={inviting} onOpenChange={setInviting} title="Invite someone" size="sm" footer={<><Button variant="ghost" onPress={() => setInviting(false)}>Cancel</Button><Button variant="primary" onPress={() => void submitInvite()} isPending={creatingInvite}>Create and copy link</Button></>}>
        <form onSubmit={submitInvite} noValidate>
          <FieldGroup>
            <FormSelectField control={inviteForm.control} name="role" label="Role" options={[{ key: "member", label: "Member", description: "Browse and play" }, { key: "admin", label: "Admin", description: "Manage libraries, sources and users" }]} />
            <FormSelectField control={inviteForm.control} name="accessLevel" label="Library access" options={[{ key: "full", label: "All libraries" }, { key: "restricted", label: "Restricted" }]} />
            <FormNumberField control={inviteForm.control} name="maxUses" label="Maximum uses" min={1} max={100} />
            <FormNumberField control={inviteForm.control} name="expiresInDays" label="Expires in (days)" min={1} max={365} />
          </FieldGroup>
        </form>
      </Dialog>
      <ConfirmDialog isOpen={Boolean(removing)} onOpenChange={(open) => !open && setRemoving(null)} title={`Delete ${removing?.displayName ?? removing?.username}?`} description="Their playback history is removed. Libraries and media are kept." confirmLabel="Delete user" destructive isPending={deletingUser} onConfirm={async () => { if (!removing) return; try { assertSuccess((await deleteUser({ variables: { id: removing.id } })).data?.deleteUser, "Could not delete"); setRemoving(null); void users.refetch(); } catch (error) { toast.danger(errorMessage(error)); } }} />
    </div>
  );
}
