import type { MockLink } from "@apollo/client/testing";
import { MockedProvider } from "@apollo/client/testing/react";
import { ToastProvider } from "@heroui/react";
import { render, type RenderOptions, type RenderResult } from "@testing-library/react";
import type { ReactElement, ReactNode } from "react";


import { InputModeProvider } from "@/lib/input-mode";
import { ThemeProvider } from "@/lib/theme";

import { TestRouterProvider, createTestRouter, type TestRouter, type TestRouterOptions } from "./router";

export interface RenderWithProvidersOptions extends TestRouterOptions {
  /** Apollo mocks; omit for components that do not query. */
  mocks?: ReadonlyArray<MockLink.MockedResponse<any, any>>;
  /** Mount the router (needed by anything using Link/useNavigate). Defaults to true. */
  router?: boolean;
  /** Mount the theme and input-mode providers. Defaults to false; they touch <html>. */
  shell?: boolean;
  renderOptions?: Omit<RenderOptions, "wrapper">;
}

export interface RenderWithProvidersResult extends RenderResult {
  router: TestRouter | null;
}

/**
 * Renders a component with the providers it can reasonably expect: Apollo (mocked), the toast
 * outlet HeroUI mutations write to, an optional memory router and the theme/input-mode context.
 */
export async function renderWithProviders(ui: ReactElement, options: RenderWithProvidersOptions = {}): Promise<RenderWithProvidersResult> {
  const { mocks = [], router: withRouter = true, shell = false, renderOptions, ...routerOptions } = options;
  // Mocks are single-use by default, which breaks any component that refetches. Reuse them
  // unless a test deliberately caps a mock to assert how often it is asked for.
  const reusable = mocks.map((mock) => ({ maxUsageCount: Number.POSITIVE_INFINITY, ...mock }));

  const wrap = (children: ReactNode): ReactNode => {
    let tree: ReactNode = (
      <>
        {children}
        <ToastProvider placement="bottom end" />
      </>
    );
    if (shell) {
      tree = (
        <ThemeProvider>
          <InputModeProvider>{tree}</InputModeProvider>
        </ThemeProvider>
      );
    }
    return (
      <MockedProvider mocks={reusable} showWarnings={false}>
        {tree}
      </MockedProvider>
    );
  };

  if (!withRouter) {
    return { ...render(<>{wrap(ui)}</>, renderOptions), router: null };
  }
  const testRouter = createTestRouter(wrap(ui), routerOptions);
  // Resolve the initial match first so the component is on screen when this returns.
  await testRouter.load();
  return { ...render(<TestRouterProvider router={testRouter} />, renderOptions), router: testRouter };
}
