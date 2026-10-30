import { useEffect, useState } from "react";
import { flushSync } from "react-dom";

/** Print the data already in memory; never trigger continuation to fill paper. */
export function usePrintMode(): boolean {
  const [printing, setPrinting] = useState(false);
  useEffect(() => {
    const media = window.matchMedia("print");
    const before = () => flushSync(() => setPrinting(true));
    const after = () => setPrinting(false);
    const changed = (event: MediaQueryListEvent) => {
      if (event.matches) before(); else after();
    };
    window.addEventListener("beforeprint", before);
    window.addEventListener("afterprint", after);
    media.addEventListener("change", changed);
    if (media.matches) setPrinting(true);
    return () => {
      window.removeEventListener("beforeprint", before);
      window.removeEventListener("afterprint", after);
      media.removeEventListener("change", changed);
    };
  }, []);
  return printing;
}
