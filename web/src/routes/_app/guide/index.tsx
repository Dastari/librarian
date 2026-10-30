import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";

import { GuidePage } from "@/features/guide/GuidePage";

const searchSchema = z.object({ view: z.enum(["calendar", "grid"]).optional() });

export const Route = createFileRoute("/_app/guide/")({
  validateSearch: searchSchema,
  component: GuidePage,
});
