import { ApolloProvider } from "@apollo/client/react";
import { ToastProvider } from "@heroui/react";
import type { ReactNode } from "react";
import { RouterProvider as AriaRouterProvider } from "react-aria-components";

import { apolloClient } from "@/lib/apollo/client";
import { InputModeProvider } from "@/lib/input-mode";
import { ThemeProvider } from "@/lib/theme";

import { router } from "./router";

/**
 * Global providers. The react-aria RouterProvider makes every HeroUI `href` (links, tabs, menu
 * items) navigate client-side through TanStack Router.
 */
export function AppProviders({ children }: { children: ReactNode }) {
  return (
    <ApolloProvider client={apolloClient}>
      <ThemeProvider>
        <InputModeProvider>
          <AriaRouterProvider navigate={(href) => void router.navigate({ href })} useHref={(href) => href}>
            {children}
            <ToastProvider placement="bottom end" maxVisibleToasts={3} />
          </AriaRouterProvider>
        </InputModeProvider>
      </ThemeProvider>
    </ApolloProvider>
  );
}
