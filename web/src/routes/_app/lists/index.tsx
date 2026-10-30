import { createFileRoute } from "@tanstack/react-router";
import { PersonalListsPage } from "@/features/personal/PersonalListsPage";
export const Route = createFileRoute("/_app/lists/")({ component: PersonalListsPage });
