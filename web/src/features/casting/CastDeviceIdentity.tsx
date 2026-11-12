import type { ActionSet } from "glass-ux/actions";
import { EntityIdentity, type EntityIdentityProps } from "glass-ux/entity";
import { IconCast, IconDeviceSpeaker, IconDeviceTv } from "@tabler/icons-react";
import type { ReactNode } from "react";

import { glyph } from "@/lib/entities/kinds";

import { castDeviceForm, castDeviceState, makeAndModel, protocolLabel, type CastDevice } from "./castDevice";

const GLYPHS = {
  tv: glyph(IconDeviceTv, "device-tv"),
  speaker: glyph(IconDeviceSpeaker, "device-speaker"),
  cast: glyph(IconCast, "cast"),
} as const;

export function castDeviceGlyph(device: CastDevice) {
  return GLYPHS[castDeviceForm(device)];
}

export function castDeviceIdentityProps<T extends CastDevice>(device: T, { actions }: { actions?: ActionSet<T> } = {}): EntityIdentityProps<T> {
  const Icon = castDeviceGlyph(device);
  return {
    kind: "Cast device",
    name: device.name,
    icon: <Icon />,
    state: castDeviceState(device),
    facts: [makeAndModel(device), protocolLabel(device), device.address],
    target: device,
    actions,
  };
}

/**
 * A cast receiver wherever it is named: a TV, speaker or cast glyph with a dot for whether it
 * answered discovery lately, then its maker and model, its protocol and its address. A favourite
 * is starred in its name.
 */
export function CastDeviceIdentity<T extends CastDevice>({ device, tier = "row", actions, headerActions, className, onOpen }: {
  device: T;
  tier?: "header" | "compact" | "row" | "chip" | "icon" | "auto";
  actions?: ActionSet<T>;
  headerActions?: ReactNode;
  className?: string;
  onOpen?: () => void;
}) {
  return (
    <EntityIdentity
      {...castDeviceIdentityProps(device, { actions })}
      title={device.isFavorite ? <>{device.name} <span aria-hidden className="text-accent">★</span></> : undefined}
      tier={tier}
      className={className}
      slots={headerActions ? { actions: headerActions } : undefined}
      onOpen={onOpen}
    />
  );
}
