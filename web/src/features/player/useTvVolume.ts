import { useSyncExternalStore } from "react";

import { tvVolumeStore, type TvVolume } from "@/lib/tv-app";

/** The TV's volume in the TV app (live, from the remote too); null in a browser. */
export function useTvVolume(): TvVolume | null {
  return useSyncExternalStore(tvVolumeStore.subscribe, tvVolumeStore.getSnapshot, () => null);
}
