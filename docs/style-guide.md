# Librarian Style Guide

This document defines the visual design patterns and UI conventions for the Librarian application. Follow these patterns to maintain consistency across the app.

---

## Design Philosophy

- **Dark-first**: The app uses a dark theme by default
- **Subtle depth**: Use gradients, backdrop blur, and shadows sparingly for visual hierarchy
- **Motion with purpose**: Animations enhance understanding, not distract
- **Information density**: Show relevant data without overwhelming users
- **Layout stability**: The UI should never shift after initial render. Key layout elements must remain stable - no content pushing other content around when loading states change or notifications appear

---

## Color System

### Semantic Colors
Use HeroUI's semantic colors consistently:

| Color | Usage |
|-------|-------|
| `primary` | Main actions, active states, links |
| `success` | Completed states, positive actions (seeding, done) |
| `warning` | Paused states, attention needed |
| `danger` | Errors, destructive actions |
| `default` | Neutral states, secondary content |
| `secondary` | Accent information, metadata |

### Text Colors
```
text-foreground     - Primary text (white in dark mode)
text-default-600    - Secondary text, labels
text-default-500    - Tertiary text, descriptions
text-default-400    - Muted text, hints, timestamps
text-default-300    - Very muted, disabled states
```

### Background Colors
```
bg-background       - Page background
bg-content1         - Primary card/surface background
bg-content2         - Secondary/nested surface background
bg-default-100      - Subtle highlight (table headers)
bg-black/50         - Overlay badges with backdrop-blur
```

---

## Layout Patterns

### Page Container
Standard page layout with responsive padding:
```tsx
<div className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
  {/* Page content */}
</div>
```

### Full-Height Scrollable Layout
For pages with sidebar + scrollable content (like Settings):
```tsx
<div className="h-[calc(100vh-4rem)] flex flex-col">
  <h1 className="text-2xl font-bold mb-6 flex-shrink-0">Title</h1>
  
  <div className="flex gap-6 flex-1 min-h-0">
    {/* Fixed sidebar */}
    <div className="w-64 flex-shrink-0">
      <Card className="sticky top-4">...</Card>
    </div>
    
    {/* Scrollable content with fade shadows */}
    <ScrollShadow className="flex-1 min-w-0">
      <div className="pb-4">{children}</div>
    </ScrollShadow>
  </div>
</div>
```

### Page Header with Actions
```tsx
<div className="flex justify-between items-center mb-6">
  <div>
    <h1 className="text-2xl font-bold">Page Title</h1>
    <p className="text-default-500">Optional description</p>
  </div>
  <div className="flex gap-2">
    <Button color="primary">Primary Action</Button>
  </div>
</div>
```

---

## Card Patterns

### Standard Content Card
```tsx
<Card className="bg-content1">
  <CardHeader className="flex gap-3">
    <div className="flex flex-col">
      <p className="text-lg font-semibold">Title</p>
      <p className="text-small text-default-500">Subtitle</p>
    </div>
  </CardHeader>
  <Divider />
  <CardBody className="gap-6">
    {/* Content with consistent gap-6 spacing */}
  </CardBody>
</Card>
```

### Media/Library Grid Cards (Poster Style)
Rich cards with background images, gradients, and hover actions:

