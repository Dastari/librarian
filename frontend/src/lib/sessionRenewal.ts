import { getSession } from "./auth";

/** Keep cookies fresh during long playback; retry outages without signing out. */
export function startSessionRenewal(refresh: () => Promise<boolean>): () => void {
  const renewIfNeeded = () => {
    const session = getSession();
    // Leave room for the one-minute timer throttling of background tabs.
    if (session && session.expiresAt <= Date.now() / 1000 + 120) {
      void refresh();
    }
  };
  const onVisible = () => {
    if (document.visibilityState === "visible") renewIfNeeded();
  };
  const interval = window.setInterval(renewIfNeeded, 30000);
  window.addEventListener("focus", renewIfNeeded);
  window.addEventListener("online", renewIfNeeded);
  document.addEventListener("visibilitychange", onVisible);
  return () => {
    window.clearInterval(interval);
    window.removeEventListener("focus", renewIfNeeded);
    window.removeEventListener("online", renewIfNeeded);
    document.removeEventListener("visibilitychange", onVisible);
  };
}
