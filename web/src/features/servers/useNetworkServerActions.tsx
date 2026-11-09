import { useMutation } from "@apollo/client/react";
import { defineActions, type ActionSet } from "glass-ux/actions";
import { IconSettings, IconTrash } from "glass-ux/icons";
import { IconPlugConnected } from "@tabler/icons-react";
import { toast } from "glass-ux/toast";
import { useMemo } from "react";

import { EntityNetworkServerDeleteDocument, TestNetworkServerDocument } from "@/graphql/generated/graphql";
import { glyph } from "@/lib/entities/kinds";
import { assertSuccess } from "@/lib/graphql/errors";
import { pluralize } from "@/lib/format";

import type { NetworkServerEntity } from "./NetworkServerIdentity";

const TestIcon = glyph(IconPlugConnected, "plug-connected");

export type NetworkServerSettingsTab = "connection" | "shares" | "remove";

/** What a test of a saved server says, in one line. */
export function testSummary(result: { success: boolean; error?: string | null; shares: ReadonlyArray<{ listed: boolean }>; sharesError?: string | null }): string {
  if (!result.success) return result.error ?? "The server did not answer";
  const listed = result.shares.filter((share) => share.listed).length;
  if (result.sharesError) return `Signed in. ${result.sharesError}`;
  return `Signed in. ${pluralize(listed, "share")} offered.`;
}

/**
 * Everything that can be done to a saved server, wherever it is shown: its settings dialog's
 * header, a table row's menu. Removing one that a library or the download folder still uses is
 * refused by the server, which says what uses it.
 */
export function useNetworkServerActions<T extends NetworkServerEntity>({ settings, onChanged }: {
  settings: (server: T, tab?: NetworkServerSettingsTab) => void;
  onChanged?: () => void;
}): ActionSet<T> {
  const [test] = useMutation(TestNetworkServerDocument);
  const [remove] = useMutation(EntityNetworkServerDeleteDocument);
  return useMemo(() => defineActions<T>("network-server", [
    {
      id: "settings", label: "Server settings", icon: IconSettings, group: "open", priority: 1,
      visible: (rows) => rows.length === 1,
      run: (rows) => settings(rows[0]!),
    },
    {
      id: "test", label: (rows) => (rows.length === 1 ? "Test connection" : `Test ${rows.length} servers`), icon: TestIcon, group: "manage", priority: 1, bulk: true, primary: true,
      run: async (rows) => {
        for (const row of rows) {
          const result = (await test({ variables: { input: { id: row.id } } })).data?.testNetworkServer;
          if (!result) throw new Error(`${row.name}: the test did not run`);
          const line = `${row.name}: ${testSummary(result)}`;
          if (result.success) toast.success(line);
          else toast.warning(line);
        }
        onChanged?.();
      },
    },
    {
      id: "remove", label: (rows) => (rows.length === 1 ? "Remove server" : `Remove ${rows.length} servers`), icon: IconTrash, group: "danger", priority: 9, bulk: true,
      confirm: (rows) => ({
        title: rows.length === 1 ? `Remove ${rows[0]!.name}?` : `Remove ${rows.length} servers?`,
        body: "Its saved sign-in is deleted. A server that a library or the download folder still uses cannot be removed until they are moved off it.",
        confirmLabel: "Remove",
        tone: "danger",
      }),
      run: async (rows) => {
        for (const row of rows) {
          assertSuccess((await remove({ variables: { id: row.id } })).data?.deleteNetworkServer, `${row.name} could not be removed`);
        }
        onChanged?.();
      },
      successMessage: (rows) => (rows.length === 1 ? "Server removed" : `${rows.length} servers removed`),
    },
  ]), [onChanged, remove, settings, test]);
}
