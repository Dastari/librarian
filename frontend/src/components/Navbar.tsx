import { Link, useLocation, useNavigate } from "@tanstack/react-router";
import {
  Navbar as HeroNavbar,
  NavbarBrand,
  NavbarContent,
  NavbarItem,
  NavbarMenuToggle,
  NavbarMenu,
  NavbarMenuItem,
} from "@heroui/navbar";
import { Button } from "@heroui/button";
import {
  Dropdown,
  DropdownTrigger,
  DropdownMenu,
  DropdownItem,
} from "@heroui/dropdown";
import { Avatar } from "@heroui/avatar";
import { Badge } from "@heroui/badge";
import { Tooltip } from "@heroui/tooltip";
import { Kbd } from "@heroui/kbd";
import { useDisclosure } from "@heroui/modal";
import { useState, useEffect } from "react";
import { useAuth } from "../hooks/useAuth";
import { useTheme } from "../hooks/useTheme";
import {
  IconBell,
  IconDownload,
  IconSearch,
  IconSun,
  IconMoon,
} from "@tabler/icons-react";
import { SearchModal } from "./SearchModal";
import { NotificationPopover } from "./NotificationPopover";
import {
  graphqlClient,
  ACTIVE_DOWNLOAD_COUNT_QUERY,
  ACTIVE_DOWNLOAD_COUNT_SUBSCRIPTION,
  NOTIFICATION_COUNTS_QUERY,
  NOTIFICATION_COUNTS_SUBSCRIPTION,
  type ActiveDownloadCount,
  type NotificationCounts,
} from "../lib/graphql";

const navItems = [
  { to: "/", label: "Home" },
  { to: "/libraries", label: "Libraries" },
  { to: "/hunt", label: "Hunt" },
  { to: "/downloads", label: "Downloads" },
  { to: "/settings", label: "Settings" },
];

