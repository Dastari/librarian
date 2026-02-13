export function sanitizeInternalRedirect(
  redirect: string | undefined,
): string | undefined {
  if (!redirect) return undefined;

  const trimmed = redirect.trim();
  if (!trimmed) return undefined;

  // Protocol-relative redirects are external and unsafe.
  if (trimmed.startsWith("//")) return undefined;

  // Safe relative paths.
  if (trimmed.startsWith("/")) return trimmed;

  // For absolute URLs, only allow same-origin targets.
  const currentOrigin =
    typeof window !== "undefined" ? window.location.origin : undefined;
  if (!currentOrigin) return undefined;

  try {
    const url = new URL(trimmed, currentOrigin);
    if (url.origin !== currentOrigin) return undefined;
    return `${url.pathname}${url.search}${url.hash}`;
  } catch {
    return undefined;
  }
}
