import { ApolloProvider } from "@apollo/client/react";
import { ActionProvider } from "glass-ux/actions";
import { PaletteProvider } from "glass-ux/command-palette";
import { TooltipProvider } from "glass-ux/tooltip";
import type { ReactNode } from "react";

import { LiveDataProvider } from "@/lib/graphql/live/provider";
import { apolloClient } from "@/lib/apollo/client";
import { InputModeProvider } from "@/lib/input-mode";
import { ThemeProvider } from "@/lib/theme";

import { router } from "./router";

/**
 * Glass UX owns appearance, tooltips, actions, confirmations and toast hosting.
 */
export function AppProviders({ children }: { children: ReactNode }) {
  return (
    <ApolloProvider client={apolloClient}>
      <LiveDataProvider>
        <ThemeProvider>
          <InputModeProvider>
            <TooltipProvider>
              <ActionProvider navigate={(href) => void router.navigate({ href })}>
                <PaletteProvider>
                  {children}
                </PaletteProvider>
              </ActionProvider>
            </TooltipProvider>
          </InputModeProvider>
        </ThemeProvider>
      </LiveDataProvider>
    </ApolloProvider>
  );
}
