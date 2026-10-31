import { ApolloProvider } from "@apollo/client/react";
import { ActionProvider } from "glass-ux/actions";
import { EntityResolverProvider } from "glass-ux/ai";
import { PaletteProvider } from "glass-ux/command-palette";
import { TooltipProvider } from "glass-ux/tooltip";
import type { ReactNode } from "react";

import { LiveDataProvider } from "@/lib/graphql/live/provider";
import { apolloClient } from "@/lib/apollo/client";
import { createEntityResolver } from "@/lib/entities/resolver";
import { InputModeProvider } from "@/lib/input-mode";
import { ThemeProvider } from "@/lib/theme";

import { router } from "./router";

const navigate = (href: string) => void router.navigate({ href });
const entityResolver = createEntityResolver(navigate);

/**
 * Glass UX owns appearance, tooltips, actions, confirmations, toast hosting and the entity
 * resolver that every chip and entity link reads.
 */
export function AppProviders({ children }: { children: ReactNode }) {
  return (
    <ApolloProvider client={apolloClient}>
      <LiveDataProvider>
        <ThemeProvider>
          <InputModeProvider>
            <TooltipProvider>
              <ActionProvider navigate={navigate}>
                <EntityResolverProvider resolver={entityResolver}>
                  <PaletteProvider>
                    {children}
                  </PaletteProvider>
                </EntityResolverProvider>
              </ActionProvider>
            </TooltipProvider>
          </InputModeProvider>
        </ThemeProvider>
      </LiveDataProvider>
    </ApolloProvider>
  );
}
