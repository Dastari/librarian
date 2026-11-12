import { useEffect, useRef, useState } from "react";

import { Scrubber, type ScrubState } from "./scrub";

interface UseScrubberOptions {
  position: () => number;
  duration: () => number;
  commit: (target: number) => void;
}

/** A {@link Scrubber} for a component, with its state to draw a preview from. */
export function useScrubber(options: UseScrubberOptions): { scrub: ScrubState | null; scrubber: Scrubber } {
  const latest = useRef(options);
  latest.current = options;
  const [scrub, setScrub] = useState<ScrubState | null>(null);
  const [scrubber] = useState(() => new Scrubber({
    position: () => latest.current.position(),
    duration: () => latest.current.duration(),
    commit: (target) => latest.current.commit(target),
    onChange: setScrub,
  }));
  useEffect(() => () => scrubber.dispose(), [scrubber]);
  return { scrub, scrubber };
}
