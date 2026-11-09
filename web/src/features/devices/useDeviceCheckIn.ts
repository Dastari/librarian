import { useApolloClient } from "@apollo/client/react";
import { useEffect } from "react";

import { CheckInDeviceDocument } from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { useInputMode } from "@/lib/input-mode";

import { clientCheckIn, currentDevice } from "./clientDevice";

/** How often an open app says it is still here (the device's "last seen"). */
const CHECK_IN_EVERY_MS = 10 * 60_000;

/**
 * Checks this browser in as one of the signed-in person's devices: on sign-in, when the app comes
 * back to the foreground, when TV mode changes, and every ten minutes while it is open. The device
 * id it gets back goes on the player's sessions. A failed check-in only leaves sessions without a
 * device; it never interrupts the app.
 */
export function useDeviceCheckIn() {
  const client = useApolloClient();
  const { user } = useSession();
  const { mode } = useInputMode();
  const userId = user?.id ?? null;
  const tvMode = mode === "tv";

  useEffect(() => {
    if (!userId) {
      currentDevice.set(null);
      return;
    }
    let cancelled = false;
    let last = 0;
    const checkIn = () => {
      last = Date.now();
      client
        .mutate({ mutation: CheckInDeviceDocument, variables: { input: clientCheckIn({ tvMode, appVersion: typeof __APP_VERSION__ === "string" ? __APP_VERSION__ : undefined }) } })
        .then((result) => {
          const device = result.data?.checkInDevice.device;
          if (!cancelled && device) currentDevice.set(device.id);
        })
        .catch(() => {});
    };
    checkIn();
    const timer = setInterval(checkIn, CHECK_IN_EVERY_MS);
    const onVisible = () => {
      if (document.visibilityState === "visible" && Date.now() - last > 60_000) checkIn();
    };
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      cancelled = true;
      clearInterval(timer);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [client, userId, tvMode]);
}
