import { CombinedGraphQLErrors, ServerError } from "@apollo/client";
import type { ErrorLike } from "@apollo/client";

export type GraphQLErrorCode = "UNAUTHORIZED" | "FORBIDDEN" | string;

export function errorCodes(error: ErrorLike | undefined | null): GraphQLErrorCode[] {
  if (!error || !CombinedGraphQLErrors.is(error)) return [];
  return error.errors.map((item) => String(item.extensions?.code ?? "")).filter(Boolean);
}

export function isUnauthorizedError(error: ErrorLike | undefined | null): boolean {
  if (!error) return false;
  if (ServerError.is(error) && error.statusCode === 401) return true;
  return errorCodes(error).includes("UNAUTHORIZED");
}

export function isForbiddenError(error: ErrorLike | undefined | null): boolean {
  if (!error) return false;
  if (ServerError.is(error) && error.statusCode === 403) return true;
  return errorCodes(error).includes("FORBIDDEN");
}

/** A single sentence suitable for a toast or inline error. */
export function errorMessage(error: unknown, fallback = "Something went wrong"): string {
  if (!error) return fallback;
  if (CombinedGraphQLErrors.is(error)) {
    const first = error.errors[0];
    return first?.message ?? fallback;
  }
  if (error instanceof Error) return error.message || fallback;
  if (typeof error === "string") return error;
  return fallback;
}

/** Mutations return `{ success, error }`; this turns a failed payload into a thrown Error. */
export function assertSuccess<T extends { success: boolean; error?: string | null }>(payload: T | null | undefined, fallback: string): T {
  if (!payload) throw new Error(fallback);
  if (!payload.success) throw new Error(payload.error || fallback);
  return payload;
}
