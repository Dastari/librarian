
import { Button } from "@/components/ui";
import { useRouter } from "@tanstack/react-router";
import type { ErrorComponentProps } from "@tanstack/react-router";
import { IconAlertTriangle } from "@tabler/icons-react";

import { useEffect } from "react";

import { errorMessage } from "@/lib/graphql/errors";
import { reloadOnceForStaleChunk } from "@/main";

const STALE_CHUNK = /Failed to fetch dynamically imported module|Importing a module script failed|error loading dynamically imported module/i;

export function RouteError({ error, reset }: ErrorComponentProps) {
  const router = useRouter();
  const stale = STALE_CHUNK.test(errorMessage(error));
  useEffect(() => {
    if (stale) reloadOnceForStaleChunk();
  }, [stale]);
  return (
    <div role="alert" className="flex min-h-[70svh] flex-col items-center justify-center gap-4 px-6 text-center">
      <IconAlertTriangle size={40} stroke={1.5} className="text-danger" />
      <h1 className="text-display-md text-foreground">{stale ? "Reloading the app" : "Something went wrong"}</h1>
      <p className="max-w-lg text-body text-muted">{stale ? "A newer version of Librarian was loaded. Reload if this page does not refresh by itself." : errorMessage(error)}</p>
      <div className="flex gap-2">
        <Button
          variant="primary"
          onPress={() => {
            reset();
            void router.invalidate();
          }}
        >
          Try again
        </Button>
        <Button variant="secondary" onPress={() => void router.navigate({ to: "/" })}>
          Go home
        </Button>
      </div>
    </div>
  );
}