```tsx
<Card
  isPressable
  onPress={handleClick}
  className="relative overflow-hidden aspect-[2/3] group border-none bg-content2"
>
  {/* Background with gradient overlay */}
  <div className="absolute inset-0 w-full h-full">
    {artwork ? (
      <>
        <img
          src={artwork}
          alt=""
          className="absolute inset-0 w-full h-full object-cover"
        />
        {/* Gradient for text readability */}
        <div className="absolute inset-0 bg-gradient-to-t from-black/90 via-black/20 to-black/40" />
      </>
    ) : (
      // Fallback gradient with type icon
      <div className="absolute inset-0 bg-gradient-to-br from-violet-900 via-purple-800 to-fuchsia-900">
        <div className="absolute inset-0 flex items-center justify-center opacity-30">
          <TypeIcon size={80} />
        </div>
      </div>
    )}
  </div>

  {/* Type badge - top left */}
  <div className="absolute top-2 left-2 z-10">
    <div className="px-2 py-1 rounded-md bg-black/50 backdrop-blur-sm text-xs font-medium text-white/90 flex items-center gap-1">
      <TypeIcon size={14} /> {typeLabel}
    </div>
  </div>

  {/* Bottom content */}
  <div className="absolute bottom-0 left-0 right-0 z-10 p-3">
    <h3 className="text-sm font-bold text-white mb-0.5 line-clamp-2 drop-shadow-lg">
      {title}
    </h3>
    <div className="flex items-center gap-1.5 text-xs text-white/70">
      <span>{count} Items</span>
      <span>•</span>
      <span>{size}</span>
    </div>
  </div>

  {/* Hover action menu - bottom right */}
  <div className="absolute bottom-2 right-2 z-20 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
    <Dropdown>...</Dropdown>
  </div>
</Card>
```

**Grid for poster cards:**
```tsx
<div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
  {items.map(item => <LibraryGridCard key={item.id} {...item} />)}
</div>
```

### Library Type Gradients
Use type-specific gradient backgrounds when no artwork is available:
```tsx
const LIBRARY_GRADIENTS = {
  MOVIES: 'from-violet-900 via-purple-800 to-fuchsia-900',
  TV: 'from-blue-900 via-indigo-800 to-cyan-900',
  MUSIC: 'from-emerald-900 via-green-800 to-teal-900',
  AUDIOBOOKS: 'from-amber-900 via-orange-800 to-yellow-900',
  OTHER: 'from-slate-800 via-gray-700 to-zinc-800',
}
```

### Empty State Card
```tsx
import { IconFolder } from '@tabler/icons-react'

<Card className="bg-content1/50 border-default-300 border-dashed border-2">
  <CardBody className="py-16 text-center">
    <div className="mx-auto w-20 h-20 rounded-full bg-default-100 flex items-center justify-center mb-6">
      <IconFolder size={48} className="text-amber-400" />
    </div>
    <h3 className="text-xl font-semibold mb-2">No items yet</h3>
    <p className="text-default-500 mb-6 max-w-md mx-auto">
      Helpful description of what to do.
    </p>
    <Button color="primary" size="lg">
      Add First Item
    </Button>
  </CardBody>
</Card>
```

---

## Data Table Pattern

Use the `DataTable` component for all tabular data. It provides:
- Search with clear button
- Filter chips with counts (using ButtonGroup for cohesive styling)
- Bulk actions on selection
- View mode toggle (table/cards)
- Sortable columns
- Row actions (inline + dropdown)

### Button Groups for Table Filtering
When filtering tables where options form logical groups (e.g., status filters like "All | Downloading | Seeding | Paused"), use `ButtonGroup` to visually connect the buttons:

```tsx
<ButtonGroup size="sm" variant="flat">
  <Button
    variant={value === null ? 'solid' : 'flat'}
    color={value === null ? 'primary' : 'default'}
    onPress={() => setValue(null)}
  >
    All
  </Button>
  <Button
    variant={value === 'active' ? 'solid' : 'flat'}
    color={value === 'active' ? 'success' : 'default'}
    onPress={() => setValue('active')}
  >
    Active
  </Button>
  <Button
    variant={value === 'paused' ? 'solid' : 'flat'}
    color={value === 'paused' ? 'warning' : 'default'}
    onPress={() => setValue('paused')}
  >
    Paused
  </Button>
</ButtonGroup>
```

**When to use ButtonGroup for filters:**
- Status/state filters (downloading, seeding, paused, error)
- Category filters where options are mutually exclusive
- View mode toggles (table/cards, list/grid)
- Any segmented control where only one option is active at a time

