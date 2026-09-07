import { ApolloLink, CombinedGraphQLErrors, execute, ServerError } from "@apollo/client";
import { GraphQLError, parse } from "graphql";
import { Observable, firstValueFrom, lastValueFrom, of, throwError } from "rxjs";
import { afterEach, describe, expect, it, vi } from "vitest";

import { session } from "@/lib/auth/session";

import { sessionLink } from "../client";

const QUERY = parse(`query Movies { movies { id } }`);
const LOGIN = parse(`mutation Login { login { success } }`);

const unauthorized = () => new CombinedGraphQLErrors({ errors: [new GraphQLError("nope", { extensions: { code: "UNAUTHORIZED" } })] });
const okResult = { data: { movies: [] } };
/**
 * `execute` wants the client the operation belongs to. The session link never reads it, but the
 * error link asks the client's query manager whether a result is incremental.
 */
const context = { client: { queryManager: { incrementalHandler: { isIncrementalResult: () => false } } } as never };

/**
 * A terminating link that fails the first attempt and succeeds afterwards, recording the
 * context of every attempt so the retry can be observed.
 */
function flakyLink(fail: () => unknown, attempts: unknown[] = []) {
  return {
    attempts,
    link: new ApolloLink((operation) => {
      attempts.push(operation.getContext());
      return attempts.length === 1 ? throwError(fail) : of(okResult);
    }),
  };
}

afterEach(() => vi.restoreAllMocks());

describe("apollo session link", () => {
  it("renews the session once and replays the operation", async () => {
    const refresh = vi.spyOn(session, "refresh").mockResolvedValue(true);
    const { link, attempts } = flakyLink(unauthorized);
    const result = await lastValueFrom(execute(ApolloLink.from([sessionLink, link]), { query: QUERY }, context));
    expect(refresh).toHaveBeenCalledTimes(1);
    expect(result).toEqual(okResult);
    expect(attempts).toHaveLength(2);
    expect((attempts[1] as { sessionRetried?: boolean }).sessionRetried).toBe(true);
  });

  it("retries a 401 from the transport as well", async () => {
    const refresh = vi.spyOn(session, "refresh").mockResolvedValue(true);
    const { link } = flakyLink(() => new ServerError("Unauthorized", { response: new Response(null, { status: 401 }), bodyText: "" }));
    await expect(lastValueFrom(execute(ApolloLink.from([sessionLink, link]), { query: QUERY }, context))).resolves.toEqual(okResult);
    expect(refresh).toHaveBeenCalledTimes(1);
  });

  it("surfaces the original error when the renewal fails", async () => {
    vi.spyOn(session, "refresh").mockResolvedValue(false);
    const { link, attempts } = flakyLink(unauthorized);
    await expect(lastValueFrom(execute(ApolloLink.from([sessionLink, link]), { query: QUERY }, context))).rejects.toThrow(/nope/);
    expect(attempts).toHaveLength(1);
  });

  it("never retries more than once", async () => {
    const refresh = vi.spyOn(session, "refresh").mockResolvedValue(true);
    const attempts: unknown[] = [];
    const always = new ApolloLink((operation) => {
      attempts.push(operation.getContext());
      return throwError(unauthorized);
    });
    await expect(lastValueFrom(execute(ApolloLink.from([sessionLink, always]), { query: QUERY }, context))).rejects.toThrow(/nope/);
    expect(refresh).toHaveBeenCalledTimes(1);
    expect(attempts).toHaveLength(2);
  });

  it("leaves the auth operations alone so a failed login is not retried", async () => {
    const refresh = vi.spyOn(session, "refresh").mockResolvedValue(true);
    const { link } = flakyLink(unauthorized);
    await expect(lastValueFrom(execute(ApolloLink.from([sessionLink, link]), { query: LOGIN }, context))).rejects.toThrow(/nope/);
    expect(refresh).not.toHaveBeenCalled();
  });

  it("passes other failures straight through", async () => {
    const refresh = vi.spyOn(session, "refresh").mockResolvedValue(true);
    const { link } = flakyLink(() => new Error("Failed to fetch"));
    await expect(lastValueFrom(execute(ApolloLink.from([sessionLink, link]), { query: QUERY }, context))).rejects.toThrow("Failed to fetch");
    expect(refresh).not.toHaveBeenCalled();
  });

  it("does not touch a successful operation", async () => {
    const refresh = vi.spyOn(session, "refresh").mockResolvedValue(true);
    const link = new ApolloLink(() => new Observable((subscriber) => {
      subscriber.next(okResult);
      subscriber.complete();
    }));
    await expect(firstValueFrom(execute(ApolloLink.from([sessionLink, link]), { query: QUERY }, context))).resolves.toEqual(okResult);
    expect(refresh).not.toHaveBeenCalled();
  });
});
