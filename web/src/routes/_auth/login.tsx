import { createFileRoute, redirect } from "@tanstack/react-router";
import { z } from "zod";

import { LoginForm } from "@/features/auth/LoginForm";
import { NeedsSetupDocument } from "@/graphql/generated/graphql";
import { session } from "@/lib/auth/session";

const searchSchema = z.object({ redirect: z.string().optional() });

export const Route = createFileRoute("/_auth/login")({
  validateSearch: searchSchema,
  beforeLoad: async ({ context, search }) => {
    await session.ready();
    if (session.getSnapshot().status === "authenticated") {
      throw redirect({ href: search.redirect && search.redirect.startsWith("/") ? search.redirect : "/" });
    }
    const { data } = await context.apollo.query({ query: NeedsSetupDocument, fetchPolicy: "network-only" });
    if (data?.needsSetup) throw redirect({ to: "/register" });
  },
  component: LoginPage,
});

function LoginPage() {
  const { redirect: redirectTo } = Route.useSearch();
  return <LoginForm redirectTo={redirectTo} allowRegister />;
}