### Toolbar Structure
```
┌─────────────────────────────────────────────────────────────────┐
│ [🔍 Search input        ] [x]                 [Bulk] [📋|▦] [+] │
├─────────────────────────────────────────────────────────────────┤
│ 🔽 Filter: [All] [Downloading 3] [Seeding 5] [Paused 1] [Clear] │
└─────────────────────────────────────────────────────────────────┘
```

### Basic Usage
```tsx
<DataTable
  stateKey="unique-key"
  data={items}
  columns={columns}
  getRowKey={(item) => item.id}
  
  // Search
  searchFn={(item, term) => item.name.toLowerCase().includes(term.toLowerCase())}
  searchPlaceholder="Search items..."
  
  // Filters
  filters={[
    {
      key: 'status',
      label: 'Status',
      type: 'select',
      position: 'toolbar',
      options: [
        { key: 'active', label: 'Active', icon: '✓', color: 'success', count: 5 },
        { key: 'paused', label: 'Paused', icon: '⏸', color: 'warning', count: 2 },
      ],
      filterFn: (item, value) => !value || item.status === value,
    },
  ]}
  
  // View modes
  showViewModeToggle
  defaultViewMode="table"
  cardRenderer={({ item }) => <ItemCard item={item} />}
  cardGridClassName="grid grid-cols-1 lg:grid-cols-2 gap-4"
  
  // Selection & actions
  selectionMode="multiple"
  bulkActions={bulkActions}
  rowActions={rowActions}
  
  // Custom toolbar content
  toolbarContent={<Button color="primary">Add</Button>}
  toolbarContentPosition="end"
/>
```

### Column Definitions
```tsx
const columns: DataTableColumn<Item>[] = [
  {
    key: 'name',
    label: 'NAME',
    sortable: true,
    render: (item) => (
      <div className="flex flex-col gap-1 min-w-0">
        <span className="font-medium truncate">{item.name}</span>
        <span className="text-xs text-default-400 font-mono truncate">
          {item.subtitle}
        </span>
      </div>
    ),
    sortFn: (a, b) => a.name.localeCompare(b.name),
  },
  {
    key: 'status',
    label: 'STATUS',
    width: 120,
    sortable: true,
    render: (item) => (
      <Chip size="sm" color={statusColors[item.status]} variant="flat">
        {item.status}
      </Chip>
    ),
  },
  {
    key: 'size',
    label: 'SIZE',
    width: 100,
    sortable: true,
    render: (item) => (
      <span className="text-sm tabular-nums">{formatBytes(item.size)}</span>
    ),
  },
]
```

### Row Actions Pattern
```tsx
import { IconPlayerPause, IconTrash } from '@tabler/icons-react'

const rowActions: RowAction<Item>[] = [
  // Inline buttons (visible in row)
  {
    key: 'pause',
    label: 'Pause',
    icon: <IconPlayerPause size={16} className="text-amber-400" />,
    color: 'warning',
    inDropdown: false, // Shows as inline icon button
    isVisible: (item) => item.status === 'active',
    onAction: (item) => handlePause(item.id),
  },
  // Dropdown items
  {
    key: 'delete',
    label: 'Delete',
    icon: <IconTrash size={16} className="text-red-400" />,
    isDestructive: true,
    inDropdown: true, // Shows in ⋮ dropdown
    onAction: (item) => handleDelete(item.id),
  },
]
```

### Table Header Styling
Headers use consistent styling:
```tsx
classNames={{
  th: 'bg-default-100 text-default-600 first:rounded-l-lg last:rounded-r-lg',
}}
```

---

## Breadcrumbs

**Always use HeroUI's `Breadcrumbs` component** instead of custom breadcrumb implementations:

```tsx
import { Breadcrumbs, BreadcrumbItem } from '@heroui/react'

<Breadcrumbs className="mb-2">
  <BreadcrumbItem href="/libraries">Libraries</BreadcrumbItem>
  <BreadcrumbItem href="/libraries/123">My Library</BreadcrumbItem>
  <BreadcrumbItem isCurrent>Settings</BreadcrumbItem>
</Breadcrumbs>
```

