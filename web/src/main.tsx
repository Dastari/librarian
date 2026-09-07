import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "@/styles/app.css";

import { AppProviders } from "@/app/providers";
import { router } from "@/app/router";
import { session } from "@/lib/auth/session";

// Start rebuilding the session from cookies immediately; route guards await it.
void session.boot();

// A stale tab after a redeploy or dev-server restart cannot load new route chunks.
// Reload once instead of showing an error; the guard prevents reload loops.
const RELOAD_FLAG = "librarian.chunkReload";
window.addEventListener("vite:preloadError", (event) => {
  event.preventDefault();
  reloadOnceForStaleChunk();
});
export function reloadOnceForStaleChunk(): boolean {
  if (sessionStorage.getItem(RELOAD_FLAG)) return false;
  sessionStorage.setItem(RELOAD_FLAG, String(Date.now()));
  window.location.reload();
  return true;
}
window.addEventListener("load", () => {
  const stamp = Number(sessionStorage.getItem(RELOAD_FLAG));
  if (stamp && Date.now() - stamp > 10_000) sessionStorage.removeItem(RELOAD_FLAG);
});

const container = document.getElementById("root");
if (!container) throw new Error("Missing #root element");

createRoot(container).render(
  <StrictMode>
    <AppProviders>
      <RouterProvider router={router} />
    </AppProviders>
  </StrictMode>,
);
