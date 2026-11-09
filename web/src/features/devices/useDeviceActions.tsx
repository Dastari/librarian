import { useMutation } from "@apollo/client/react";
import { defineActions, type ActionSet } from "glass-ux/actions";
import { IconEdit, IconLogOut, IconSettings, IconTrash } from "glass-ux/icons";
import { useMemo } from "react";

import { EntityDeviceDeleteDocument, SignOutDeviceDocument } from "@/graphql/generated/graphql";
import { assertSuccess } from "@/lib/graphql/errors";

import { currentDevice } from "./clientDevice";
import type { DeviceEntity } from "./DeviceIdentity";

export type DeviceSettingsTab = "general" | "details" | "remove";

/**
 * Everything that can be done to a device, wherever it is shown: its settings dialog's header, a
 * table row's menu. Signing out ends its login and whatever it is playing; forgetting also removes
 * it from the list until it signs in again. Your own devices, or anyone's for an administrator
 * (the server checks).
 */
export function useDeviceActions<T extends DeviceEntity>({ settings, onChanged }: {
  settings: (device: T, tab?: DeviceSettingsTab) => void;
  onChanged?: () => void;
}): ActionSet<T> {
  const [signOut] = useMutation(SignOutDeviceDocument);
  const [forget] = useMutation(EntityDeviceDeleteDocument);
  return useMemo(() => defineActions<T>("device", [
    {
      id: "settings", label: "Device settings", icon: IconSettings, group: "open", priority: 1,
      visible: (rows) => rows.length === 1,
      run: (rows) => settings(rows[0]!),
    },
    {
      id: "rename", label: "Rename", icon: IconEdit, group: "manage", priority: 2,
      visible: (rows) => rows.length === 1,
      run: (rows) => settings(rows[0]!, "general"),
    },
    {
      id: "sign-out", label: (rows) => (rows.length === 1 ? "Sign out" : `Sign out ${rows.length} devices`), icon: IconLogOut, group: "manage", priority: 3, bulk: true,
      disabledReason: (rows) => (rows.every((row) => row.signedOutAt) ? "Already signed out" : false),
      confirm: (rows) => ({
        title: rows.length === 1 ? `Sign out ${rows[0]!.name}?` : `Sign out ${rows.length} devices?`,
        body: rows.some((row) => row.id === currentDevice.get())
          ? "This includes the device you are using: you will be asked to sign in again."
          : "It has to sign in again, and anything playing on it stops.",
        confirmLabel: "Sign out",
        tone: "danger",
      }),
      run: async (rows) => {
        for (const row of rows) assertSuccess((await signOut({ variables: { id: row.id } })).data?.signOutDevice, `${row.name} could not be signed out`);
        onChanged?.();
      },
      successMessage: (rows) => (rows.length === 1 ? `${rows[0]!.name} signed out` : `${rows.length} devices signed out`),
    },
    {
      id: "forget", label: (rows) => (rows.length === 1 ? "Forget device" : `Forget ${rows.length} devices`), icon: IconTrash, group: "danger", priority: 9, bulk: true,
      confirm: (rows) => ({
        title: rows.length === 1 ? `Forget ${rows[0]!.name}?` : `Forget ${rows.length} devices?`,
        body: "It is signed out and removed from this list. Its plays stay in the history under its name. It comes back if it signs in again.",
        confirmLabel: "Forget",
        tone: "danger",
      }),
      run: async (rows) => {
        for (const row of rows) assertSuccess((await forget({ variables: { id: row.id } })).data?.deleteDevice, `${row.name} could not be forgotten`);
        onChanged?.();
      },
      successMessage: (rows) => (rows.length === 1 ? "Device forgotten" : `${rows.length} devices forgotten`),
    },
  ]), [forget, onChanged, settings, signOut]);
}
