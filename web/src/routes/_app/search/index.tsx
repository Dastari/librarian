import { createFileRoute } from "@tanstack/react-router";

import { SearchPage } from "@/features/search/SearchPage";

export const Route = createFileRoute("/_app/search/")({
  component: SearchPage,
});
