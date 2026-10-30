import { Panel as GlassPanel, PanelBody, PanelHeader } from "glass-ux/surface";
import type { ReactNode } from "react";

interface PanelProps {
  children: ReactNode;
  className?: string;
  title?: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  flush?: boolean;
  tone?: "default" | "secondary" | "glass";
}

export function Panel({ children, className, title, description, actions, flush, tone = "default" }: PanelProps) {
  return (
    <GlassPanel glass={tone === "secondary" ? "solid" : undefined} className={className}>
      {title || actions || description ? <PanelHeader title={title} description={description} actions={actions} level={3} /> : null}
      {flush ? children : <PanelBody>{children}</PanelBody>}
    </GlassPanel>
  );
}
