---
name: Full Code Review
overview: Comprehensive code review covering dead code, replicated patterns, database schema issues, UI consistency problems, and opportunities for reusable components.
todos:
  - id: quality-display-fix
    content: Update /shows/$showId.tsx settings bar to show inline quality settings instead of old qualityProfileId system
    status: completed
  - id: replace-custom-svg
    content: Replace custom MoreIcon SVG in TvShowCard.tsx with IconDotsVertical from @tabler/icons-react
    status: completed
  - id: status-chip-component
    content: Create reusable StatusChip component for monitored/active/disabled/error states
    status: completed
  - id: episode-status-chip
    content: Create reusable EpisodeStatusChip component with getStatusColor/getStatusLabel logic
    status: completed
  - id: quality-badge-component
    content: Create reusable QualityBadge component for resolution/codec/HDR display
    status: completed
  - id: export-default-quality
    content: Export DEFAULT_QUALITY_SETTINGS from QualitySettingsCard.tsx to eliminate duplication
    status: completed
  - id: database-schema-audit
    content: "Document or remove unused tables: subscriptions, media_items, events, artwork"
    status: completed
  - id: backend-dead-code-audit
    content: "Review all 49 #[allow(dead_code)] annotations and remove truly dead code"
    status: completed
---

# Full Backend and Frontend Code Review

## 1. Database Schema Issues

### Potentially Unused/Legacy Tables

| Table | Status | Recommendation |

|-------|--------|----------------|

| `subscriptions` | Created in migration 001, but replaced by `tv_shows` monitoring | Consider removing or deprecating |

| `media_items` | Only referenced in schema.rs, no active repository | Legacy - review for removal |

| `events` | Defined but no repository in `db/` | No usage found - consider removing |

| `artwork` | Defined but no repository, shows use `poster_url` directly | Review if needed |

| `jobs` | Defined with scheduled_at, but no `db/jobs.rs` repository | Partially used via migrations only |

### Schema Overlap

- **quality_profiles table vs inline quality settings**: Both exist. Libraries and TV shows now have inline `allowed_resolutions`, `allowed_video_codecs`, etc. but `quality_profiles` table and `qualityProfileId` references still exist. This creates confusion about which system to use.

**Recommendation:** Either deprecate `quality_profiles` in favor of inline settings, or document clearly when each is used.

---

## 2. Backend Dead Code

### Files with `#[allow(dead_code)]` (49 instances)

**High Priority - Core Types ([`backend/src/graphql/types.rs`](backend/src/graphql/types.rs)):**

- Line 459: `CreateLibraryInput` - replaced by `CreateLibraryFullInput`
- Line 495: `UpdateLibraryInput` - replaced by `UpdateLibraryFullInput`  
- Line 810: `LibraryScanProgress` - defined for "future subscription use" but unused
- Line 1091: `WantedEpisode` - defined for "future wanted list feature" but unused

**Services with dead code:**

- `metadata.rs` - 2 instances
- `tvmaze.rs` - 7 instances (unused response structs)
- `cache.rs` - 3 instances
- `organizer.rs` - 2 instances
- `prowlarr.rs` - 3 instances
- `transcoder.rs` - 6 instances
- `filename_parser.rs` - 3 instances

**Recommendation:** Review each `#[allow(dead_code)]` annotation. Keep those marked for future implementation, remove truly dead code.

---

## 3. Frontend UI Inconsistencies

### Show Detail Page Settings Bar (CRITICAL)

In [`frontend/src/routes/shows/$showId.tsx`](frontend/src/routes/shows/$showId.tsx) lines 692-776:

The settings summary card shows:

- "Inheriting from library" chip for auto-download
- Organization/Rename/Monitor/Quality settings display

**Issue:** The Quality display still uses the old `qualityProfileId` based system:

```tsx
const showProfile = show.qualityProfileId
  ? qualityProfiles.find(p => p.id === show.qualityProfileId)
  : null
```

This doesn't reflect the new inline quality settings (`allowedResolutionsOverride`, `allowedVideoCodecsOverride`, etc.).

**Recommendation:** Update to display the effective inline quality settings (resolutions, codecs, HDR) instead of just the profile name.

### Custom SVG Icon Violation

In [`frontend/src/components/library/TvShowCard.tsx`](frontend/src/components/library/TvShowCard.tsx) lines 15-31:

