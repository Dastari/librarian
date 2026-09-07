import { createFileRoute } from "@tanstack/react-router";

import { WantedPage } from "@/features/wanted/WantedPage";

export const Route = createFileRoute("/_app/wanted/")({
  component: WantedPage,
});