**Key patterns:**
- Use `href` for navigation links
- Mark the current page with `isCurrent` (makes it non-clickable and styled differently)
- Place breadcrumbs at the top of the page header, typically with `className="mb-2"`

---

## Settings/Tab Layout

### Vertical Tab Navigation
```tsx
import type { TablerIcon } from '@tabler/icons-react'
import { IconFolder, IconSettings } from '@tabler/icons-react'

interface TabConfig {
  key: string
  path: string
  label: string
  Icon: TablerIcon
  iconColor: string
  description: string
}

const tabs: TabConfig[] = [
  { key: 'files', path: '/files', label: 'Files', Icon: IconFolder, iconColor: 'text-amber-400', description: 'Browse files' },
  { key: 'settings', path: '/settings', label: 'Settings', Icon: IconSettings, iconColor: 'text-default-400', description: 'Configuration' },
]

<div className="flex gap-6">
  {/* Sidebar */}
  <div className="w-64 shrink-0">
    <Card className="sticky top-4">
      <CardBody className="p-2">
        <nav className="flex flex-col gap-1">
          {tabs.map((tab) => (
            <Link
              key={tab.key}
              to={tab.path}
              className={`
                flex items-center gap-3 px-4 py-3 rounded-lg transition-all duration-200
                ${isActive(tab.path)
                  ? 'bg-primary text-primary-foreground shadow-md'
                  : 'hover:bg-content2 text-default-600 hover:text-foreground'
                }
              `}
            >
              <tab.Icon size={20} className={isActive(tab.path) ? '' : tab.iconColor} />
              <div className="flex flex-col">
                <span className="font-medium text-sm">{tab.label}</span>
                <span className={`text-xs ${isActive(tab.path) ? 'text-primary-foreground/70' : 'text-default-400'}`}>
                  {tab.description}
                </span>
              </div>
            </Link>
          ))}
        </nav>
      </CardBody>
    </Card>
  </div>

  {/* Content with scroll shadow */}
  <ScrollShadow className="flex-1 min-w-0">
    {children}
  </ScrollShadow>
</div>
```

---

## Loading States

### Full Page Loading
```tsx
<div className="flex justify-center items-center py-20">
  <Spinner size="lg" />
</div>
```

### Loading within Layout
Keep navigation/layout visible while content loads:
```tsx
<Layout>
  {isLoading ? (
    <div className="flex justify-center items-center py-20">
      <Spinner size="lg" />
    </div>
  ) : (
    content
  )}
</Layout>
```

### Skeleton Loading (Tables)
```tsx
<Skeleton className="w-full h-4 rounded" />
```

---

## Feedback Messages (Toasts)

### ⛔ NEVER Use Inline Error/Success Messages

**Do NOT display error or success messages inline in the page layout.** Inline messages cause layout shifts which disrupt the user's visual flow and create a jarring experience.

```tsx
// ❌ BAD - Inline error/success cards cause layout shift
{error && (
  <Card className="bg-danger-50 border-danger mb-6">
    <CardBody>
      <p className="text-danger">{error}</p>
    </CardBody>
  </Card>
)}

// ❌ BAD - Inline success message
{success && (
  <Card className="bg-success-50 border-success mb-6">
    <CardBody>
      <p className="text-success">{success}</p>
    </CardBody>
  </Card>
)}
```

### ✅ ALWAYS Use Toast Notifications

Use `addToast` from HeroUI for all success, error, and informational feedback:

```tsx
import { addToast } from '@heroui/react'

// Success feedback
addToast({
  title: 'Settings Saved',
  description: 'Your changes have been saved successfully.',
  color: 'success',
})

// Error feedback
addToast({
  title: 'Error',
  description: 'Failed to save settings. Please try again.',
  color: 'danger',
})

// Info/warning feedback
addToast({
  title: 'Feed Polled',
  description: 'Found 3 new episodes.',
  color: 'primary',
})
```

