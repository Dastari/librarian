declare function queryPromise<TData = any, TVariables = Record<string, unknown>>(
  query: any,
  variables?: TVariables,
): Promise<{ data?: TData; error?: Error }>;

declare function mutationPromise<TData = any, TVariables = Record<string, unknown>>(
  mutation: any,
  variables?: TVariables,
): Promise<{ data?: TData; error?: Error }>;

declare function subscriptionStream<TData = any, TVariables = Record<string, unknown>>(
  subscription: any,
  variables?: TVariables,
): any;

declare const graphqlClient: {
  query<TData = any, TVariables = Record<string, unknown>>(
    query: any,
    variables?: TVariables,
  ): Promise<{ data?: TData; error?: Error }>;
  mutation<TData = any, TVariables = Record<string, unknown>>(
    mutation: any,
    variables?: TVariables,
  ): Promise<{ data?: TData; error?: Error }>;
  subscription<TData = any, TVariables = Record<string, unknown>>(
    subscription: any,
    variables?: TVariables,
  ): any;
};
