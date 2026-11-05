import { useEffect, useRef } from "react";

const EVENT = "librarian:focus-search";

/** Puts the caret in the search page's own field: the top bar's box, pressed while already there. */
export function requestSearchFocus() {
  window.dispatchEvent(new Event(EVENT));
}

export function useSearchFocusRequest(focus: () => void) {
  const latest = useRef(focus);
  latest.current = focus;
  useEffect(() => {
    const onRequest = () => latest.current();
    window.addEventListener(EVENT, onRequest);
    return () => window.removeEventListener(EVENT, onRequest);
  }, []);
}
