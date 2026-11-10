import { createFileRoute } from "@tanstack/react-router";

import { NowPlayingPage } from "@/features/statistics/NowPlayingPage";

export const Route = createFileRoute("/_app/settings/statistics/")({
  component: NowPlayingPage,
});
