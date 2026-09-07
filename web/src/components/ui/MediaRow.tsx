import { IconChevronLeft, IconChevronRight } from "@tabler/icons-react";
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
    setCanBack(element.scrollLeft > 8);
    setCanForward(element.scrollLeft + element.clientWidth < element.scrollWidth - 8);
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
    element.scrollBy({ left: direction * element.clientWidth * 0.85, behavior: "smooth" });
  };

  return (
    <div className={cn("group/row relative", className)}>
      <div ref={ref} role="list" aria-label={ariaLabel} className="snap-row page-gutter">
        {children}
      </div>
      <RowArrow side="left" visible={canBack} onPress={() => page(-1)} />
      <RowArrow side="right" visible={canForward} onPress={() => page(1)} />
    </div>
  );
}

function RowArrow({ side, visible, onPress }: { side: "left" | "right"; visible: boolean; onPress: () => void }) {
  const Icon = side === "left" ? IconChevronLeft : IconChevronRight;
  return (
    <button
      type="button"
      tabIndex={-1}
      aria-hidden
      onClick={onPress}
      className={cn(
        "glass-control absolute top-1/2 z-10 hidden size-11 -translate-y-1/2 place-items-center rounded-full text-foreground opacity-0 transition-opacity duration-fast hover-capable:grid group-hover/row:opacity-100 tv:hidden",
        !visible && "pointer-events-none group-hover/row:opacity-0",
        side === "left" ? "left-[calc(var(--page-gutter)/2-1.375rem)]" : "right-[calc(var(--page-gutter)/2-1.375rem)]",
      )}
    >
      <Icon size={22} />
    </button>
  );
}
