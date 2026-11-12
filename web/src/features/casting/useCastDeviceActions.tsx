import { useMutation } from "@apollo/client/react";
import { defineActions, type ActionDefinition, type ActionSet } from "glass-ux/actions";
import { copyText } from "glass-ux/copy-button";
import { IconCopy, IconEdit, IconInfo, IconSettings, IconStar, IconTrash } from "glass-ux/icons";
import { IconPower } from "@tabler/icons-react";
import { useMemo } from "react";

import { EntityCastDeviceDeleteDocument, EntityCastDeviceUpdateDocument } from "@/graphql/generated/graphql";
import { glyph } from "@/lib/entities/kinds";
import { assertSuccess } from "@/lib/graphql/errors";

import type { CastDevice } from "./castDevice";

export type CastDeviceSettingsTab = "general" | "discovery" | "remove";

const PowerIcon = glyph(IconPower, "power");
const plural = (rows: readonly unknown[], one: string, other: string) => (rows.length === 1 ? one : other.replace("#", String(rows.length)));

/**
 * Everything that can be done to a cast receiver, wherever it is shown: its settings dialog's
 * header, a table row's menu. Anyone signed in can see how a receiver was found and copy its
 * address; changing or removing one is for administrators (the server checks too).
 */
export function useCastDeviceActions<T extends CastDevice>({ isAdmin, open, onChanged }: {
  isAdmin: boolean;
  open: (device: T, tab?: CastDeviceSettingsTab) => void;
  onChanged?: () => void;
}): ActionSet<T> {
  const [update] = useMutation(EntityCastDeviceUpdateDocument);
  const [remove] = useMutation(EntityCastDeviceDeleteDocument);
  return useMemo(() => {
    const save = async (rows: readonly T[], input: { isFavorite?: boolean; enabled?: boolean }) => {
      for (const row of rows) assertSuccess((await update({ variables: { id: row.id, input } })).data?.updateCastDevice, `${row.name} could not be changed`);
      onChanged?.();
    };
    const everyone: ActionDefinition<T>[] = [
      {
        id: "details", label: "How it was found", icon: IconInfo, group: "open", priority: 1,
        visible: (rows) => rows.length === 1,
        run: (rows) => open(rows[0]!, "discovery"),
      },
      {
        id: "copy-address", label: (rows) => plural(rows, "Copy address", "Copy # addresses"), icon: IconCopy, group: "open", priority: 4, bulk: true,
        run: (rows) => copyText(rows.map((row) => (rows.length === 1 ? `${row.address}:${row.port}` : `${row.name}: ${row.address}:${row.port}`)).join("\n")),
        successMessage: (rows) => plural(rows, "Address copied", "Addresses copied"),
      },
    ];
    if (!isAdmin) return defineActions<T>("cast-device", everyone);
    return defineActions<T>("cast-device", [
      ...everyone,
      {
        id: "settings", label: "Device settings", icon: IconSettings, group: "open", priority: 2,
        visible: (rows) => rows.length === 1,
        run: (rows) => open(rows[0]!, "general"),
      },
      {
        id: "rename", label: "Rename", icon: IconEdit, group: "manage", priority: 3,
        visible: (rows) => rows.length === 1,
        run: (rows) => open(rows[0]!, "general"),
      },
      {
        id: "favourite", label: (rows) => (rows.every((row) => row.isFavorite) ? "Remove from favourites" : "Make favourite"), icon: IconStar, group: "organise", priority: 2, bulk: true,
        run: (rows) => { const isFavorite = !rows.every((row) => row.isFavorite); return save(rows.filter((row) => row.isFavorite !== isFavorite), { isFavorite }); },
        successMessage: (rows) => (rows.every((row) => row.isFavorite) ? "Removed from favourites" : "Made favourite"),
      },
      {
        id: "power", label: (rows) => (rows.every((row) => row.enabled === false) ? plural(rows, "Turn on", "Turn on # devices") : plural(rows, "Turn off", "Turn off # devices")), icon: PowerIcon, group: "manage", priority: 4, bulk: true,
        run: (rows) => { const enabled = rows.every((row) => row.enabled === false); return save(rows.filter((row) => (row.enabled !== false) !== enabled), { enabled }); },
        successMessage: (rows) => (rows.every((row) => row.enabled === false) ? "Turned on" : "Turned off"),
      },
      {
        id: "remove", label: (rows) => plural(rows, "Remove", "Remove # devices"), icon: IconTrash, group: "danger", priority: 9, bulk: true,
        confirm: (rows) => ({
          title: rows.length === 1 ? `Remove ${rows[0]!.name}?` : `Remove ${rows.length} devices?`,
          body: rows.every((row) => row.isManual && row.discoveryOrigin !== "NETWORK")
            ? "Anything casting to it stops. It is forgotten until you add it again."
            : "Anything casting to it stops. If it is still on the network, discovery finds it again, without its name or favourite.",
          confirmLabel: "Remove",
          tone: "danger",
        }),
        run: async (rows) => {
          for (const row of rows) assertSuccess((await remove({ variables: { id: row.id } })).data?.deleteCastDevice, `${row.name} could not be removed`);
          onChanged?.();
        },
        successMessage: (rows) => plural(rows, "Device removed", "# devices removed"),
      },
    ]);
  }, [isAdmin, onChanged, open, remove, update]);
}
