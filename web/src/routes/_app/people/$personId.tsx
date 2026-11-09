import { useLiveRoute } from "@/lib/graphql/live/hooks";
import { createFileRoute } from "@tanstack/react-router";

import { PersonPage } from "@/features/people/PersonPage";
import { PersonDetailDocument } from "@/graphql/generated/graphql";

export const Route = createFileRoute("/_app/people/$personId")({
  loader: async ({ context, params }) => {
    const { data } = await context.apollo.query({ query: PersonDetailDocument, variables: { id: params.personId }, fetchPolicy: "cache-first" });
    return { crumb: data?.person?.name ?? "Person" };
  },
  component: () => {
    const { personId } = Route.useParams();
    useLiveRoute(PersonDetailDocument, { id: personId });
    return <PersonPage personId={personId} />;
  },
});
