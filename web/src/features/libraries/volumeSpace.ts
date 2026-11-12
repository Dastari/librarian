/**
 * Free space on a library's disk or share, for the Libraries table and the Downloads page
 * (docs/design.md Q146). The server stores one reading per volume, with its level already
 * settled (thresholds plus hysteresis); the meter follows that level.
 */

export interface VolumeReading {
  status: string;
  level: string;
  label?: string | null;
  totalBytes?: number | null;
  availableBytes?: number | null;
  checkedAt?: string | null;
  detail?: string | null;
}

/** The low-space thresholds: fractions of the volume and sizes in bytes. */
export interface SpaceThresholds {
  warningFraction: number;
  warningBytes: number;
  criticalFraction: number;
  criticalBytes: number;
}

export const SPACE_SETTING_KEYS = {
  warningFraction: "storage.warning_free_fraction",
  warningBytes: "storage.warning_free_bytes",
  criticalFraction: "storage.critical_free_fraction",
  criticalBytes: "storage.critical_free_bytes",
} as const;

export const DEFAULT_SPACE_THRESHOLDS: SpaceThresholds = {
  warningFraction: 0.1,
  warningBytes: 50e9,
  criticalFraction: 0.05,
  criticalBytes: 20e9,
};

export type VolumeSpace =
  | { state: "unknown"; text: string; description: string }
  | { state: "unavailable"; text: string; description: string }
  | {
      state: "measured";
      level: "normal" | "warning" | "critical";
      /** Used bytes, against `max`. */
      value: number;
      max: number;
      valueText: string;
      description: string;
      warningAt: number;
      dangerAt: number;
    };

const UNITS = ["B", "KB", "MB", "GB", "TB", "PB"];

/** Bytes the way disks are sold (powers of 1000): "1.2 TB", "48 GB". */
export function formatDiskBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "—";
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < UNITS.length - 1) {
    value /= 1000;
    unit += 1;
  }
  if (unit === 0) return `${Math.round(value)} B`;
  const text = value < 10 ? value.toFixed(1).replace(/\.0$/, "") : Math.round(value).toString();
  return `${text} ${UNITS[unit]}`;
}

function floor(total: number, fraction: number, bytes: number) {
  return Math.max(total * fraction, bytes);
}

const clamp = (value: number) => Math.min(1, Math.max(0, value));
/** Just past a full bar: a threshold the bar can never reach. */
const NEVER = 1.0001;

function level(value: string): "normal" | "warning" | "critical" {
  return value === "critical" || value === "warning" ? value : "normal";
}

/** What the Free space cell shows for a volume, or for a library not measured yet. */
export function volumeSpace(volume: VolumeReading | null | undefined, thresholds: SpaceThresholds = DEFAULT_SPACE_THRESHOLDS): VolumeSpace {
  if (!volume) return { state: "unknown", text: "Not measured yet", description: "Free space is read every few minutes." };
  const total = volume.totalBytes ?? 0;
  const available = volume.availableBytes;
  const where = volume.label ? ` on ${volume.label}` : "";
  if (volume.status !== "ok") {
    return { state: "unavailable", text: "Unavailable", description: `The free space${where} cannot be read right now${volume.detail ? `: ${volume.detail}` : "."}` };
  }
  if (total <= 0 || available === null || available === undefined) return { state: "unknown", text: "Not measured yet", description: "Free space is read every few minutes." };
  const free = Math.min(Math.max(available, 0), total);
  const used = total - free;
  const ratio = used / total;
  const stored = level(volume.level);
  // Where the bar turns: at the thresholds, moved only as far as the stored level needs, so the
  // colour always says what the server decided (it keeps a level until space is clearly back).
  let warningAt = clamp(1 - floor(total, thresholds.warningFraction, thresholds.warningBytes) / total);
  let dangerAt = clamp(1 - floor(total, thresholds.criticalFraction, thresholds.criticalBytes) / total);
  if (stored === "critical") {
    dangerAt = Math.min(dangerAt, ratio);
    warningAt = Math.min(warningAt, dangerAt);
  } else if (stored === "warning") {
    warningAt = Math.min(warningAt, ratio);
    if (ratio >= dangerAt) dangerAt = ratio >= 1 ? NEVER : Math.min(NEVER, ratio + 1e-9);
  } else {
    if (ratio >= warningAt) warningAt = ratio >= 1 ? NEVER : Math.min(NEVER, ratio + 1e-9);
    if (ratio >= dangerAt) dangerAt = Math.max(warningAt, ratio >= 1 ? NEVER : Math.min(NEVER, ratio + 1e-9));
  }
  const valueText = `${formatDiskBytes(free)} free of ${formatDiskBytes(total)}`;
  const percent = ((free / total) * 100).toFixed(1).replace(/\.0$/, "");
  const state = stored === "critical" ? "Critically low" : stored === "warning" ? "Low" : null;
  const description = `${valueText} (${percent}%)${where}${state ? `. ${state} on space.` : ""}`;
  return { state: "measured", level: stored, value: used, max: total, valueText, description, warningAt, dangerAt };
}

/** The thresholds from the `storage` settings, with the defaults for anything missing. */
export function spaceThresholds(values: ReadonlyMap<string, string>): SpaceThresholds {
  const read = (key: string, fallback: number, max = Number.POSITIVE_INFINITY) => {
    const raw = values.get(key);
    const parsed = raw === undefined || raw === "" ? Number.NaN : Number(raw);
    return Number.isFinite(parsed) && parsed >= 0 && parsed <= max ? parsed : fallback;
  };
  const warningFraction = read(SPACE_SETTING_KEYS.warningFraction, DEFAULT_SPACE_THRESHOLDS.warningFraction, 1);
  const warningBytes = read(SPACE_SETTING_KEYS.warningBytes, DEFAULT_SPACE_THRESHOLDS.warningBytes);
  return {
    warningFraction,
    warningBytes,
    criticalFraction: Math.min(read(SPACE_SETTING_KEYS.criticalFraction, DEFAULT_SPACE_THRESHOLDS.criticalFraction, 1), warningFraction),
    criticalBytes: Math.min(read(SPACE_SETTING_KEYS.criticalBytes, DEFAULT_SPACE_THRESHOLDS.criticalBytes), warningBytes),
  };
}
