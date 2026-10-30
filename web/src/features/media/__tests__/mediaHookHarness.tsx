import { ApolloClient, ApolloLink, InMemoryCache, type Operation } from "@apollo/client";
import { ApolloProvider } from "@apollo/client/react";
import type { ReactNode } from "react";
import { Observable } from "rxjs";

/** Control only the transport; hooks, observable queries and cache remain real. */
export function mediaHookHarness() {
  const requests: Array<{
    operation: Operation;
    resolve: (data: Record<string, unknown>) => void;
    reject: (error: Error) => void;
  }> = [];
  const client = new ApolloClient({
    cache: new InMemoryCache(),
    link: new ApolloLink((operation) => new Observable((observer) => {
      requests.push({
        operation,
        resolve: (data) => { observer.next({ data }); observer.complete(); },
        reject: (error) => observer.error(error),
      });
    })),
  });
  const wrapper = ({ children }: { children: ReactNode }) => <ApolloProvider client={client}>{children}</ApolloProvider>;
  return { client, requests, wrapper };
}
