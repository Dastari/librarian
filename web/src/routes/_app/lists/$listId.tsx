import { createFileRoute } from "@tanstack/react-router";
import { PersonalListPage } from "@/features/personal/PersonalListPage";
export const Route = createFileRoute("/_app/lists/$listId")({
  staticData: { fixedHeight: true },
  component: () => <PersonalListPage listId={Route.useParams().listId} />,
});
