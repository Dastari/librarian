import { useEffect, useState } from "react";

import { SettingRow } from "glass-ux/setting-row";

import { Button } from "@/components/ui";
import { useInputMode } from "@/lib/input-mode";
import { applyUpdate, checkForUpdate, usePwa } from "@/lib/pwa";

import { SettingsPage, SettingsSection } from "./SettingsSection";

/** A read-only fact: its name on the left, its value on the right, like a setting without a control. */
function Fact({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return <SettingRow label={label} control={<span className={mono ? "text-body-sm break-all font-mono text-muted" : "text-body-sm text-muted"}>{value}</span>} />;
}

export function AboutSettings() {
  const { mode } = useInputMode();
  const { offlineReady, updateAvailable } = usePwa();
  const [checking, setChecking] = useState(false);
  const [installed, setInstalled] = useState(false);
  useEffect(() => {
    setInstalled(window.matchMedia("(display-mode: standalone)").matches || ("standalone" in navigator && Boolean((navigator as Navigator & { standalone?: boolean }).standalone)));
  }, []);

  const update = !import.meta.env.PROD
    ? "Development server (updates load live)"
    : updateAvailable ? "A new version is ready" : "Up to date";

  return (
    <SettingsPage>
      <SettingsSection
        title="Librarian"
        width="readable"
        actions={import.meta.env.PROD ? (updateAvailable ? (
          <Button size="sm" variant="primary" onPress={() => void applyUpdate()}>Refresh to update</Button>
        ) : (
          <Button size="sm" variant="secondary" isPending={checking} onPress={() => {
            setChecking(true);
            void checkForUpdate().finally(() => setChecking(false));
          }}>Check for updates</Button>
        )) : undefined}
      >
        {[
          { label: "Web app version", value: __APP_VERSION__ },
          { label: "Build", value: __BUILD_ID__, mono: true },
          { label: "Built", value: __BUILD_TIME__ },
          { label: "Updates", value: update },
          { label: "Installed as app", value: installed ? "Yes" : "No, open in a browser" },
          { label: "Offline ready", value: offlineReady ? "Yes" : "Caches after first load" },
          { label: "Input mode", value: mode },
          { label: "Backend", value: `${window.location.origin}/graphql`, mono: true },
        ].map((fact) => <Fact key={fact.label} {...fact} />)}
      </SettingsSection>
      <SettingsSection title="Keyboard and remote" width="readable">
        {[
          { label: "Arrow keys", value: "Move focus" },
          { label: "Enter", value: "Open or play" },
          { label: "Backspace / Escape", value: "Back" },
          { label: "Space or K", value: "Play / pause" },
          { label: "J / L", value: "Seek 10 seconds" },
          { label: "F", value: "Fullscreen" },
          { label: "M", value: "Mute" },
        ].map((fact) => <Fact key={fact.label} {...fact} />)}
      </SettingsSection>
    </SettingsPage>
  );
}
