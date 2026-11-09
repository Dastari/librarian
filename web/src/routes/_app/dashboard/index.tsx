import { createFileRoute } from "@tanstack/react-router";

import { NowPlayingPage } from "@/features/dashboard/NowPlayingPage";

export const Route = createFileRoute("/_app/dashboard/")({
  component: NowPlayingPage,
});
