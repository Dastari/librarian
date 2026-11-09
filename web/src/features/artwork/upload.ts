import type { ArtworkTarget } from "./slots";
import { slotInfo } from "./slots";
import type { ArtworkKind } from "@/graphql/generated/graphql";

/**
 * Upload a picture for a title (PUT /api/artwork/{entity}/{id}/{kind}, the file as the body).
 * The server checks it, stores it with the sizes the app uses and locks it.
 */
export async function uploadArtwork(target: ArtworkTarget, kind: ArtworkKind, file: File): Promise<void> {
  const [entity, path] = slotInfo(target.entityType, kind).path;
  let response: Response;
  try {
    response = await fetch(`/api/artwork/${entity}/${encodeURIComponent(target.entityId)}/${path}`, {
      method: "PUT",
      credentials: "same-origin",
      headers: { "content-type": file.type || "application/octet-stream" },
      body: file,
    });
  } catch {
    throw new Error("The picture couldn't be uploaded. Check the connection and try again.");
  }
  let error: string | undefined;
  try {
    const body = (await response.json()) as { success?: boolean; error?: string | null };
    if (response.ok && body.success) return;
    error = body.error ?? undefined;
  } catch {
    // A proxy's page or an empty answer: fall back on the status.
  }
  if (response.status === 413) throw new Error("The file is larger than 20 MB");
  throw new Error(error ?? `The picture couldn't be uploaded (HTTP ${response.status})`);
}
