
import { Button } from "@/components/ui";
import { Link } from "@tanstack/react-router";
import { IconCompass } from "@tabler/icons-react";

export function NotFound() {
  return (
    <div className="flex min-h-[70svh] flex-col items-center justify-center gap-4 px-6 text-center">
      <IconCompass size={40} stroke={1.5} className="text-muted" />
      <h1 className="text-display-md text-foreground">This page doesn't exist</h1>
      <p className="text-body text-muted">The link may be old or the item may have been removed.</p>
      <Link to="/">
        <Button variant="primary">Go home</Button>
      </Link>
    </div>
  );
}
