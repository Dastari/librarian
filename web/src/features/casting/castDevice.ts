import type { EntityState } from "glass-ux/entity";

import type { CastDeviceFieldsFragment } from "@/graphql/generated/graphql";
import { formatRelative, parseTimestamp } from "@/lib/format";

export type CastDevice = CastDeviceFieldsFragment;

/** What discovery learned about a receiver (`CastDevice.discoveryDetails`, written by the server's `cast::describe`). */
export interface CastDiscoveryDetails {
  protocol?: "googleCast" | "dlna";
  reportedName?: string;
  modelDescription?: string;
  modelNumber?: string;
  serviceName?: string;
  hostname?: string;
  addresses?: string[];
  capabilities?: string[];
  runningApp?: string;
  txt?: Record<string, string>;
  descriptionUrl?: string;
  server?: string;
  usn?: string;
  services?: string[];
  descriptionError?: string;
}

export function discoveryDetails(device: Pick<CastDevice, "discoveryDetails">): CastDiscoveryDetails {
  if (!device.discoveryDetails) return {};
  try {
    const value: unknown = JSON.parse(device.discoveryDetails);
    return value && typeof value === "object" ? (value as CastDiscoveryDetails) : {};
  } catch {
    return {};
  }
}

/**
 * The name the receiver reports, tidied as discovery tidies it (Samsung's "[TV] " prefix
 * dropped), or nothing when it reported none.
 */
export function reportedDisplayName(device: Pick<CastDevice, "discoveryDetails">): string | undefined {
  const name = discoveryDetails(device).reportedName?.trim();
  if (!name) return undefined;
  const tagged = /^\[([^\]]{1,6})\]\s*(.+)$/.exec(name);
  return tagged ? tagged[2]!.trim() : name;
}

export type CastProtocol ="googleCast" | "dlna" | "unknown";

export function castProtocol(device: Pick<CastDevice, "deviceType">): CastProtocol {
  const type = device.deviceType.trim().toUpperCase();
  if (type === "CHROMECAST" || type === "CHROMECAST_AUDIO") return "googleCast";
  if (type === "DLNA_RENDERER") return "dlna";
  return "unknown";
}

export const PROTOCOL_LABEL: Record<CastProtocol, string> = { googleCast: "Google Cast", dlna: "DLNA", unknown: "Unknown" };

/** "Google Cast", "Google Cast · audio only", "DLNA". */
export function protocolLabel(device: Pick<CastDevice, "deviceType">): string {
  const audio = device.deviceType.trim().toUpperCase() === "CHROMECAST_AUDIO";
  return audio ? "Google Cast · audio only" : PROTOCOL_LABEL[castProtocol(device)];
}

/** How the row came to exist. */
export function originLabel(device: Pick<CastDevice, "isManual" | "discoveryOrigin" | "deviceType">): string {
  if (device.isManual && device.discoveryOrigin !== "NETWORK") return "Added by address";
  const protocol = castProtocol(device);
  if (protocol === "googleCast") return "Found on the network (mDNS)";
  if (protocol === "dlna") return "Found on the network (SSDP)";
  return "Found on the network";
}

/** Why Librarian can't send video to it, or nothing when it can. */
export function playbackBlocker(device: CastDevice): string | undefined {
  if (device.enabled === false) return "Turned off in Settings › Casting.";
  if (castProtocol(device) === "dlna") return "DLNA receivers are listed but can't play from Librarian yet.";
  if (castProtocol(device) !== "googleCast" || device.playbackSupported !== true) return "Only Google Cast receivers can play.";
  return undefined;
}

/** "Samsung Electronics UA55KS8000", or the model alone. */
export function makeAndModel(device: Pick<CastDevice, "manufacturer" | "model">): string {
  const maker = device.manufacturer?.trim();
  const model = device.model?.trim();
  if (maker && model) return model.toLowerCase().startsWith(maker.split(" ")[0]!.toLowerCase()) ? model : `${maker} ${model}`;
  return maker || model || "";
}

/** Answered discovery in the last quarter of an hour. */
const AVAILABLE_MS = 15 * 60_000;

export function castDeviceState(device: CastDevice, now = Date.now()): EntityState {
  if (device.enabled === false) return { id: "disabled", label: "Turned off", tone: "neutral", paused: true };
  if (device.lastProbeError) return { id: "error", label: "Not responding", tone: "danger" };
  const seen = parseTimestamp(device.lastSeenAt)?.getTime();
  if (seen === undefined) return { id: "manual", label: device.isManual ? "Added by address" : "Not seen yet", tone: "neutral" };
  if (now - seen < AVAILABLE_MS) return { id: "available", label: "Available", tone: "success" };
  return { id: "away", label: `Seen ${formatRelative(device.lastSeenAt, now)}`, tone: "neutral" };
}

/** A TV, a speaker, or a cast dongle, for its glyph. */
export function castDeviceForm(device: CastDevice): "tv" | "speaker" | "cast" {
  if (device.deviceType.trim().toUpperCase() === "CHROMECAST_AUDIO") return "speaker";
  const details = discoveryDetails(device);
  const said = [device.model, details.reportedName, details.modelDescription].filter(Boolean).join(" ").toLowerCase();
  if (/\btv\b|television|\[tv\]|bravia|tizen|webos|roku|shield|fire ?tv/.test(said)) return "tv";
  if (castProtocol(device) === "dlna") return /speaker|audio|sonos|receiver|\[av\]/.test(said) ? "speaker" : "tv";
  return "cast";
}
