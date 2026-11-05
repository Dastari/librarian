import { useLayoutEffect, useRef, type ComponentProps } from "react";

import { cn } from "@/lib/utils";

interface PageScrollerProps extends ComponentProps<"div"> {
  /** False for a bounded pane whose children own their scrolling: nothing here can scroll. */
  scrolling?: boolean;
}

/**
 * The one scroller of a page. `--view-h` on it is its visible height, which heroes and shelves
 * use to share exactly the first screen. A bounded pane uses `overflow: clip`, not `hidden`:
 * a hidden box is still scrollable from code, so a focus or reveal could slide the page away
 * with no way back for the person using it.
 */
export function PageScroller({ scrolling = true, className, children, ...rest }: PageScrollerProps) {
  const ref = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const element = ref.current;
    if (!element) return;
    const measure = () => element.style.setProperty("--view-h", `${element.clientHeight}px`);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  return (
    <div {...rest} ref={ref} data-page-scroller={scrolling ? "" : undefined} className={cn("min-h-0 min-w-0 overflow-x-clip", scrolling ? "scrollbar-thin overflow-y-auto" : "overflow-y-clip", className)}>
      {children}
    </div>
  );
}
