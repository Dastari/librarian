import { CombinedGraphQLErrors, ServerError } from "@apollo/client";
import { GraphQLError } from "graphql";
import { describe, expect, it } from "vitest";

import { assertSuccess, errorCodes, errorMessage, isForbiddenError, isUnauthorizedError } from "../errors";

const combined = (...errors: GraphQLError[]) => new CombinedGraphQLErrors({ errors });
const serverError = (statusCode: number) =>
  new ServerError("Server error", { response: new Response(null, { status: statusCode }), bodyText: "" });

describe("graphql errors", () => {
  it("reads the extension codes off a combined error", () => {
    const error = combined(new GraphQLError("nope", { extensions: { code: "UNAUTHORIZED" } }), new GraphQLError("plain"));
    expect(errorCodes(error)).toEqual(["UNAUTHORIZED"]);
    expect(errorCodes(null)).toEqual([]);
    expect(errorCodes(new Error("boom"))).toEqual([]);
  });

  it("recognises unauthorized from the code and from a 401", () => {
    expect(isUnauthorizedError(combined(new GraphQLError("x", { extensions: { code: "UNAUTHORIZED" } })))).toBe(true);
    expect(isUnauthorizedError(serverError(401))).toBe(true);
    expect(isUnauthorizedError(serverError(500))).toBe(false);
    expect(isUnauthorizedError(undefined)).toBe(false);
  });

  it("recognises forbidden from the code and from a 403", () => {
    expect(isForbiddenError(combined(new GraphQLError("x", { extensions: { code: "FORBIDDEN" } })))).toBe(true);
    expect(isForbiddenError(serverError(403))).toBe(true);
    expect(isForbiddenError(serverError(401))).toBe(false);
    expect(isForbiddenError(null)).toBe(false);
  });

  it("reduces anything to one sentence", () => {
    expect(errorMessage(combined(new GraphQLError("Library not found")))).toBe("Library not found");
    expect(errorMessage(new Error("boom"))).toBe("boom");
    expect(errorMessage(new Error(""))).toBe("Something went wrong");
    expect(errorMessage("a string")).toBe("a string");
    expect(errorMessage(null)).toBe("Something went wrong");
    expect(errorMessage({ weird: true }, "Could not save")).toBe("Could not save");
  });

  it("turns a failed mutation payload into a throw", () => {
    expect(assertSuccess({ success: true, value: 1 }, "nope")).toEqual({ success: true, value: 1 });
    expect(() => assertSuccess({ success: false, error: "Disk full" }, "nope")).toThrow("Disk full");
    expect(() => assertSuccess({ success: false }, "Could not save")).toThrow("Could not save");
    expect(() => assertSuccess(null, "Could not save")).toThrow("Could not save");
  });
});
