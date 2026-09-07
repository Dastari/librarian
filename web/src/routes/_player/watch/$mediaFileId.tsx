import { createFileRoute, useNavigate } from "@tanstack/react-router";

import { WatchPage } from "@/features/player/WatchPage";

export const Route = createFileRoute("/_player/watch/$mediaFileId")({
  component: () => {
    const { mediaFileId } = Route.useParams();
    const navigate = useNavigate();
    return <WatchPage mediaFileId={mediaFileId} onBack={() => (window.history.length > 1 ? window.history.back() : void navigate({ to: "/" }))} />;
  },
});
