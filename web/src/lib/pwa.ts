/**
 * The installed app's service worker and new-version detection, as one store.
 *
 * Production builds register the Workbox worker (prompt style: a new build waits until the person
 * refreshes) and also poll `version.json`, so a deployment is noticed where no worker runs or
 * before the worker has looked. The development server (the live site on :3000 runs Vite with
 * HMR) serves the web manifest, so the installed app launches standalone, but registers no
 * worker and polls nothing; it unregisters any worker a production preview left on the same
 * origin, so a cache can never stand in for the dev server.
 */
import { useSyncExternalStore } from "react";

export interface PwaState {
  /** A newer build than this tab's is deployed. */
  updateAvailable: boolean;
  /** The worker has cached the app shell. */
  offlineReady: boolean;
}

const VERSION_POLL_MS = 5 * 60 * 1000;
const INSTALL_WAIT_MS = 15 * 1000;

let state: PwaState = { updateAvailable: false, offlineReady: false };
const listeners = new Set<() => void>();
let started = false;
let registration: ServiceWorkerRegistration | undefined;
let skipWaiting: ((reload?: boolean) => Promise<void>) | undefined;

function set(patch: Partial<PwaState>): void {
  const next = { ...state, ...patch };
  if (next.updateAvailable === state.updateAvailable && next.offlineReady === state.offlineReady) return;
  state = next;
  for (const listener of listeners) listener();
}

export const pwaStore = {
  subscribe(listener: () => void): () => void {
    listeners.add(listener);
    return () => listeners.delete(listener);
  },
  getSnapshot: (): PwaState => state,
};

export function usePwa(): PwaState {
  return useSyncExternalStore(pwaStore.subscribe, pwaStore.getSnapshot, pwaStore.getSnapshot);
}

/** Start once at boot. A no-op outside production builds, apart from retiring stray workers. */
export function startPwa(): void {
  if (started || typeof window === "undefined") return;
  started = true;
  if (!import.meta.env.PROD) {
    void retireWorkers();
    return;
  }
  void registerWorker();
  window.setInterval(() => void checkForUpdate(), VERSION_POLL_MS);
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "visible") void checkForUpdate();
  });
  window.addEventListener("online", () => void checkForUpdate());
}

async function registerWorker(): Promise<void> {
  if (!("serviceWorker" in navigator)) return;
  const { registerSW } = await import("virtual:pwa-register");
  skipWaiting = registerSW({
    immediate: true,
    onNeedRefresh: () => set({ updateAvailable: true }),
    onOfflineReady: () => set({ offlineReady: true }),
    onRegisteredSW: (_url, registered) => {
      registration = registered;
    },
    onRegisterError: (error) => console.warn("Service worker registration failed", error),
  });
  // Earlier builds cached artwork at runtime; the worker keeps only the app shell now.
  if ("caches" in window) void caches.delete("artwork").catch(() => undefined);
}

async function retireWorkers(): Promise<void> {
  if (!("serviceWorker" in navigator)) return;
  try {
    const registrations = await navigator.serviceWorker.getRegistrations();
    await Promise.all(registrations.map((worker) => worker.unregister()));
  } catch {
    // Nothing registered, or storage is blocked: either way there is nothing to retire.
  }
}

/** Ask the worker and `version.json` whether a newer build is deployed. */
export async function checkForUpdate(): Promise<boolean> {
  if (!import.meta.env.PROD) return false;
  try {
    await registration?.update();
  } catch {
    // Offline or the server is restarting; the next check tries again.
  }
  try {
    const response = await fetch(`/version.json?t=${Date.now()}`, { cache: "no-store", credentials: "omit" });
    if (response.ok) {
      const deployed = (await response.json()) as { buildId?: unknown };
      if (typeof deployed.buildId === "string" && deployed.buildId !== __BUILD_ID__) set({ updateAvailable: true });
    }
  } catch {
    // As above.
  }
  return state.updateAvailable;
}

function waitForInstall(worker: ServiceWorker): Promise<void> {
  return new Promise((resolve) => {
    const timer = window.setTimeout(resolve, INSTALL_WAIT_MS);
    worker.addEventListener("statechange", () => {
      if (worker.state === "installed" || worker.state === "redundant") {
        window.clearTimeout(timer);
        resolve();
      }
    });
  });
}

/**
 * Load the newest build. A waiting worker takes over and the page reloads under it. When the
 * worker has not fetched the new build yet, fetch it first; if it still cannot take over, drop it
 * so the reload comes from the network (it registers again afterwards).
 */
export async function applyUpdate(): Promise<void> {
  if (registration && skipWaiting) {
    if (!registration.waiting) {
      try {
        await registration.update();
      } catch {
        // Fall through to the checks below.
      }
      if (registration.installing) await waitForInstall(registration.installing);
    }
    if (registration.waiting) {
      // The plugin reloads once the new worker controls the page; this catches the case where
      // it never saw that worker arrive.
      navigator.serviceWorker.addEventListener("controllerchange", () => window.location.reload(), { once: true });
      await skipWaiting(true);
      window.setTimeout(() => window.location.reload(), 4000);
      return;
    }
    await registration.unregister().catch(() => false);
  }
  window.location.reload();
}
