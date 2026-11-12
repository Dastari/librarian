import { useMutation } from "@apollo/client/react";
import { defineActions, type ActionSet } from "glass-ux/actions";
import { copyText } from "glass-ux/copy-button";
import { IconChartLine, IconCopy, IconKey, IconSettings, IconTrash } from "glass-ux/icons";
import { IconLink, IconShieldCheck, IconShieldOff, IconUserOff } from "@tabler/icons-react";
import { useMemo, useState, type ReactNode } from "react";

import { runSettingsSelection } from "@/features/settings/runSettingsSelection";
import { EntityUserDeleteDocument, EntityUserUpdateDocument, ResetUserTwoFactorDocument } from "@/graphql/generated/graphql";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { glyph } from "@/lib/entities/kinds";
import { assertSuccess } from "@/lib/graphql/errors";

import { ResetLinkDialog, TemporaryPasswordDialog } from "./AccountRecoveryDialogs";
import { userName, type UserEntity } from "./UserIdentity";

/** The profile dialog's tabs (design.md Q147). */
export const USER_SETTINGS_TABS = ["profile", "security", "devices", "preferences", "statistics", "history"] as const;
export type UserSettingsTab = (typeof USER_SETTINGS_TABS)[number];
export const isUserSettingsTab = (value: unknown): value is UserSettingsTab => (USER_SETTINGS_TABS as readonly unknown[]).includes(value);

const RoleIcon = glyph(IconShieldCheck, "shield-check");
const SignInIcon = glyph(IconUserOff, "user-off");
const LinkIcon = glyph(IconLink, "link");
const TwoFactorOffIcon = glyph(IconShieldOff, "shield-off");

/**
 * Everything that can be done to a person's account, wherever they are shown: their settings
 * dialog's header, a table row's menu. Anyone opens their own profile; administrators also change
 * roles and sign-in access, give a temporary password or a reset link, turn off someone's
 * two-factor sign-in (account recovery) and delete accounts. The server checks each.
 *
 * Returns the dialogs the recovery actions open; render them beside whatever shows the actions.
 */
