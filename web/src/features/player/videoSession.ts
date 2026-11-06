import type { MediaWindowRect } from "glass-ux/media-player";

import type { PlayItem } from "./store";

/** A mini player's state, kept so a page refresh picks it back up. One per person on a device. */
export interface VideoSession {
  item: PlayItem;
  position: number;
  playing: boolean;
}

const sessionKey = (userId: string) => `librarian.player.mini.${userId}`;
const RECT_KEY = "librarian.player.miniRect";

function read<T>(key: string): T | null {
  try {
    const stored = window.localStorage.getItem(key);
    return stored ? (JSON.parse(stored) as T) : null;
  } catch {
    return null;
  }
}

function write(key: string, value: unknown) {
  try {
    if (value === null) window.localStorage.removeItem(key);
    else window.localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // A browser refusing storage only costs the resume.
  }
}

export function readVideoSession(userId: string): VideoSession | null {
  const session = read<VideoSession>(sessionKey(userId));
  return session?.item?.mediaFileId && session.item.entity ? session : null;
}

export const saveVideoSession = (userId: string, session: VideoSession) => write(sessionKey(userId), session);
export const clearVideoSession = (userId: string) => write(sessionKey(userId), null);

export const readMiniRect = () => read<MediaWindowRect>(RECT_KEY) ?? undefined;
export const saveMiniRect = (rect: MediaWindowRect) => write(RECT_KEY, rect);