```tsx
const MoreIcon = () => (
  <svg ...>
    <circle cx="12" cy="12" r="1" />
    ...
  </svg>
)
```

**Issue:** Uses custom SVG instead of `IconDotsVertical` from `@tabler/icons-react`.

### Chip Usage Inconsistencies

59 `<Chip>` usages across 21 files with inconsistent patterns:

| Pattern | Files | Issue |

|---------|-------|-------|

| Monitored status | `$showId.tsx`, `LibraryShowsTab.tsx`, `subscriptions/index.tsx` | Different colors and labels |

| Episode status | `$showId.tsx` | Uses `getStatusColor()` helper - good pattern |

| RSS feed status | `settings/rss.tsx` | Inline color decisions |

| Indexer health | `settings/indexers.tsx` | Different pattern than RSS |

| Quality badges | Multiple files | No consistent component |

**Example inconsistency:**

- `TvShowCard.tsx` uses inline styled div with `bg-success/80` instead of `<Chip>`
- `$showId.tsx` uses `<Chip color="primary">Monitored</Chip>`

---

## 4. Replicated Code Opportunities

### Quality Settings Default

Duplicated in:

- [`LibrarySettingsTab.tsx`](frontend/src/components/library/LibrarySettingsTab.tsx) lines 35-44
- [`ShowSettingsModal.tsx`](frontend/src/components/shows/ShowSettingsModal.tsx) lines 33-42
```tsx
const DEFAULT_QUALITY_SETTINGS: QualitySettings = {
  allowedResolutions: [],
  allowedVideoCodecs: [],
  ...
}
```


**Recommendation:** Export from `QualitySettingsCard.tsx` and import where needed.

### Status Color/Label Helpers

In [`$showId.tsx`](frontend/src/routes/shows/$showId.tsx) lines 59-95:

```tsx
function getStatusColor(status: EpisodeStatus): 'success' | 'warning' | ...
function getStatusLabel(status: EpisodeStatus): string
```

These should be shared utilities used by all episode status displays.

---

## 5. Recommended Reusable Components

### Create: `components/shared/StatusChip.tsx`

```tsx
interface StatusChipProps {
  status: 'monitored' | 'unmonitored' | 'active' | 'disabled' | 'error';
  size?: 'sm' | 'md';
}
```

**Use cases:** Monitored badges, RSS feed status, indexer health, library scanning status.

### Create: `components/shared/EpisodeStatusChip.tsx`

```tsx
interface EpisodeStatusChipProps {
  status: EpisodeStatus;
  size?: 'sm' | 'md';
}
```

Centralizes the `getStatusColor`/`getStatusLabel` logic.

### Create: `components/shared/QualityBadge.tsx`

```tsx
interface QualityBadgeProps {
  resolution?: string;
  codec?: string;
  hdr?: boolean;
  hdrType?: string;
}
```

**Use cases:** Unmatched files tab, RSS test modal, metadata parsing display.

---

## 6. GraphQL Naming Convention Review

**Good:**

- Field names use camelCase consistently
- Enums use SCREAMING_SNAKE_CASE (`DOWNLOADING`, `SEEDING`, etc.)
- Input types suffixed with `Input`
- Result types suffixed with `Result`

**Minor issues:**

- `TvShow` vs `tvShow` - type uses PascalCase, fields use camelCase (correct)
- Query naming is consistent: `tvShows`, `tvShow`, `addTvShow`

**No major issues found.**

---

## 7. Files to Clean Up

### Backend

- Remove legacy `CreateLibraryInput`/`UpdateLibraryInput` if truly unused
- Review `media/transcoder.rs` - 6 dead code items, may be placeholder
- Review `services/prowlarr.rs` - appears to be legacy, replaced by indexer module

### Frontend  

- Delete unused icon imports if any
- Consolidate duplicate `formatBytes` calls (backend provides `sizeFormatted`)

---

## Summary of Action Items

1. **Update Show Detail Page** - Fix quality display to use inline settings
2. **Replace custom MoreIcon** - Use `IconDotsVertical` from Tabler
3. **Create StatusChip component** - Consolidate badge patterns
4. **Create EpisodeStatusChip component** - Consolidate episode status display
5. **Export DEFAULT_QUALITY_SETTINGS** - Remove duplication
6. **Database schema cleanup** - Document or remove unused tables
7. **Backend dead code audit** - Review all `#[allow(dead_code)]` annotations