export function useUserActions<T extends UserEntity>({ settings, onChanged }: {
  settings: (user: T, tab?: UserSettingsTab) => void;
  onChanged?: () => void;
}): { actions: ActionSet<T>; dialogs: ReactNode; temporaryPassword: (user: T) => void; resetLink: (user: T) => void } {
  const isAdmin = useIsAdmin();
  const { user: me } = useSession();
  const [update] = useMutation(EntityUserUpdateDocument);
  const [remove] = useMutation(EntityUserDeleteDocument);
  const [resetTwoFactor] = useMutation(ResetUserTwoFactorDocument);
  const [temporary, setTemporary] = useState<T | null>(null);
  const [linkFor, setLinkFor] = useState<T | null>(null);
  const others = (rows: readonly T[]) => isAdmin && rows.every((row) => row.id !== me?.id);

  const actions = useMemo(() => defineActions<T>("user", [
    {
      id: "settings", label: (rows) => (rows[0]?.id === me?.id ? "Your profile" : "Profile and settings"), icon: IconSettings, group: "open", priority: 1,
      visible: (rows) => rows.length === 1 && (isAdmin || rows[0]!.id === me?.id),
      run: (rows) => settings(rows[0]!),
    },
    {
      id: "statistics", label: "Statistics", icon: IconChartLine, group: "open", priority: 2,
      visible: (rows) => rows.length === 1 && (isAdmin || rows[0]!.id === me?.id),
      run: (rows) => settings(rows[0]!, "statistics"),
    },
    {
      id: "copy-usernames", label: (rows) => (rows.length === 1 ? "Copy username" : "Copy usernames"), icon: IconCopy, group: "open", bulk: true,
      visible: () => isAdmin,
      run: (rows) => copyText(rows.map((row) => row.username).join("\n")), successMessage: () => "Usernames copied",
    },
    ...(["member", "admin"] as const).map((role) => ({
      id: `make-${role}`, bulk: true, group: "manage" as const, priority: 3, icon: RoleIcon,
      label: (rows: readonly T[]) => (rows.length === 1 ? (role === "admin" ? "Make admin" : "Make member") : role === "admin" ? "Make admins" : "Make members"),
      visible: (rows: readonly T[]) => isAdmin && rows.some((row) => row.role !== role),
      disabledReason: (rows: readonly T[]) => (rows.some((row) => row.id === me?.id) ? "Deselect your own account before changing roles." : false),
      confirm: (rows: readonly T[]) => (role === "member" || rows.length > 1 ? {
        title: rows.length === 1 ? `Make ${userName(rows[0]!)} ${role === "admin" ? "an admin" : "a member"}?` : `Make ${rows.filter((row) => row.role !== role).length} people ${role === "admin" ? "admins" : "members"}?`,
        body: role === "admin" ? "Admins can manage libraries, sources, settings and other people." : "Their administrator access is removed and their sessions are signed out.",
        confirmLabel: role === "admin" ? "Make admin" : "Make member",
        tone: "warning" as const,
      } : null),
      run: (rows: readonly T[]) => runSettingsSelection(rows.filter((row) => row.role !== role), "Change role", userName, async (row) => assertSuccess((await update({ variables: { id: row.id, input: { role } } })).data?.updateUser, "Could not change the role"), onChanged),
      successMessage: () => "Roles updated",
    })),
    ...([true, false] as const).map((active) => ({
      id: active ? "enable" : "disable", bulk: true, group: "manage" as const, priority: 4, icon: SignInIcon,
      label: active ? "Enable sign-in" : "Disable sign-in",
      visible: (rows: readonly T[]) => isAdmin && rows.some((row) => row.isActive !== active),
      disabledReason: (rows: readonly T[]) => (rows.some((row) => row.id === me?.id) ? "Deselect your own account before changing sign-in access." : false),
      confirm: (rows: readonly T[]) => (active ? null : {
        title: rows.length === 1 ? `Disable ${userName(rows[0]!)}?` : `Disable sign-in for ${rows.filter((row) => row.isActive).length} people?`,
        body: "They are signed out and cannot sign in until enabled again. Their history is kept.",
        confirmLabel: "Disable",
        tone: "warning" as const,
      }),
      run: (rows: readonly T[]) => runSettingsSelection(rows.filter((row) => row.isActive !== active), active ? "Enable sign-in" : "Disable sign-in", userName, async (row) => assertSuccess((await update({ variables: { id: row.id, input: { isActive: active } } })).data?.updateUser, "Could not change sign-in access"), onChanged),
      successMessage: () => (active ? "Sign-in enabled" : "Sign-in disabled"),
    })),
    {
      id: "temporary-password", label: "Set a temporary password…", icon: IconKey, group: "manage", priority: 5,
      visible: (rows) => rows.length === 1 && others(rows),
      run: (rows) => setTemporary(rows[0]!),
    },
    {
      id: "reset-link", label: "Make a reset link…", icon: LinkIcon, group: "manage", priority: 6,
      visible: (rows) => rows.length === 1 && others(rows),
      run: (rows) => setLinkFor(rows[0]!),
    },
    {
      id: "reset-two-factor", label: "Turn off two-factor sign-in", icon: TwoFactorOffIcon, group: "manage", priority: 7,
      visible: (rows) => rows.length === 1 && others(rows) && Boolean(rows[0]!.twoFactorEnabledAt),
      confirm: (rows) => ({
        title: `Turn off two-factor sign-in for ${userName(rows[0]!)}?`,
        body: "For when they have lost their authenticator and their recovery codes. They sign in with their password alone until they turn it on again. This is recorded in the server log.",
        confirmLabel: "Turn off",
        tone: "danger",
      }),
      run: async (rows) => {
        assertSuccess((await resetTwoFactor({ variables: { userId: rows[0]!.id } })).data?.resetUserTwoFactor, "Two-factor sign-in could not be turned off");
        onChanged?.();
      },
      successMessage: (rows) => `Two-factor sign-in is off for ${userName(rows[0]!)}`,
    },
    {
      id: "delete", label: (rows) => (rows.length === 1 ? "Delete account" : `Delete ${rows.length} accounts`), icon: IconTrash, group: "danger", priority: 9, bulk: true,
      visible: (rows) => isAdmin && (rows.length > 1 || rows[0]!.id !== me?.id),
      disabledReason: (rows) => (rows.some((row) => row.id === me?.id) ? "Deselect your own account before deleting accounts." : false),
      confirm: (rows) => ({
        title: rows.length === 1 ? `Delete ${userName(rows[0]!)}?` : `Delete ${rows.length} accounts?`,
        body: "Their playback history, devices and lists are removed. Libraries and media are kept.",
        confirmLabel: rows.length === 1 ? "Delete account" : "Delete accounts",
        tone: "danger",
      }),
      run: (rows) => runSettingsSelection(rows, "Delete", userName, async (row) => assertSuccess((await remove({ variables: { id: row.id } })).data?.deleteUser, "Could not delete the account"), onChanged),
      successMessage: (rows) => (rows.length === 1 ? "Account deleted" : `${rows.length} accounts deleted`),
    },
  ]), [isAdmin, me?.id, onChanged, remove, resetTwoFactor, settings, update]); // eslint-disable-line react-hooks/exhaustive-deps

  const dialogs = (
    <>
      {temporary ? <TemporaryPasswordDialog user={temporary} open onOpenChange={(open) => !open && setTemporary(null)} onChanged={onChanged} /> : null}
      {linkFor ? <ResetLinkDialog user={linkFor} open onOpenChange={(open) => !open && setLinkFor(null)} /> : null}
    </>
  );
  return { actions, dialogs, temporaryPassword: setTemporary, resetLink: setLinkFor };
}
