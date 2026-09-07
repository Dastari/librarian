import { useEffect, useState } from "react";

import { KeyValueList, Panel } from "@/components/ui";
import { useInputMode } from "@/lib/input-mode";
import { usePwaUpdate } from "@/lib/pwa";

export function AboutSettings() {
  const { mode } = useInputMode();
  const { offlineReady } = usePwaUpdate();
  const [installed, setInstalled] = useState(false);
  useEffect(() => {
    setInstalled(window.matchMedia("(display-mode: standalone)").matches || ("standalone" in navigator && Boolean((navigator as Navigator & { standalone?: boolean }).standalone)));
  }, []);

  return (
    <div className="flex flex-col gap-6">
      <Panel title="Librarian">
        <KeyValueList
          items={[
            { label: "Web app version", value: __APP_VERSION__ },
            { label: "Build", value: __BUILD_TIME__ },
            { label: "Installed as app", value: installed ? "Yes" : "No, open in a browser" },
            { label: "Offline ready", value: offlineReady ? "Yes" : "Caches after first load" },
            { label: "Input mode", value: mode },
            { label: "Backend", value: `${window.location.origin}/graphql`, mono: true },
          ]}
        />
      </Panel>
      <Panel title="Keyboard and remote">
        <KeyValueList
          columns={2}
          items={[
            { label: "Arrow keys", value: "Move focus" },
            { label: "Enter", value: "Open or play" },
            { label: "Backspace / Escape", value: "Back" },
            { label: "Space or K", value: "Play / pause" },
            { label: "J / L", value: "Seek 10 seconds" },
            { label: "F", value: "Fullscreen" },
            { label: "M", value: "Mute" },
          ]}
        />
      </Panel>
    </div>
  );
}