export function Navbar() {
  const { user, signOut, loading, error } = useAuth();
  const { isDark, toggleTheme } = useTheme();
  const [isMenuOpen, setIsMenuOpen] = useState(false);
  const [activeDownloadCount, setActiveDownloadCount] = useState(0);
  const [unreadNotificationCount, setUnreadNotificationCount] = useState(0);
  const {
    isOpen: isSearchOpen,
    onOpen: onSearchOpen,
    onClose: onSearchClose,
  } = useDisclosure();
  const location = useLocation();
  const navigate = useNavigate();

  // Keyboard shortcut for search (Cmd/Ctrl + K)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        onSearchOpen();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onSearchOpen]);

  // Fetch initial count and subscribe to lightweight updates
  useEffect(() => {
    if (!user) {
      setActiveDownloadCount(0);
      return;
    }

    // Fetch initial count
    graphqlClient
      .query<{ activeDownloadCount: number }>(ACTIVE_DOWNLOAD_COUNT_QUERY, {})
      .toPromise()
      .then((result) => {
        if (result.data?.activeDownloadCount !== undefined) {
          setActiveDownloadCount(result.data.activeDownloadCount);
        }
      });

    // Subscribe to active download count changes
    // This only fires when the count changes, not on every progress update
    const countSub = graphqlClient
      .subscription<{
        activeDownloadCount: ActiveDownloadCount;
      }>(ACTIVE_DOWNLOAD_COUNT_SUBSCRIPTION, {})
      .subscribe({
        next: (result) => {
          if (result.data?.activeDownloadCount) {
            setActiveDownloadCount(result.data.activeDownloadCount.count);
          }
        },
      });

    return () => {
      countSub.unsubscribe();
    };
  }, [user]);

  // Fetch notification count and subscribe to updates
  useEffect(() => {
    if (!user) {
      setUnreadNotificationCount(0);
      return;
    }

    // Fetch initial count
    graphqlClient
      .query<{ notificationCounts: NotificationCounts }>(
        NOTIFICATION_COUNTS_QUERY,
        {},
      )
      .toPromise()
      .then((result) => {
        if (result.data?.notificationCounts) {
          setUnreadNotificationCount(
            result.data.notificationCounts.unreadCount,
          );
        }
      });

    // Subscribe to notification count changes
    const countSub = graphqlClient
      .subscription<{
        notificationCounts: NotificationCounts;
      }>(NOTIFICATION_COUNTS_SUBSCRIPTION, {})
      .subscribe({
        next: (result) => {
          if (result.data?.notificationCounts) {
            setUnreadNotificationCount(
              result.data.notificationCounts.unreadCount,
            );
          }
        },
      });

    return () => {
      countSub.unsubscribe();
    };
  }, [user]);

  const isActive = (path: string) => {
    if (path === "/") return location.pathname === "/";
    return location.pathname.startsWith(path);
  };

  return (
    <HeroNavbar
      isBordered
      position="sticky"
      isBlurred
      isMenuOpen={isMenuOpen}
      onMenuOpenChange={setIsMenuOpen}
      classNames={{
        base: "border-none bg-transparent",
        wrapper: "container max-w-auto bg-transparent ",
      }}
    >
      {/* Mobile menu toggle */}
      <NavbarContent className="sm:hidden" justify="start">
        <NavbarMenuToggle
          aria-label={isMenuOpen ? "Close menu" : "Open menu"}
        />
      </NavbarContent>

      {/* Brand */}
      <NavbarContent className="sm:hidden pr-3" justify="center">
        <NavbarBrand>
          <Link to="/" className="flex items-center gap-2">
            <img src="/logo.svg" alt="Librarian" className="h-7 w-7" />
            <span
              className="text-xl"
              style={{ fontFamily: '"Playwrite Australia SA", cursive' }}
            >
              Librarian
            </span>
          </Link>
        </NavbarBrand>
      </NavbarContent>

      <NavbarContent className="hidden sm:flex gap-4" justify="start">
        <NavbarBrand>
          <Link to="/" className="flex items-center gap-2">
            <img src="/logo.svg" alt="Librarian" className="h-7 w-7" />
            <span
              className="text-xl"
              style={{ fontFamily: '"Playwrite Australia SA", cursive' }}
            >
              Librarian
            </span>
          </Link>
        </NavbarBrand>
      </NavbarContent>

      {/* Desktop navigation */}
      {user && (
        <NavbarContent className="hidden sm:flex gap-1" justify="center">
          {navItems.map((item) => (
            <NavbarItem key={item.to} isActive={isActive(item.to)}>
              <Button
                className="text-sm font-semibold"
                as={Link}
                to={item.to}
                variant={"light"}
                color={isActive(item.to) ? "primary" : "default"}
                size="sm"
              >
                {item.label}
              </Button>
            </NavbarItem>
          ))}
        </NavbarContent>
      )}

      {/* Right side - search, download indicator, theme toggle & auth status */}
      <NavbarContent justify="end" className="gap-2">
        {/* Library search button - only show when logged in */}
        {user && (
          <NavbarItem className="hidden sm:flex">
            <Button
              variant="light"
              color="primary"
              size="sm"
              startContent={<IconSearch size={16} />}
              endContent={
                <Kbd keys={["command"]} className="hidden lg:inline-flex">
                  K
                </Kbd>
              }
              onPress={onSearchOpen}
              className="min-w-[180px] justify-start"
            >
              <span className="grow text-sm font-semibold">Search...</span>
            </Button>
          </NavbarItem>
        )}

        {/* Download indicator - only show when logged in */}
        {user && (
          <NavbarItem>
            <Tooltip
              content={
                activeDownloadCount > 0
                  ? `${activeDownloadCount} active download${activeDownloadCount !== 1 ? "s" : ""}`
                  : "No active downloads"
              }
            >
              <Button
                isIconOnly
                variant="light"
                size="sm"
                as={Link}
                to="/downloads"
                aria-label={`${activeDownloadCount} active downloads`}
              >
                <Badge
                  content={activeDownloadCount}
                  color="primary"
                  size="sm"
                  isInvisible={activeDownloadCount === 0}
                  showOutline={false}
                >
                  <IconDownload size={20} className="text-blue-400" />
                </Badge>
              </Button>
            </Tooltip>
          </NavbarItem>
        )}

        {/* Notification indicator - only show when logged in */}
        {user && (
          <NavbarItem>
            <NotificationPopover
              trigger={
                <Button
                  isIconOnly
                  variant="light"
                  size="sm"
                  aria-label={`${unreadNotificationCount} unread notifications`}
                >
                  <Badge
                    content={unreadNotificationCount}
                    color="warning"
                    size="sm"
                    isInvisible={unreadNotificationCount === 0}
                    showOutline={false}
                  >
                    <IconBell size={20} className="text-amber-400" />
                  </Badge>
                </Button>
              }
            />
          </NavbarItem>
        )}

        {/* Theme toggle */}
        <NavbarItem>
          <Tooltip
            content={isDark ? "Switch to light mode" : "Switch to dark mode"}
          >
            <Button
              isIconOnly
              variant="light"
              size="sm"
              onPress={toggleTheme}
              aria-label={
                isDark ? "Switch to light mode" : "Switch to dark mode"
              }
            >
              {isDark ? (
                <IconSun size={20} className="text-default-500" />
              ) : (
                <IconMoon size={20} className="text-default-500" />
              )}
            </Button>
          </Tooltip>
        </NavbarItem>

        {error ? (
          <NavbarItem>
            <span className="text-danger text-sm">Auth error</span>
          </NavbarItem>
        ) : loading ? (
          <NavbarItem>
            <span className="text-default-500 text-sm">Loading...</span>
          </NavbarItem>
        ) : user ? (
          <NavbarItem>
            <Dropdown placement="bottom-end">
              <DropdownTrigger>
                <Avatar
                  isBordered
                  as="button"
                  className="transition-transform"
                  color="primary"
                  name={user.email?.charAt(0).toUpperCase()}
                  size="sm"
                />
              </DropdownTrigger>
              <DropdownMenu aria-label="User actions" variant="flat">
                <DropdownItem key="profile" className="h-14 gap-2">
                  <p className="font-semibold">Signed in as</p>
                  <p className="text-default-500">{user.email}</p>
                </DropdownItem>
                <DropdownItem
                  key="notifications"
                  onPress={() => navigate({ to: "/notifications" })}
                >
                  Notifications
                </DropdownItem>
                <DropdownItem
                  key="settings"
                  onPress={() => navigate({ to: "/settings" })}
                >
                  Settings
                </DropdownItem>
                <DropdownItem
                  key="logout"
                  color="danger"
                  onPress={() => signOut()}
                >
                  Sign Out
                </DropdownItem>
              </DropdownMenu>
            </Dropdown>
          </NavbarItem>
        ) : (
          <NavbarItem>
            <Button
              color="primary"
              variant="shadow"
              size="sm"
              onPress={() => navigate({ to: "/", search: { signin: true } })}
            >
              Sign In
            </Button>
          </NavbarItem>
        )}
      </NavbarContent>

      {/* Mobile menu */}
      <NavbarMenu>
        {user &&
          navItems.map((item) => (
            <NavbarMenuItem key={item.to}>
              <Link
                to={item.to}
                className={`w-full ${
                  isActive(item.to)
                    ? "text-primary font-semibold"
                    : "text-foreground"
                }`}
                onClick={() => setIsMenuOpen(false)}
              >
                {item.label}
              </Link>
            </NavbarMenuItem>
          ))}
        {user && (
          <NavbarMenuItem>
            <Button
              color="danger"
              variant="flat"
              className="w-full mt-4"
              onPress={() => {
                signOut();
                setIsMenuOpen(false);
              }}
            >
              Sign Out
            </Button>
          </NavbarMenuItem>
        )}
      </NavbarMenu>

      {/* Search Modal */}
      <SearchModal isOpen={isSearchOpen} onClose={onSearchClose} />
    </HeroNavbar>
  );
}
