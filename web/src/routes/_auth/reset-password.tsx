import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";

import { ResetPasswordForm } from "@/features/auth/ResetPasswordForm";

const searchSchema = z.object({ token: z.string().optional() });

/** A one-time reset link an administrator passed on (design.md Q148). Works signed out. */
export const Route = createFileRoute("/_auth/reset-password")({
  validateSearch: searchSchema,
  component: () => <ResetPasswordForm token={Route.useSearch().token ?? ""} />,
});
