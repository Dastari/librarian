
import { Button } from "@/components/ui";
import { IconRefresh, IconX } from "@tabler/icons-react";

import { usePwaUpdate } from "@/lib/pwa";

/** Shown when a new build is ready. One tap reloads into it. */
export function UpdatePrompt() {
  const { needRefresh, applyUpdate, dismiss } = usePwaUpdate();
  if (!needRefresh) return null;
  return (
    <div role="status" className="glass-strong fixed bottom-[calc(var(--tabbar-height)+var(--safe-bottom)+0.75rem)] right-4 z-40 flex items-center gap-3 rounded-card border border-glass-border px-4 py-3 shadow-overlay md:bottom-6">
      <IconRefresh size={18} className="text-brand" />
      <span className="text-body-sm text-foreground">A new version is ready.</span>
      <Button size="sm" variant="primary" onPress={() => void applyUpdate()}>
        Reload
      </Button>
      <Button size="sm" variant="ghost" isIconOnly aria-label="Dismiss" onPress={dismiss}>
        <IconX size={16} />
      </Button>
    </div>
  );
}
