import { toast } from "glass-ux/toast";
import { useEffect } from "react";

import { applyUpdate, usePwa } from "@/lib/pwa";

const TOAST_ID = "app-update";

/**
 * When a newer build is deployed, one toast offers to load it. It stays until it is used or
 * closed; Settings › About keeps the same action for later.
 */
export function UpdatePrompt() {
  const { updateAvailable } = usePwa();
  useEffect(() => {
    if (!updateAvailable) return;
    toast.info("A new version of Librarian is available", {
      id: TOAST_ID,
      description: "Refresh to load it. Anything playing will stop.",
      timeout: 0,
      action: { label: "Refresh", onClick: () => void applyUpdate() },
    });
  }, [updateAvailable]);
  return null;
}
