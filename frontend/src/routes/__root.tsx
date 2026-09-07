import {
  Outlet,
  createRootRouteWithContext,
  redirect,
  useRouteContext,
  useRouter,
  type ErrorComponentProps,
} from "@tanstack/react-router";
import { TanStackRouterDevtoolsPanel } from "@tanstack/react-router-devtools";
import { TanStackDevtools } from "@tanstack/react-devtools";
import { Button } from "@heroui/button";
import { Card, CardBody } from "@heroui/card";
import { IconAlertTriangle } from "@tabler/icons-react";
import { Navbar } from "../components/Navbar";
import { NotFound } from "../components/NotFound";
import { ErrorLogToaster } from "../components/ErrorLogToaster";
import { GraphQLErrorToaster } from "../components/GraphQLErrorToaster";
import { PersistentPlayer } from "../components/PersistentPlayer";
import { PersistentAudioPlayer } from "../components/PersistentAudioPlayer";
import { ServerDisconnectedOverlay } from "../components/ServerDisconnectedOverlay";
import { CastControlBar } from "../components/cast";
import {
  PlaybackProvider,
  usePlaybackContext,
} from "../contexts/PlaybackContext";
import type { AuthContext } from "../lib/auth-context";
import { ensureAuthenticated } from "../lib/route-auth";

interface RouterContext {
  auth: AuthContext;
}

export const Route = createRootRouteWithContext<RouterContext>()({
  beforeLoad: async ({ context, location }) => {
    const path = location.pathname;
    if (path === "/" || path.startsWith("/auth/login")) {
      return;
    }

    const ok = await ensureAuthenticated(context.auth);
    if (!ok) {
      throw redirect({
        to: "/",
        search: {
          signin: true,
          redirect: location.href,
        },
      });
    }
  },
  component: RootLayout,
  notFoundComponent: NotFound,
  errorComponent: RootErrorComponent,
});

function RootErrorComponent({ error, reset }: ErrorComponentProps) {
  const router = useRouter();

  return (
    <>
      <Navbar />
      <main className="flex grow items-center justify-center p-6">
        <Card className="max-w-lg w-full">
          <CardBody className="text-center space-y-4">
            <IconAlertTriangle size={48} className="text-danger-400 mx-auto" />
            <h1 className="text-xl font-bold text-danger">
              Something went wrong
            </h1>
            <p className="text-default-500">
              {error instanceof Error
                ? error.message
                : "An unexpected error occurred"}
            </p>
            {import.meta.env.DEV && error instanceof Error && error.stack && (
              <details className="text-left mt-4">
                <summary className="cursor-pointer text-sm text-default-400 hover:text-default-600">
                  Stack trace (dev only)
                </summary>
                <pre className="mt-2 p-3 bg-default-100 rounded-lg text-xs overflow-auto max-h-64 text-left">
                  {error.stack}
                </pre>
              </details>
            )}
            <div className="flex gap-2 justify-center pt-4">
              <Button
                color="primary"
                onPress={() => {
                  reset();
                  router.invalidate();
                }}
              >
                Try Again
              </Button>
              <Button
                variant="flat"
                onPress={() => router.navigate({ to: "/" })}
              >
                Go Home
              </Button>
            </div>
          </CardBody>
        </Card>
      </main>
    </>
  );
}

function RootLayoutContent() {
  const { auth } = useRouteContext({ from: "__root__" });
  const { session } = usePlaybackContext();
  const contentType = (session?.contentType ?? "").toUpperCase();

  // Check if audio player is visible (track or audiobook playing)
  const isAudioPlayerVisible =
    contentType === "TRACK" || contentType === "AUDIOBOOK";

  return (
    <div className="flex h-screen min-h-0 flex-col">
      <main
        className="flex min-h-0 flex-1 flex-col overflow-y-auto"
        style={{
          scrollbarGutter: "stable",
          paddingBottom: isAudioPlayerVisible ? 80 : 0,
        }}
      >
        <Navbar />
        <Outlet />
      </main>

      {/* GraphQL error toaster - shows toast for GraphQL/network errors */}
      <GraphQLErrorToaster />

      {auth.isAuthenticated ? (
        <>
          {/* Authenticated backend log stream. */}
          <ErrorLogToaster />

          {/* Persistent media and casting controls query protected data. */}
          <PersistentPlayer />
          <PersistentAudioPlayer />
          <CastControlBar />

          {/* Authenticated GraphQL websocket health. */}
          <ServerDisconnectedOverlay />
        </>
      ) : null}

      {/* Dev tools - only in development */}
      {import.meta.env.DEV ? (
        <TanStackDevtools
          config={{
            position: "bottom-right",
          }}
          plugins={[
            {
              name: "Tanstack Router",
              render: <TanStackRouterDevtoolsPanel />,
            },
          ]}
        />
      ) : null}
    </div>
  );
}

function RootLayout() {
  return (
    <PlaybackProvider>
      <RootLayoutContent />
    </PlaybackProvider>
  );
}
