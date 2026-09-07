/**
 * Session store.
 *
 * Credentials live only in server-set HttpOnly cookies. The browser never sees a token; it
 * learns about the session through the `refreshToken` and `me` operations. This module keeps
 * the non-secret view of that state (user, expiry) and owns renewal:
 *
 * - renews early (before the 15 minute access window ends)
 * - renews on focus, visibility and network recovery when close to expiry
 * - serialises renewal across tabs with the Web Locks API
 * - exposes `ensureFresh()` for the Apollo error link to retry unauthorized operations
 */
import type { ApolloClient } from "@apollo/client";

import { CurrentUserDocument, LogoutDocument, RefreshSessionDocument } from "@/graphql/generated/graphql";
import type { CurrentUserQuery } from "@/graphql/generated/graphql";
import { isUnauthorizedError } from "@/lib/graphql/errors";

export type SessionUser = NonNullable<CurrentUserQuery["me"]>;
export type SessionStatus = "booting" | "anonymous" | "authenticated";

export interface SessionState {
  status: SessionStatus;
  user: SessionUser | null;
  /** Epoch milliseconds when the access credential expires; null when anonymous. */
  expiresAt: number | null;
}

type Listener = () => void;

const LOCK_NAME = "librarian.session.refresh";
const EARLY_RENEW_MS = 90_000;
const NEAR_EXPIRY_MS = 3 * 60_000;
const MIN_TIMER_MS = 5_000;

class SessionStore {
  private state: SessionState = { status: "booting", user: null, expiresAt: null };
  private listeners = new Set<Listener>();
  private timer: ReturnType<typeof setTimeout> | null = null;
  private inflight: Promise<boolean> | null = null;
  private client: ApolloClient | null = null;
  private onAuthChange: Array<(authenticated: boolean) => void> = [];
  private bootPromise: Promise<void> | null = null;

  attach(client: ApolloClient): void {
    this.client = client;
    if (typeof window !== "undefined") {
      window.addEventListener("focus", this.handleWake);
      window.addEventListener("online", this.handleWake);
      document.addEventListener("visibilitychange", this.handleWake);
    }
  }

  subscribe = (listener: Listener): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  getSnapshot = (): SessionState => this.state;

  /** Fires after login/logout so infrastructure (websocket, cache) can reset. */
  onChange(listener: (authenticated: boolean) => void): () => void {
    this.onAuthChange.push(listener);
    return () => {
      this.onAuthChange = this.onAuthChange.filter((item) => item !== listener);
    };
  }

  private set(next: Partial<SessionState>): void {
    this.state = { ...this.state, ...next };
    for (const listener of this.listeners) listener();
  }

  /** Called once at startup: rebuild the session from cookies. Safe to call repeatedly. */
  boot(): Promise<void> {
    if (!this.bootPromise) {
      this.bootPromise = (async () => {
        const refreshed = await this.refresh();
        if (!refreshed) {
          this.set({ status: "anonymous", user: null, expiresAt: null });
          return;
        }
        await this.loadUser();
      })();
    }
    return this.bootPromise;
  }

  /** Resolves once the initial boot has settled; route guards await this. */
  ready(): Promise<void> {
    return this.boot();
  }

  private async loadUser(): Promise<void> {
    if (!this.client) return;
    try {
      const { data } = await this.client.query({ query: CurrentUserDocument, fetchPolicy: "network-only" });
      if (data?.me) {
        this.set({ status: "authenticated", user: data.me });
        return;
      }
    } catch {
      // A transient failure keeps the previous status; the timer will retry.
    }
    if (this.state.status === "booting") this.set({ status: "anonymous", user: null, expiresAt: null });
  }

  /** Marks the session as authenticated after a successful login or registration. */
  establish(user: SessionUser, expiresInSeconds: number): void {
    this.set({ status: "authenticated", user, expiresAt: Date.now() + expiresInSeconds * 1000 });
    this.schedule();
    this.notify(true);
  }

  async logout(): Promise<void> {
    try {
      await this.client?.mutate({ mutation: LogoutDocument });
    } catch {
      // The cookies are cleared server-side; a network failure still ends the local session.
    }
    this.clearTimer();
    this.set({ status: "anonymous", user: null, expiresAt: null });
    this.notify(false);
    await this.client?.clearStore();
  }

  /** Renews when the access credential is about to expire. Cheap to call often. */
  async ensureFresh(): Promise<boolean> {
    const { expiresAt, status } = this.state;
    if (status === "authenticated" && expiresAt && expiresAt - Date.now() > NEAR_EXPIRY_MS) return true;
    return this.refresh();
  }

  /** Rotates the refresh cookie. Returns false only when the server says the session is gone. */
  refresh(): Promise<boolean> {
    if (this.inflight) return this.inflight;
    this.inflight = this.withLock(() => this.performRefresh()).finally(() => {
      this.inflight = null;
    });
    return this.inflight;
  }

  private async performRefresh(): Promise<boolean> {
    if (!this.client) return false;
    try {
      const { data } = await this.client.mutate({ mutation: RefreshSessionDocument, fetchPolicy: "no-cache" });
      const payload = data?.refreshToken;
      if (payload?.success && payload.tokens) {
        const expiresAt = Date.now() + payload.tokens.expiresIn * 1000;
        this.set({ expiresAt, status: this.state.user ? "authenticated" : this.state.status });
        this.schedule();
        return true;
      }
      this.expire();
      return false;
    } catch (error) {
      if (isUnauthorizedError(error as never)) {
        this.expire();
        return false;
      }
      // Network or server outage: keep the refresh cookie and try again shortly.
      this.schedule(30_000);
      return this.state.status === "authenticated";
    }
  }

  private expire(): void {
    const wasAuthenticated = this.state.status === "authenticated";
    this.clearTimer();
    this.set({ status: "anonymous", user: null, expiresAt: null });
    if (wasAuthenticated) {
      this.notify(false);
      void this.client?.clearStore();
    }
  }

  private notify(authenticated: boolean): void {
    for (const listener of this.onAuthChange) listener(authenticated);
  }

  private schedule(delay?: number): void {
    this.clearTimer();
    const { expiresAt } = this.state;
    const wait = delay ?? (expiresAt ? Math.max(expiresAt - Date.now() - EARLY_RENEW_MS, MIN_TIMER_MS) : null);
    if (wait === null) return;
    this.timer = setTimeout(() => {
      void this.refresh();
    }, wait);
  }

  private clearTimer(): void {
    if (this.timer) clearTimeout(this.timer);
    this.timer = null;
  }

  private handleWake = (): void => {
    if (document.visibilityState === "hidden") return;
    if (this.state.status !== "authenticated") return;
    void this.ensureFresh();
  };

  private async withLock<T>(task: () => Promise<T>): Promise<T> {
    const locks = typeof navigator !== "undefined" ? navigator.locks : undefined;
    if (!locks) return task();
    return locks.request(LOCK_NAME, task);
  }
}

export const session = new SessionStore();
