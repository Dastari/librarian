import type { ActionSet } from "glass-ux/actions";
import { EntityIdentity, type EntityIdentityProps, type EntityState } from "glass-ux/entity";
import { IconDeviceDesktop, IconDeviceMobile, IconDeviceTablet, IconDeviceTv, IconDevices } from "@tabler/icons-react";
import type { ReactNode } from "react";

import { glyph } from "@/lib/entities/kinds";
import { formatRelative, parseTimestamp } from "@/lib/format";

/** What the identity needs of a device (the `DeviceFields` fragment has it all). */
export interface DeviceEntity {
  id: string;
  userId: string;
  name: string;
  formFactor: string;
  platform?: string | null;
  product?: string | null;
  appVersion?: string | null;
  lastIp?: string | null;
  lastSeenAt: string;
  signedOutAt?: string | null;
}

const GLYPHS = {
  tv: glyph(IconDeviceTv, "device-tv"),
  phone: glyph(IconDeviceMobile, "device-mobile"),
  tablet: glyph(IconDeviceTablet, "device-tablet"),
  desktop: glyph(IconDeviceDesktop, "device-desktop"),
  unknown: glyph(IconDevices, "device"),
} as const;

/** The glyph for a TV, phone, tablet or computer. */
export function deviceGlyph(formFactor: string | null | undefined) {
  return GLYPHS[(formFactor ?? "unknown") as keyof typeof GLYPHS] ?? GLYPHS.unknown;
}

/** Seen in the last ten minutes: the app is open there now. */
const ACTIVE_MS = 10 * 60_000;

export function deviceState(device: DeviceEntity, { current = false, now = Date.now() }: { current?: boolean; now?: number } = {}): EntityState {
  if (current) return { id: "current", label: "This device", tone: "accent" };
  if (device.signedOutAt) return { id: "signed-out", label: "Signed out", tone: "neutral" };
  const seen = parseTimestamp(device.lastSeenAt)?.getTime() ?? 0;
  if (now - seen < ACTIVE_MS) return { id: "active", label: "Active now", tone: "success" };
  return { id: "idle", label: `Seen ${formatRelative(device.lastSeenAt, now)}`, tone: "neutral" };
}

/** "Safari · iPadOS" */
export const deviceSoftware = (device: Pick<DeviceEntity, "product" | "platform">) => [device.product, device.platform].filter(Boolean).join(" · ");

export function deviceIdentityProps<T extends DeviceEntity>(device: T, { actions, current, owner }: { actions?: ActionSet<T>; current?: boolean; owner?: string } = {}): EntityIdentityProps<T> {
  const Icon = deviceGlyph(device.formFactor);
  return {
    kind: "Device",
    name: device.name,
    icon: <Icon />,
    state: deviceState(device, { current }),
    facts: [
      owner ?? "",
      deviceSoftware(device),
      device.signedOutAt ? `Signed out ${formatRelative(device.signedOutAt)}` : `Last seen ${formatRelative(device.lastSeenAt)}`,
    ],
    target: device,
    actions,
  };
}

/**
 * A device wherever it is named: the TV, phone, tablet or computer glyph with a dot for whether
 * the app is open there, then its browser, platform and when it was last seen. "This device" marks
 * the one you are using.
 */
export function DeviceIdentity<T extends DeviceEntity>({ device, tier = "row", actions, current, owner, headerActions, className, onOpen }: {
  device: T;
  tier?: "header" | "compact" | "row" | "chip" | "icon" | "auto";
  actions?: ActionSet<T>;
  current?: boolean;
  /** The owner's name, in an administrator's list of everyone's devices. */
  owner?: string;
  headerActions?: ReactNode;
  className?: string;
  onOpen?: () => void;
}) {
  return (
    <EntityIdentity
      {...deviceIdentityProps(device, { actions, current, owner })}
      tier={tier}
      className={className}
      slots={headerActions ? { actions: headerActions } : undefined}
      onOpen={onOpen}
    />
  );
}
