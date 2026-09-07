import { useSyncExternalStore } from "react";

import { session } from "./session";
import type { SessionState } from "./session";

export function useSession(): SessionState {
  return useSyncExternalStore(session.subscribe, session.getSnapshot, session.getSnapshot);
}

export function useIsAdmin(): boolean {
  const { user } = useSession();
  return user?.role === "admin";
}
