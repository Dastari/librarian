import { Link, useLocation, useNavigate } from '@tanstack/react-router'
import { Navbar as HeroNavbar, NavbarBrand, NavbarContent, NavbarItem, NavbarMenuToggle, NavbarMenu, NavbarMenuItem } from '@heroui/navbar'
import { Button } from '@heroui/button'
import { Dropdown, DropdownTrigger, DropdownMenu, DropdownItem } from '@heroui/dropdown'
import { Avatar } from '@heroui/avatar'
import { Chip } from '@heroui/chip'
import { Tooltip } from '@heroui/tooltip'
import { useState } from 'react'
import { Sun, Moon } from 'lucide-react'
import { useAuth } from '../hooks/useAuth'
import { useTheme } from '../hooks/useTheme'
import { IconAlertTriangle } from '@tabler/icons-react'

const navItems = [
  { to: '/', label: 'Home' },
  { to: '/libraries', label: 'Libraries' },
  { to: '/downloads', label: 'Downloads' },
  { to: '/subscriptions', label: 'Subscriptions' },
  { to: '/settings', label: 'Settings' },
]

export function Navbar() {
  const { user, signOut, loading, error, isConfigured } = useAuth()
  const { isDark, toggleTheme } = useTheme()
  const [isMenuOpen, setIsMenuOpen] = useState(false)
  const location = useLocation()
  const navigate = useNavigate()

  const isActive = (path: string) => {
    if (path === '/') return location.pathname === '/'
    return location.pathname.startsWith(path)
  }

  return (
    <HeroNavbar
      isBordered
      position="sticky"
      isBlurred
      isMenuOpen={isMenuOpen}
      onMenuOpenChange={setIsMenuOpen}
      classNames={{
        base: 'border-none bg-transparent',
        wrapper: 'container max-w-auto bg-transparent ',
      }}
    >
      {/* Mobile menu toggle */}
      <NavbarContent className="sm:hidden" justify="start">
        <NavbarMenuToggle aria-label={isMenuOpen ? 'Close menu' : 'Open menu'} />
      </NavbarContent>

      {/* Brand */}
      <NavbarContent className="sm:hidden pr-3" justify="center">
        <NavbarBrand>
          <Link to="/" className="flex items-center gap-2">
            <span className="text-xl font-bold">📚 Librarian</span>
          </Link>
        </NavbarBrand>
      </NavbarContent>

      <NavbarContent className="hidden sm:flex gap-4" justify="start">
        <NavbarBrand>
          <Link to="/" className="flex items-center gap-2">
            <span className="text-xl font-bold">📚 Librarian</span>
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
                variant={'light'}
                color={isActive(item.to) ? 'primary' : 'default'}
                size="sm"
              >
                {item.label}
              </Button>
            </NavbarItem>
          ))}
        </NavbarContent>
      )}

      {/* Right side - theme toggle & auth status */}
      <NavbarContent justify="end">
        {/* Theme toggle */}
        <NavbarItem>
          <Tooltip content={isDark ? 'Switch to light mode' : 'Switch to dark mode'}>
            <Button
              isIconOnly
              variant="light"
              size="sm"
              onPress={toggleTheme}
              aria-label={isDark ? 'Switch to light mode' : 'Switch to dark mode'}
            >
              {isDark ? (
                <Sun className="w-5 h-5 text-default-500" />
              ) : (
                <Moon className="w-5 h-5 text-default-500" />
              )}
            </Button>
          </Tooltip>
        </NavbarItem>

        {!isConfigured ? (
          <NavbarItem>
            <Chip color="warning" variant="flat" size="sm">
              <IconAlertTriangle size={16} className="inline mr-1 text-amber-400" /> Auth not configured
            </Chip>
          </NavbarItem>
        ) : error ? (
          <NavbarItem>
            <Chip color="danger" variant="flat" size="sm">
              Auth error
            </Chip>
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
                <DropdownItem key="settings" onPress={() => navigate({ to: '/settings' })}>
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
              onPress={() => navigate({ to: '/', search: { signin: true } })}
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
                className={`w-full ${isActive(item.to)
                  ? 'text-primary font-semibold'
                  : 'text-foreground'
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
                signOut()
                setIsMenuOpen(false)
              }}
            >
              Sign Out
            </Button>
          </NavbarMenuItem>
        )}
      </NavbarMenu>
    </HeroNavbar>
  )
}
