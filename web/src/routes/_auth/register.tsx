import { createFileRoute, redirect } from "@tanstack/react-router";
import { z } from "zod";

import { RegisterForm } from "@/features/auth/RegisterForm";
import { NeedsSetupDocument } from "@/graphql/generated/graphql";
import { session } from "@/lib/auth/session";

const searchSchema = z.object({ invite: z.string().optional() });

export const Route = createFileRoute("/_auth/register")({
  validateSearch: searchSchema,
  beforeLoad: async ({ context }) => {
    await session.ready();
    if (session.getSnapshot().status === "authenticated") throw redirect({ to: "/" });
    const { data } = await context.apollo.query({ query: NeedsSetupDocument, fetchPolicy: "network-only" });
    return { needsSetup: Boolean(data?.needsSetup) };
  },
  loader: ({ context }) => ({ needsSetup: context.needsSetup }),
  component: RegisterPage,
});

function RegisterPage() {
  const { needsSetup } = Route.useLoaderData();
  const { invite } = Route.useSearch();
  return <RegisterForm isSetup={needsSetup} inviteToken={invite} />;
}
