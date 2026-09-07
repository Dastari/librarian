import { API_BASE_URL } from "./baseUrl";
import { ensureFreshSession } from "../refreshSession";

type AuthFetchOptions = RequestInit & {
  baseUrl?: string;
};

export function buildApiUrl(pathOrUrl: string, baseUrl = API_BASE_URL): string {
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
  const { baseUrl, headers, ...init } = options;

  const requestHeaders = new Headers(headers);

  await ensureFreshSession();

  return fetch(buildApiUrl(pathOrUrl, baseUrl), {
    credentials: "include",
    ...init,
    headers: requestHeaders,
  });
}
