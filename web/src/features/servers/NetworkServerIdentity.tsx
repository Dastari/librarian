import type { ActionSet } from "glass-ux/actions";
import { EntityIdentity, type EntityIdentityProps, type EntityState } from "glass-ux/entity";
import type { ReactNode } from "react";

import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { formatRelative, pluralize } from "@/lib/format";

/** What the identity needs of a saved server (the `NetworkServerFields` fragment has it all). */
export interface NetworkServerEntity {
  id: string;
  name: string;
  host: string;
  port?: number | null;
  username?: string | null;
  hasPassword: boolean;
  knownShares: string[];
  status?: string | null;
  lastError?: string | null;
  lastCheckedAt?: string | null;
}

/** `host` or `host:port`, as typed. */
export const serverAddress = (server: Pick<NetworkServerEntity, "host" | "port">) => (server.port ? `${server.host}:${server.port}` : server.host);

/** Whether the server answered the last time it was tried. */
export function networkServerState(server: NetworkServerEntity): EntityState {
  if (server.status === "failing") return { id: "failing", label: "Not answering", tone: "danger" };
  if (server.status === "ok") return { id: "ok", label: "Connected", tone: "success" };
  return { id: "untested", label: "Not tested yet", tone: "neutral" };
}

/** Who it signs in as: the user, or a guest. */
export const signInLabel = (server: Pick<NetworkServerEntity, "username">) => (server.username ? `Signs in as ${server.username}` : "Guest sign-in");

export function networkServerIdentityProps<T extends NetworkServerEntity>(server: T, { actions }: { actions?: ActionSet<T> } = {}): EntityIdentityProps<T> {
  const failing = server.status === "failing";
  const shares = server.knownShares.length;
  return {
    kind: ENTITY_KINDS.networkServer.noun,
    name: server.name,
    icon: <ENTITY_KINDS.networkServer.icon />,
    state: networkServerState(server),
    facts: failing && server.lastError
      ? [server.lastError]
      : [
          server.name === server.host ? signInLabel(server) : `${serverAddress(server)} · ${signInLabel(server)}`,
          shares > 0 ? pluralize(shares, "share") : "",
          server.lastCheckedAt ? `Checked ${formatRelative(server.lastCheckedAt)}` : "",
        ],
    target: server,
    actions,
  };
}

/**
 * A saved file server wherever it is named: the server glyph with a dot for whether it answered,
 * then its address, who it signs in as and how many shares are known. While it fails the second
 * line is the reason.
 */
export function NetworkServerIdentity<T extends NetworkServerEntity>({ server, tier = "row", actions, headerActions, heading, className, onOpen }: {
  server: T;
  tier?: "header" | "compact" | "row" | "chip" | "icon" | "auto";
  actions?: ActionSet<T>;
  headerActions?: ReactNode;
  heading?: "h1" | "h2" | "h3" | "h4" | "p";
  className?: string;
  onOpen?: () => void;
}) {
  return (
    <EntityIdentity
      {...networkServerIdentityProps(server, { actions })}
      tier={tier}
      heading={heading}
      className={className}
      slots={headerActions ? { actions: headerActions } : undefined}
      onOpen={onOpen}
    />
  );
}
