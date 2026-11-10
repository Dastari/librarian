import { useMutation } from "@apollo/client/react";
import { EditableAvatar } from "glass-ux/avatar";
import { emailSchema } from "glass-ux/email-field";
import { EntitySettingsDialog, type EntitySettingsTab } from "glass-ux/entity-settings";
import { formBehaviour, useAppForm } from "glass-ux/form";
import { IconChartLine, IconHistory, IconLock, IconSliders, IconUser } from "glass-ux/icons";
import { phoneSchema } from "glass-ux/phone-field";
import { SettingRow } from "glass-ux/setting-row";
import { toast } from "glass-ux/toast";
import { useEffect, useMemo, useState } from "react";
import { z } from "zod";

import { Button, ConfirmDialog } from "@/components/ui";
import { DevicesPage } from "@/features/statistics/DevicesPage";
import { HistoryPage } from "@/features/statistics/HistoryPage";
import { StatisticsPage, type StatisticsSearch } from "@/features/statistics/StatisticsPage";
import { EntityUserGetDocument, EntityUserUpdateDocument, GenreChoicesDocument, ResetUserTwoFactorDocument, RevertArtworkDocument, type UserFieldsFragment } from "@/graphql/generated/graphql";
import { withoutAction } from "@/lib/actions/withoutAction";
import { bumpArtwork } from "@/lib/artworkRevisions";
import { session } from "@/lib/auth/session";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { formatDateTime, formatRelative } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

import { ChangePasswordDialog } from "./ChangePasswordDialog";
import { TwoFactorPasswordDialog, TwoFactorSetupDialog } from "./TwoFactorDialogs";
import { useUserActions, type UserSettingsTab } from "./useUserActions";
import { userAvatarSrc, userIdentityProps, userName } from "./UserIdentity";

const MAX_GENRES = 12;

const schema = z.object({
  displayName: z.string().trim().max(80, "Use at most 80 characters"),
  email: emailSchema(),
  phone: phoneSchema(),
  genreInterests: z.array(z.string()).max(MAX_GENRES, `Pick at most ${MAX_GENRES} genres`),
});
type Values = z.input<typeof schema>;

const valuesOf = (user: UserFieldsFragment): Values => ({
  displayName: user.displayName ?? "",
  email: user.email ?? "",
  phone: user.phone ?? "",
  genreInterests: user.genreInterests ?? [],
});

/** A fact about the account, read-only. */
function Fact({ label, value }: { label: string; value: string | null | undefined }) {
  return <SettingRow label={label} control={<span className="text-body-sm">{value || "—"}</span>} />;
}

/** Upload the cropped photo as the user's avatar slot (the artwork upload route, design.md Q127). */
async function uploadAvatar(userId: string, blob: Blob, type: string): Promise<void> {
  let response: Response;
  try {
    response = await fetch(`/api/artwork/user/${encodeURIComponent(userId)}/avatar`, { method: "PUT", credentials: "same-origin", headers: { "content-type": type || "image/png" }, body: blob });
  } catch {
    throw new Error("The photo couldn't be uploaded. Check the connection and try again.");
  }
  let error: string | undefined;
  try {
    const body = (await response.json()) as { success?: boolean; error?: string | null };
    if (response.ok && body.success) return;
    error = body.error ?? undefined;
  } catch {
    // A proxy's page or an empty answer: fall back on the status.
  }
  throw new Error(error ?? `The photo couldn't be uploaded (HTTP ${response.status})`);
}

/**
 * A person's settings, which for yourself is your profile (design.md Q147): your photo, name and
 * contact details; your password, two-factor sign-in and devices; the genres Home shows rows for;
 * and your statistics and play history. An administrator opening someone else's sees their profile,
 * account recovery (a temporary password, a reset link, turning off two-factor sign-in), their
 * devices, statistics and history. The identity and its actions sit in the header.
 */
