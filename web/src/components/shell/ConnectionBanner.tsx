import { IconWifiOff } from "@tabler/icons-react";
import { useEffect, useState } from "react";

import { wsClient } from "@/lib/apollo/client";

/** Tells the user when the browser is offline or the live connection to the server is down. */
export function ConnectionBanner() {
  const [offline, setOffline] = useState(typeof navigator !== "undefined" && !navigator.onLine);
  const [socketDown, setSocketDown] = useState(false);

  useEffect(() => {
    const on = () => setOffline(false);
    const off = () => setOffline(true);
    window.addEventListener("online", on);
    window.addEventListener("offline", off);
    const disposers = [
      wsClient.on("connected", () => setSocketDown(false)),
      wsClient.on("closed", () => setSocketDown(true)),
      wsClient.on("error", () => setSocketDown(true)),
    ];
    return () => {
      window.removeEventListener("online", on);
      window.removeEventListener("offline", off);
      for (const dispose of disposers) dispose();
    };
  }, []);

  if (!offline && !socketDown) return null;
  return (
    <div role="status" className="flex items-center justify-center gap-2 bg-warning/15 px-4 py-1.5 text-label text-foreground">
      <IconWifiOff size={14} className="text-warning" />
      {offline ? "You're offline. Showing what was loaded last." : "Reconnecting to the server…"}
    </div>
  );
}