### Benefits of Toast Notifications
- **No layout shift**: Toasts overlay the UI without moving content
- **Consistent position**: Users know where to look for feedback
- **Auto-dismiss**: Toasts disappear automatically without user action
- **Non-blocking**: Users can continue interacting with the page

### When to Show Toasts
- After successful mutations (save, delete, create)
- When errors occur (API failures, validation errors)
- When background operations complete (polling, syncing)
- For authentication errors

### When NOT to Show Toasts
- For validation errors on form fields (use field-level error messages)
- For loading states (use spinners or skeletons)
- For empty states (use empty state components within the layout)

---

## Status Chips

Use consistent status chip patterns:

```tsx
// Status colors
const STATUS_COLORS = {
  DOWNLOADING: 'primary',
  SEEDING: 'success', 
  PAUSED: 'warning',
  CHECKING: 'secondary',
  QUEUED: 'default',
  ERROR: 'danger',
}

<Chip size="sm" color={STATUS_COLORS[status]} variant="flat">
  {statusLabel}
</Chip>
```

---

## Progress Indicators

### Progress Bar with State Colors
```tsx
<Progress
  value={progress * 100}
  color={
    state === 'SEEDING' ? 'success' :
    state === 'ERROR' ? 'danger' :
    state === 'PAUSED' ? 'warning' :
    'primary'
  }
  size="sm"
  aria-label="Progress"
/>
```

### Progress with Label
```tsx
<div className="flex flex-col gap-1">
  <Progress value={progress * 100} color="primary" size="sm" />
  <span className="text-xs text-default-500 tabular-nums">
    {(progress * 100).toFixed(1)}%
  </span>
</div>
```

---

## Typography

### Numeric Data
Use `tabular-nums` for numbers that change or align:
```tsx
<span className="text-sm tabular-nums">{bytes}</span>
<span className="text-xs tabular-nums">{percentage}%</span>
```

### Monospace Data
Use for hashes, codes, IDs:
```tsx
<span className="text-xs text-default-400 font-mono truncate">
  {hash.slice(0, 16)}...
</span>
```

### Truncation
```tsx
// Single line
<span className="truncate">{text}</span>

// Multi-line (2 lines max)
<span className="line-clamp-2">{text}</span>
```

---

## Icons

### Use Tabler Icons
**Always use `@tabler/icons-react`** for all icons. Never use emojis or custom SVG components.

```tsx
import { IconFolder, IconMovie, IconTrash } from '@tabler/icons-react'

// Basic usage with size prop
<IconFolder size={20} />

// With semantic color
<IconFolder size={20} className="text-amber-400" />

// In buttons
<Button startContent={<IconTrash size={16} className="text-red-400" />}>
  Delete
</Button>
```

### Icon Color Conventions
Apply consistent colors based on icon meaning:

| Category | Color | Example Icons |
|----------|-------|---------------|
| Folders | `text-amber-400` | `IconFolder`, `IconFolderOpen` |
| Movies | `text-purple-400` | `IconMovie` |
| TV Shows | `text-blue-400` | `IconDeviceTv` |
| Music | `text-green-400` | `IconMusic` |
| Audiobooks | `text-orange-400` | `IconHeadphones` |
| Success | `text-green-400` | `IconCheck`, `IconCircleCheck` |
| Warning | `text-amber-400` | `IconAlertTriangle` |
| Error/Delete | `text-red-400` | `IconTrash`, `IconX` |
| Download | `text-blue-400` | `IconArrowDown`, `IconDownload` |
| Upload | `text-green-400` | `IconArrowUp`, `IconUpload` |
| RSS | `text-orange-400` | `IconRss` |
| Neutral | `text-default-400` | `IconSettings`, `IconFile`, `IconClipboard` |

### Common Icon Sizes
```tsx
size={12}  // Inline with small text (speed indicators)
size={16}  // Buttons, menu items, table cells
size={20}  // Tab icons, list items  
size={24}  // Section headers
size={32}  // Card headers
size={48}  // Empty states
size={64}  // Hero placeholders
```