export function UserSettingsDialog({ userId, open, onOpenChange, tab, onTabChange, onChanged }: {
  userId: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  tab: UserSettingsTab;
  onTabChange: (tab: UserSettingsTab) => void;
  onChanged?: () => void;
}) {
  const { user: me } = useSession();
  const isAdmin = useIsAdmin();
  const self = me?.id === userId;
  const query = useQuery(EntityUserGetDocument, { variables: { id: userId }, skip: !open });
  const user = (query.data ?? query.previousData)?.user ?? null;
  const genres = useQuery(GenreChoicesDocument, { skip: !open || !self });
  const [update] = useMutation(EntityUserUpdateDocument);
  const [revert] = useMutation(RevertArtworkDocument);
  const [resetTwoFactor, { loading: resetting }] = useMutation(ResetUserTwoFactorDocument);
  const [period, setPeriod] = useState<StatisticsSearch>({});
  const [dialog, setDialog] = useState<null | "password" | "two-factor" | "disable" | "codes" | "reset-two-factor">(null);

  const refresh = () => {
    void query.refetch();
    if (self) void session.reconcile();
    onChanged?.();
  };

  const form = useAppForm({
    ...formBehaviour,
    defaultValues: (user ? valuesOf(user) : { displayName: "", email: "", phone: "", genreInterests: [] }) as Values,
    validators: { onDynamic: schema },
    onSubmit: async ({ value, formApi }) => {
      const saved = schema.parse(value);
      const result = assertSuccess((await update({
        variables: {
          id: userId,
          input: {
            displayName: saved.displayName || null,
            email: saved.email || null,
            phone: saved.phone || null,
            ...(self ? { genreInterests: saved.genreInterests } : {}),
          },
        },
      })).data?.updateUser, "The profile could not be saved");
      if (result.user) formApi.reset(valuesOf(result.user));
      refresh();
    },
  });
  // Take the server's values whenever the dialog opens or another person is shown.
  useEffect(() => { if (open && user) form.reset(valuesOf(user)); }, [open, user?.id, user?.updatedAt]); // eslint-disable-line react-hooks/exhaustive-deps

  const userActions = useUserActions<UserFieldsFragment>({ settings: (_, next) => onTabChange(next ?? "profile"), onChanged: refresh });
  const headerActions = useMemo(() => withoutAction(userActions.actions, "settings"), [userActions.actions]);
  const genreOptions = useMemo(() => {
    const choices = genres.data?.genreChoices ?? [];
    const known = new Set(choices.map((choice) => choice.genre.toLowerCase()));
    // Interests kept from titles that have since gone stay choosable, so saving keeps them.
    const kept = (user?.genreInterests ?? []).filter((genre) => !known.has(genre.toLowerCase())).map((genre) => ({ value: genre, label: genre }));
    return [...choices.map((choice) => ({ value: choice.genre, label: choice.genre, description: [choice.movies ? `${choice.movies} ${choice.movies === 1 ? "movie" : "movies"}` : "", choice.shows ? `${choice.shows} ${choice.shows === 1 ? "show" : "shows"}` : ""].filter(Boolean).join(" · ") })), ...kept];
  }, [genres.data, user?.genreInterests]);

  if (!user) {
    return <EntitySettingsDialog open={open} onOpenChange={onOpenChange} form={form} loading title={self ? "Your profile" : "Account"} tabs={[{ value: "profile", label: "Profile", icon: <IconUser />, content: null }]} />;
  }
  const name = userName(user);
  const canEdit = self || isAdmin;
  const twoFactorOn = Boolean(user.twoFactorEnabledAt);
  const DevicesIcon = ENTITY_KINDS.device.icon;

  const tabs: EntitySettingsTab[] = [
    {
      value: "profile", label: "Profile", icon: <IconUser />, fields: ["displayName", "email", "phone"],
      content: (
        <div className="flex flex-col gap-3">
          <div className="flex items-center gap-4" data-nav-section>
            <EditableAvatar
              name={name}
              src={userAvatarSrc(user, 192)}
              size="xl"
              disabled={!canEdit}
              requirements={{ maxSize: 20 * 1024 * 1024, minWidth: 100, minHeight: 100 }}
              onSave={async (change) => {
                await uploadAvatar(user.id, change.blob, change.type);
                bumpArtwork("user", user.id);
                refresh();
              }}
              onRemove={user.avatarUrl ? async () => {
                assertSuccess((await revert({ variables: { entityType: "USER", entityId: user.id, kind: "AVATAR" } })).data?.revertArtwork, "The photo could not be removed");
                bumpArtwork("user", user.id);
                refresh();
              } : undefined}
            />
            <p className="min-w-0 text-body-sm text-muted">{self ? "Your photo shows on your account and wherever you are named." : `${name}'s photo.`}</p>
          </div>
          <form.AppField name="displayName">{(field) => <field.TextField label="Display name" hint={`Shown instead of ${user.username}.`} maxLength={80} disabled={!canEdit} />}</form.AppField>
          <form.AppField name="email">{(field) => <field.EmailField label="Email" optional hint="You can sign in with it. Nothing sends mail yet." disabled={!canEdit} />}</form.AppField>
          <form.AppField name="phone">{(field) => <field.PhoneField label="Phone" optional disabled={!canEdit} />}</form.AppField>
          <div className="flex flex-col">
            <Fact label="Username" value={user.username} />
            <Fact label="Role" value={user.role === "admin" ? "Admin" : "Member"} />
            <Fact label="Joined" value={formatDateTime(user.createdAt)} />
            <Fact label="Last sign-in" value={user.lastLoginAt ? formatDateTime(user.lastLoginAt) : "Never"} />
          </div>
        </div>
      ),
    },
    {
      value: "security", label: "Security", icon: <IconLock />,
      content: self ? (
        <div className="flex flex-col">
          <SettingRow
            label="Password"
            description={user.passwordChangedAt ? `Changed ${formatRelative(user.passwordChangedAt)}.` : "Changing it signs out your other devices."}
            control={<Button variant="secondary" size="sm" onPress={() => setDialog("password")}>Change password…</Button>}
          />
          <SettingRow
            label="Two-factor sign-in"
            description={twoFactorOn
              ? `On since ${formatDateTime(user.twoFactorEnabledAt)}. ${user.recoveryCodesRemaining} recovery ${user.recoveryCodesRemaining === 1 ? "code" : "codes"} left.`
              : "Off. Signing in can also ask for a code from an authenticator app."}
            control={twoFactorOn ? (
              <div className="flex flex-wrap justify-end gap-2">
                <Button variant="secondary" size="sm" onPress={() => setDialog("codes")}>New recovery codes…</Button>
                <Button variant="danger-soft" size="sm" onPress={() => setDialog("disable")}>Turn off…</Button>
              </div>
            ) : <Button variant="primary" size="sm" onPress={() => setDialog("two-factor")}>Turn on…</Button>}
          />
          {user.mustChangePassword ? <p className="text-body-sm text-warning">Your password is temporary. Change it now.</p> : null}
        </div>
      ) : (
        <div className="flex flex-col">
          <SettingRow
            label="Password"
            description={user.mustChangePassword ? "Temporary: they choose a new one at their next sign-in." : user.passwordChangedAt ? `Changed ${formatRelative(user.passwordChangedAt)}.` : "Set by them."}
            control={isAdmin ? (
              <div className="flex flex-wrap justify-end gap-2">
                <Button variant="secondary" size="sm" onPress={() => userActions.temporaryPassword(user)}>Temporary password…</Button>
                <Button variant="secondary" size="sm" onPress={() => userActions.resetLink(user)}>Reset link…</Button>
              </div>
            ) : null}
          />
          <SettingRow
            label="Two-factor sign-in"
            description={twoFactorOn ? `On since ${formatDateTime(user.twoFactorEnabledAt)}.` : "Off."}
            control={isAdmin && twoFactorOn ? <Button variant="danger-soft" size="sm" onPress={() => setDialog("reset-two-factor")}>Turn off…</Button> : null}
          />
        </div>
      ),
    },
    {
      value: "devices", label: "Devices", icon: <DevicesIcon />,
      content: <div className="flex h-[min(34rem,60vh)] min-h-0 flex-col"><DevicesPage scope="personal" userId={user.id} /></div>,
    },
    ...(self ? [{
      value: "preferences", label: "Preferences", icon: <IconSliders />, fields: ["genreInterests"],
      content: (
        <div className="flex flex-col gap-3">
          <form.AppField name="genreInterests">
            {(field) => <field.MultiSelectField label="Genres you like" hint="Home shows a row for each, newest first." options={genreOptions} placeholder={genres.loading ? "Loading genres…" : "Choose genres"} wrap />}
          </form.AppField>
        </div>
      ),
    }] : []),
    {
      value: "statistics", label: "Statistics", icon: <IconChartLine />,
      content: (
        <StatisticsPage
          scope="personal"
          period={period.period ?? "30d"}
          person={period.user}
          fixedPerson={self ? undefined : user.id}
          onSearchChange={(next) => setPeriod((previous) => ({ ...previous, ...next }))}
        />
      ),
    },
    {
      value: "history", label: "History", icon: <IconHistory />,
      content: <div className="flex h-[min(34rem,60vh)] min-h-0 flex-col"><HistoryPage scope="personal" userId={user.id} /></div>,
    },
  ];
  const shown = tabs.some((entry) => entry.value === tab) ? tab : "profile";

  return (
    <>
      <EntitySettingsDialog
        open={open}
        onOpenChange={onOpenChange}
        form={form}
        size="xl"
        entityName={name}
        identity={userIdentityProps(user, { actions: headerActions, self })}
        tabs={tabs}
        tab={shown}
        onTabChange={(next) => onTabChange(next as UserSettingsTab)}
        saveDisabledReason={canEdit ? undefined : "Only administrators can change someone else's profile."}
        onSaveError={(error) => toast.danger(errorMessage(error, "Could not save the profile"))}
      />
      {userActions.dialogs}
      {dialog === "password" ? <ChangePasswordDialog open onOpenChange={(next) => !next && setDialog(null)} onChanged={refresh} userInputs={[user.username, name, user.email ?? ""].filter(Boolean)} /> : null}
      {dialog === "two-factor" ? <TwoFactorSetupDialog open onOpenChange={(next) => !next && setDialog(null)} onChanged={refresh} /> : null}
      {dialog === "disable" || dialog === "codes" ? <TwoFactorPasswordDialog purpose={dialog === "disable" ? "disable" : "codes"} open onOpenChange={(next) => !next && setDialog(null)} onChanged={refresh} /> : null}
      <ConfirmDialog
        isOpen={dialog === "reset-two-factor"}
        onOpenChange={(next) => !next && setDialog(null)}
        title={`Turn off two-factor sign-in for ${name}?`}
        description="For when they have lost their authenticator and their recovery codes. They sign in with their password alone until they turn it on again. This is recorded in the server log."
        confirmLabel="Turn off"
        destructive
        isPending={resetting}
        onConfirm={async () => {
          try {
            assertSuccess((await resetTwoFactor({ variables: { userId: user.id } })).data?.resetUserTwoFactor, "Two-factor sign-in could not be turned off");
            toast.success(`Two-factor sign-in is off for ${name}`);
            refresh();
          } catch (error) {
            toast.danger(errorMessage(error));
          } finally {
            setDialog(null);
          }
        }}
      />
    </>
  );
}
