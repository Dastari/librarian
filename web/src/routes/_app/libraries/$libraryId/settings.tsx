import { createFileRoute, redirect } from "@tanstack/react-router";

/** A library's settings live in its dialog; this keeps old links working. */
export const Route = createFileRoute("/_app/libraries/$libraryId/settings")({
  beforeLoad: ({ params }) => {
    throw redirect({ href: `/libraries/${params.libraryId}/files?settings=general`, replace: true });
  },
});
