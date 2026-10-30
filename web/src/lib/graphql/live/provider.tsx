import { useApolloClient } from "@apollo/client/react";
import { useEffect, useLayoutEffect, useMemo, useState, type ReactNode } from "react";

import { SessionChangedDocument } from "@/graphql/generated/graphql";
import { session } from "@/lib/auth/session";
import { useSession } from "@/lib/auth/useSession";
import { isForbiddenError, isUnauthorizedError } from "@/lib/graphql/errors";
import { router } from "@/app/router";

import { apolloLiveSource } from "./documents";
import { LiveContext } from "./hooks";
import { LiveRegistry } from "./registry";
import { liveTransport } from "./transport";

export function LiveDataProvider({ children }: { children: ReactNode }) {
  const client = useApolloClient();
  const { status, user } = useSession();
  const principal = status === "authenticated" && user ? `${user.id}:${user.role}:${session.getEpoch()}` : status;
  const registry = useMemo(() => new LiveRegistry(apolloLiveSource(client, user?.role.toLowerCase() === "admin")), [client, principal]);
  const [ready, setReady] = useState<{ registry: LiveRegistry; principal: string }>();
  // Authority transitions are a cache barrier. No child can read the previous
  // principal's normalized cache while a new epoch is becoming active.
  useLayoutEffect(() => {
    let active = true;
    registry.reset(status === "authenticated");
    void client.clearStore().then(() => {
      if (active) {
        setReady({ registry, principal });
        void router.invalidate();
      }
    });
    return () => { active = false; registry.reset(false); };
  }, [client, registry, principal]);
  useEffect(() => liveTransport.subscribe((connected) => registry.transport(connected)), [registry]);
  useEffect(() => {
    if (status !== "authenticated" || ready?.registry !== registry) return;
    let active = true;
    let retry: ReturnType<typeof setTimeout> | undefined;
    let failures = 0;
    const schedule = () => {
      if (!active || failures >= 4) return;
      retry = setTimeout(start, Math.min(30_000, 1500 * 2 ** failures++));
    };
    let subscription: { unsubscribe(): void } | undefined;
    const reconcile = async () => {
      await session.reconcile();
      if (!active) return;
      await router.invalidate();
      registry.invalidate();
    };
    const start = () => {
      if (!active) return;
      subscription = client.subscribe({ query: SessionChangedDocument }).subscribe({
        next: ({ data }) => {
          const payload = data?.sessionChanged;
          if (!payload || !["ready", "changed", "resync"].includes(payload.kind.toLowerCase())) return;
          failures = 0;
          void reconcile().catch(() => {});
        }, // ready also closes the read/listener race.
        error: (error: unknown) => {
          if (!active) return;
          // Read/session recovery only. Commands are never replayed here.
          const recover = async () => {
            if (isUnauthorizedError(error as never)) await session.refresh();
            await reconcile();
            if (active && session.getSnapshot().status === "authenticated" && !isForbiddenError(error as never)) schedule();
          };
          void recover().catch(() => { schedule(); });
        },
        complete: () => { if (active) { void reconcile().catch(() => {}); schedule(); } },
      });
    };
    start();
    return () => { active = false; if (retry) clearTimeout(retry); subscription?.unsubscribe(); };
  }, [client, registry, principal, status, ready?.registry]);
  const context = useMemo(() => ({ registry, epoch: principal }), [registry, principal]);
  return <LiveContext.Provider value={context}>{ready?.registry === registry && ready.principal === principal ? children : null}</LiveContext.Provider>;
}
