const SCHEMA_VALIDATION_ERROR =
  /^(Unknown (argument|field|type)|Cannot query field|Field ".+" of required type)/i;

/**
 * Build one actionable notification for a GraphQL operation. A single
 * operation can return many validation errors, but surfacing each as its own
 * toast obscures the actual problem and can cover authentication forms.
 */
export function summarizeGraphQLErrors(
  operationName: string,
  messages: readonly string[],
): string {
  const uniqueMessages = [
    ...new Set(messages.map((message) => message.trim())),
  ].filter(Boolean);

  if (uniqueMessages.length === 0) {
    return `GraphQL operation ${operationName} failed`;
  }

  if (uniqueMessages.some((message) => SCHEMA_VALIDATION_ERROR.test(message))) {
    return [
      `Frontend/backend GraphQL schema mismatch in ${operationName}.`,
      uniqueMessages.join(" "),
      "Rebuild and restart the backend, then reload the frontend.",
    ].join(" ");
  }

  if (uniqueMessages.length === 1) {
    return uniqueMessages[0];
  }

  const remaining = uniqueMessages.length - 1;
  return `${uniqueMessages[0]} (+${remaining} more ${
    remaining === 1 ? "error" : "errors"
  } in ${operationName})`;
}