### Tab/Menu Icon Pattern
When defining tabs or menus with icons:

```tsx
import type { TablerIcon } from '@tabler/icons-react'
import { IconSettings, IconFolder } from '@tabler/icons-react'

interface TabConfig {
  key: string
  label: string
  Icon: TablerIcon
  iconColor: string
}

const tabs: TabConfig[] = [
  { key: 'files', label: 'Files', Icon: IconFolder, iconColor: 'text-amber-400' },
  { key: 'settings', label: 'Settings', Icon: IconSettings, iconColor: 'text-default-400' },
]

// Render - active tabs use default color, inactive show iconColor
<tab.Icon size={20} className={isActive ? '' : tab.iconColor} />
```

---

## Overlays & Badges

### Floating Badge on Cards
```tsx
import { IconMovie } from '@tabler/icons-react'

<div className="px-2 py-1 rounded-md bg-black/50 backdrop-blur-sm text-xs font-medium text-white/90 flex items-center gap-1">
  <IconMovie size={14} /> {label}
</div>
```

### Hover-Reveal Actions
```tsx
<div className="opacity-0 group-hover:opacity-100 transition-opacity duration-200">
  <Button size="sm" variant="flat" className="bg-black/50 backdrop-blur-sm text-white">
    Action
  </Button>
</div>
```

---

## Responsive Patterns

### Grid Breakpoints
```tsx
// Poster cards (portrait)
className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4"

// Content cards (landscape)
className="grid grid-cols-1 lg:grid-cols-2 gap-4"

// Dashboard widgets
className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4"
```

### Stack on Mobile
```tsx
className="flex flex-col sm:flex-row gap-4"
```

---

## Animation Patterns

### Transitions
```tsx
// Opacity
className="transition-opacity duration-200"

// All properties
className="transition-all duration-200"

// Longer for artwork changes
className="transition-opacity duration-800"
```

### Hover States
```tsx
// Card hover
className="hover:bg-content2 transition-colors"

// Pressable feedback
<Card isPressable className="hover:bg-content2">
```

---

## Best Practices

1. **Consistency**: Use the same patterns throughout the app
2. **Hierarchy**: Primary actions stand out, secondary are subtle
3. **Feedback**: Use toasts for success/error messages, never inline elements that shift layout
4. **Accessibility**: Include aria-labels, proper contrast
5. **Performance**: Use `tabular-nums` for changing numbers, `truncate` for long text
6. **Mobile-first**: Design for mobile, enhance for desktop
7. **Layout stability**: The UI must not shift after initial render. Use skeleton loading instead of spinners that change layout, use toasts instead of inline error messages
8. **Icons**: Always use `@tabler/icons-react` with semantic colors - never emojis or custom SVGs

---

## Layout Stability Rules

The UI should **never shift** once the page layout has rendered. This is a core principle.

### ❌ Things That Cause Layout Shift (AVOID)
- Inline error/success cards that push content down
- Full-screen spinners that replace content (use skeleton rows instead)
- Conditional elements that appear/disappear in the middle of content
- Images without explicit dimensions

### ✅ Patterns That Maintain Stability
- **Toast notifications**: Overlay the UI without moving anything
- **Skeleton loading**: Placeholder rows in tables/grids maintain the same height as real data
- **Fixed headers/footers**: Navigation and controls stay in place
- **Modal dialogs**: Overlay content without shifting the background
- **Empty states**: Render in the same space as data would occupy

### Loading State Hierarchy
1. **Skeleton rows** for DataTable and lists (preferred - maintains layout)
2. **Inline spinners** only within fixed-height containers
3. **Full-page spinners** only for initial app/route loading before layout is established

### New Data Behavior
- Adding new items to a list/grid is acceptable (content grows)
- But existing content should not jump around
- Use animations (`transition-all`) when adding items to make changes feel intentional
