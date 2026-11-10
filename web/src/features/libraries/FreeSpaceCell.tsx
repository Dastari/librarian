import { Badge } from "glass-ux/badge";
import { TextCell } from "glass-ux/data-table";
import { MeterCell } from "glass-ux/meter-cell";
import { Tooltip } from "glass-ux/tooltip";

import { DEFAULT_SPACE_THRESHOLDS, volumeSpace, type SpaceThresholds, type VolumeReading } from "./volumeSpace";

/**
 * The Free space cell: used against total on the library's disk or share, "1.2 TB free of 8 TB",
 * coloured by the server's level. A share that cannot be read says so instead of showing 0.
 */
export function FreeSpaceCell({ volume, thresholds = DEFAULT_SPACE_THRESHOLDS }: { volume: VolumeReading | null | undefined; thresholds?: SpaceThresholds }) {
  const space = volumeSpace(volume, thresholds);
  if (space.state === "unavailable") {
    return <Tooltip content={space.description}><span><Badge size="sm" marker="dot" tone="warning">{space.text}</Badge></span></Tooltip>;
  }
  if (space.state === "unknown") return <TextCell muted>{space.text}</TextCell>;
  return <MeterCell label="Free space" tone="data" value={space.value} max={space.max} valueText={space.valueText} description={space.description} warningAt={space.warningAt} dangerAt={space.dangerAt} />;
}
