import { getAccessToken } from "../auth";

const API_URL = import.meta.env.VITE_API_URL || "http://localhost:3001";

type AuthFetchOptions = RequestInit & {
  includeAuth?: boolean;
  baseUrl?: string;
};

export function buildApiUrl(pathOrUrl: string, baseUrl = API_URL): string {
  if (/^https?:\/\//i.test(pathOrUrl)) {
    return pathOrUrl;
  }
  const path = pathOrUrl.startsWith("/") ? pathOrUrl : `/${pathOrUrl}`;
  return `${baseUrl}${path}`;
}

export async function authFetch(
  pathOrUrl: string,
  options: AuthFetchOptions = {},
): Promise<Response> {
  const { includeAuth = true, baseUrl, headers, ...init } = options;

  const requestHeaders = new Headers(headers);
  if (includeAuth) {
    const authToken = getAccessToken();
    if (authToken && !requestHeaders.has("Authorization")) {
      requestHeaders.set("Authorization", `Bearer ${authToken}`);
    }
  }

  return fetch(buildApiUrl(pathOrUrl, baseUrl), {
    credentials: "include",
    ...init,
    headers: requestHeaders,
  });
}
