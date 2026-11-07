import { createFileRoute } from "@tanstack/react-router";

/** The player itself lives above the routes (`GlobalVideo`), and fills the screen while this is open. */
export const Route = createFileRoute("/_player/watch/$mediaFileId")({
  component: () => <div className="h-dvh w-full bg-black" />,
});
