// An empty base keeps browser requests on the origin serving the application.
// Local backend addresses belong in Vite's server-side proxy configuration.
export const API_BASE_URL = (import.meta.env.VITE_API_URL ?? "")
  .trim()
  .replace(/\/+$/, "");

export function graphqlWebSocketUrl(
  baseUrl = API_BASE_URL,
  pageOrigin = window.location.origin,
): string {
  const url = new URL(`${baseUrl.replace(/\/+$/, "")}/graphql/ws`, pageOrigin);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  return url.href;
}
