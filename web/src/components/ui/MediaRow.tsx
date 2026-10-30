import { IconChevronLeft, IconChevronRight } from "glass-ux/icons";
import { Button } from "glass-ux/button";
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";

import { cn } from "@/lib/utils";

interface MediaRowProps {
  children: ReactNode;
  className?: string;
  ariaLabel: string;
}

/**
 * Horizontal, snap-scrolling row of cards with edge arrows on pointer devices. The row is
 * gutter-aware so the first card lines up with page headings while the last one bleeds to the
 * viewport edge.
 */
export function MediaRow({ children, className, ariaLabel }: MediaRowProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [canBack, setCanBack] = useState(false);
  const [canForward, setCanForward] = useState(false);

  const update = useCallback(() => {
    const element = ref.current;
    if (!element) return;
    const offset = Math.abs(element.scrollLeft);
    setCanBack(offset > 8);
    setCanForward(offset + element.clientWidth < element.scrollWidth - 8);
  }, []);

  useEffect(() => {
    const element = ref.current;
    if (!element) return;
    update();
    element.addEventListener("scroll", update, { passive: true });
    const observer = new ResizeObserver(update);
    observer.observe(element);
    return () => {
      element.removeEventListener("scroll", update);
      observer.disconnect();
    };
  }, [update, children]);

  const page = (direction: 1 | -1) => {
    const element = ref.current;
    if (!element) return;
    const inlineDirection = getComputedStyle(element).direction === "rtl" ? -1 : 1;
    element.scrollBy({ left: inlineDirection * direction * element.clientWidth * 0.85, behavior: "smooth" });
  };

  return (
    <div className={cn("group/row relative", className)}>
      <div ref={ref} role="list" aria-label={ariaLabel} data-nav-group className="snap-row page-gutter">
        {children}
      </div>
      <RowArrow side="start" visible={canBack} onPress={() => page(-1)} />
      <RowArrow side="end" visible={canForward} onPress={() => page(1)} />
    </div>
  );
}

function RowArrow({ side, visible, onPress }: { side: "start" | "end"; visible: boolean; onPress: () => void }) {
  const Icon = side === "start" ? IconChevronLeft : IconChevronRight;
  return (
    <Button
      variant="secondary" shape="circle" icon={<Icon className="rtl:rotate-180" />} aria-label={side === "start" ? "Previous items" : "Next items"}
      onClick={onPress} disabled={!visible} data-print="hide"
      className={cn("absolute top-1/2 z-10 -translate-y-1/2", !visible && "invisible", side === "start" ? "start-2" : "end-2")}
    />
  );
}
