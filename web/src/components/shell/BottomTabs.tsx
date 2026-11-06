import { Link, useNavigate, useRouterState } from "@tanstack/react-router";
import { TabBar } from "glass-ux/shell";

import { PRIMARY_NAV, isNavActive } from "./nav";

/** Phone navigation: Glass UX's floating tab bar, with the other destinations under More. */
export function BottomTabs() {
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const navigate = useNavigate();
  const primary = PRIMARY_NAV.filter((item) => ["home", "libraries", "search", "downloads"].includes(item.key));
  const secondary = PRIMARY_NAV.filter((item) => !primary.includes(item));
  return (
    <TabBar.Root aria-label="Primary" data-print="hide">
      {primary.map((item) => <TabBar.Item key={item.key} label={item.label} icon={<item.icon />} active={isNavActive(item.href, pathname)} render={<Link to={item.href as never} data-focusable />} />)}
      <TabBar.More active={secondary.some((item) => isNavActive(item.href, pathname))}>
        {secondary.map((item) => <TabBar.MoreItem key={item.key} label={item.label} icon={<item.icon />} active={isNavActive(item.href, pathname)} onClick={() => void navigate({ to: item.href as never })} />)}
      </TabBar.More>
    </TabBar.Root>
  );
}
