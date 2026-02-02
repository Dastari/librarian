# Frontend Refactor Plan

This document outlines the sweeping frontend changes needed to align with backend refactoring and modernize the codebase.

## Overview

The frontend needs significant updates to:
1. Remove deprecated library-level settings that have moved to content entities
2. Adopt Apollo Client hooks consistently
3. Implement proper form handling with react-hook-form + zod
4. Tightly couple to generated GraphQL types (no custom extensions)
5. Use modern React patterns for loading states

---

## 1. Schema Changes: AutoHunt & AutoDownload Migration

### What Changed
`AutoHunt` and `AutoDownload` have been **removed from Library settings** and now exist directly on content entities:
- Shows
- Movies  
- Albums
- Audiobooks

### Frontend Impact

#### Files to Update
- `src/components/library/LibrarySettingsForm.tsx` - Remove AutoHunt/AutoDownload fields
- `src/components/shared/SettingBadges.tsx` - Update `AutoDownloadBadge` usage context
- `src/routes/libraries/$libraryId.tsx` - Remove AutoDownload badge from library header
- `src/routes/libraries/index.tsx` - Remove AutoDownload from library cards if present

#### Files to Verify Content-Level Settings
- `src/components/shows/ShowSettingsModal.tsx` - Should have AutoDownload/AutoHunt
- ✅ `src/routes/shows/$showId.tsx` - **OPTIMIZED:** Using consolidated data fetching, generated types, removed ShimmerLoader
- `src/routes/movies/$movieId.tsx` - Add/verify AutoDownload/AutoHunt
- `src/routes/albums/$albumId.tsx` - Add/verify AutoDownload/AutoHunt  
- `src/routes/audiobooks/$audiobookId.tsx` - Add/verify AutoDownload/AutoHunt

---

## 2. Apollo Client Hooks Migration

### Current State
The codebase uses a mix of:
- Custom `graphqlClient` wrapper with `.query()/.mutation()/.subscription().toPromise()`
- Custom `useGraphQLQuery` hook in `src/hooks/useGraphQL.ts`
- Manual state management (`useState` for data, loading, error)

### Target State
Use Apollo Client hooks consistently:
- `useQuery` for queries
- `useMutation` for mutations  
- `useSubscription` for subscriptions

### Benefits
- Built-in caching with `cache-and-network` policy
- `previousData` to prevent flash of loading states
- Automatic refetching and cache updates
- TypeScript integration with generated types

### Implementation Pattern

```tsx
import { useQuery, useMutation, gql } from '@apollo/client'
import type { LibraryQuery, LibraryQueryVariables } from '../lib/graphql/generated/graphql'

// Use generated TypedDocumentNode when available, or define inline
const LIBRARY_QUERY = gql`
  query Library($Id: String!) {
    Library(Id: $Id) {
      Id
      Name
      Path
      LibraryType
      Scanning
    }
  }
`

function LibraryPage({ libraryId }: { libraryId: string }) {
  const { data, previousData, loading, error, refetch } = useQuery<LibraryQuery, LibraryQueryVariables>(
    LIBRARY_QUERY,
    { 
      variables: { Id: libraryId },
      fetchPolicy: 'cache-and-network',
    }
  )

  // Use previousData to prevent flash during refetch
  const library = data?.Library ?? previousData?.Library

  // Loading only on initial load, not refetches
  if (loading && !library) {
    return <Spinner />
  }

  return <div>{library?.Name}</div>
}
```

### Mutations with Loading State

```tsx
import { useMutation } from '@apollo/client'

function UpdateButton({ libraryId }: { libraryId: string }) {
  const [updateLibrary, { loading: isUpdating }] = useMutation(UPDATE_LIBRARY_MUTATION)

  const handleUpdate = async () => {
    try {
      await updateLibrary({
        variables: { Id: libraryId, Input: { Name: 'New Name' } },
        // Optionally refetch queries after mutation
        refetchQueries: ['Library'],
      })
      addToast({ title: 'Updated', color: 'success' })
    } catch (error) {
      addToast({ title: 'Error', description: sanitizeError(error), color: 'danger' })
    }
  }

  return (
    <Button onPress={handleUpdate} isLoading={isUpdating}>
      Update
    </Button>
  )
}
```

### Files to Migrate
Priority order (most impactful first):

1. **Route Pages** (heavy data fetching)
   - ✅ `src/routes/shows/$showId.tsx` - **COMPLETED:** Consolidated queries, removed ShimmerLoader, using generated types
   - `src/routes/libraries/$libraryId.tsx` - Already cleaned up (ShimmerLoader removed)
   - `src/routes/movies/$movieId.tsx` - Needs cleanup
   - `src/routes/albums/$albumId.tsx` - Needs cleanup
   - `src/routes/audiobooks/$audiobookId.tsx` - Needs cleanup
   - `src/routes/hunt.tsx`
   - `src/routes/settings/*.tsx`

2. **Library Tab Components**
   - `src/components/library/LibraryShowsTab.tsx`
   - `src/components/library/LibraryMoviesTab.tsx`
   - `src/components/library/LibraryAlbumsTab.tsx`
   - `src/components/library/LibraryAudiobooksTab.tsx`
   - `src/components/library/LibraryArtistsTab.tsx`
   - `src/components/library/LibraryTracksTab.tsx`
   - `src/components/library/LibraryAuthorsTab.tsx`

3. **Data Tables & Lists**
   - `src/components/downloads/TorrentTable.tsx`

---

## 3. Form Handling with react-hook-form + zod + HeroUI

### Current State
Forms use manual `useState` for each field, manual validation, and direct mutation calls.

### Target State
Use react-hook-form with zod schemas derived from GraphQL input types.

### Dependencies to Add
```bash
cd frontend
pnpm add react-hook-form zod @hookform/resolvers
```

### Implementation Pattern

```tsx
import { useForm, Controller } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { z } from 'zod'
import { Input, Switch, Button } from '@heroui/react'
import { useMutation } from '@apollo/client'

// Define schema matching GraphQL input type
const updateLibrarySchema = z.object({
  Name: z.string().min(1, 'Name is required'),
  Path: z.string().min(1, 'Path is required'),
  AutoScan: z.boolean(),
  ScanIntervalMinutes: z.number().min(1).max(1440),
  WatchForChanges: z.boolean(),
})

type UpdateLibraryForm = z.infer<typeof updateLibrarySchema>

function LibrarySettingsForm({ library }: { library: Library }) {
  const [updateLibrary, { loading }] = useMutation(UPDATE_LIBRARY_MUTATION)

  const {
    control,
    handleSubmit,
    formState: { errors, isDirty },
  } = useForm<UpdateLibraryForm>({
    resolver: zodResolver(updateLibrarySchema),
    defaultValues: {
      Name: library.Name,
      Path: library.Path,
      AutoScan: library.AutoScan,
      ScanIntervalMinutes: library.ScanIntervalMinutes,
      WatchForChanges: library.WatchForChanges,
    },
  })

  const onSubmit = async (data: UpdateLibraryForm) => {
    try {
      await updateLibrary({
        variables: { Id: library.Id, Input: data },
      })
      addToast({ title: 'Settings saved', color: 'success' })
    } catch (error) {
      addToast({ title: 'Error', description: sanitizeError(error), color: 'danger' })
    }
  }

  return (
    <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
      <Controller
        name="Name"
        control={control}
        render={({ field }) => (
          <Input
            {...field}
            label="Library Name"
            isInvalid={!!errors.Name}
            errorMessage={errors.Name?.message}
          />
        )}
      />

      <Controller
        name="AutoScan"
        control={control}
        render={({ field }) => (
          <Switch
            isSelected={field.value}
            onValueChange={field.onChange}
          >
            Auto Scan
          </Switch>
        )}
      />

      <Button
        type="submit"
        color="primary"
        isLoading={loading}
        isDisabled={!isDirty}
      >
        Save Changes
      </Button>
    </form>
  )
}
```

### Forms to Migrate
- `src/components/library/LibrarySettingsForm.tsx`
- `src/components/library/AddLibraryModal.tsx`
- `src/components/shows/ShowSettingsModal.tsx`
- `src/components/settings/QualitySettingsCard.tsx`
- All add/edit modals for content types

---

## 4. Generated GraphQL Types: Strict Usage

### Principle
**Never extend or create custom types that don't exist in the backend.**

If a field doesn't exist in generated types:
1. It probably doesn't exist in the schema
2. The schema needs updating, OR
3. The frontend code referencing it should be removed

### Type Sources
- **Primary**: `src/lib/graphql/generated/graphql.ts` - All types from codegen
- **Derived**: `src/lib/graphql/codegen-nodes.ts` - Query result node types

### Pattern: Query-Specific Types

Use TypedDocumentNode from codegen for fully typed queries:

```tsx
import { useQuery } from '@apollo/client'
import { LibraryDocument, type LibraryQuery } from '../lib/graphql/generated/graphql'

function LibraryPage({ id }: { id: string }) {
  // Fully typed - variables and return type inferred
  const { data } = useQuery(LibraryDocument, {
    variables: { Id: id },
  })

  // data.Library is properly typed
  return <div>{data?.Library?.Name}</div>
}
```

### Cleanup Required
1. Remove custom type extensions in route files
2. Remove types from `src/lib/graphql/types.ts` that duplicate generated types
3. Remove fields from queries that don't exist in schema
4. Update components to only use fields that exist

---

## 5. Loading States with useTransition

### When to Use
Use `useTransition` for non-urgent state updates that should show a pending state:
- Navigation during data loading
- Filter/search updates
- Sorting changes

### Pattern

```tsx
import { useTransition } from 'react'

function FilterableList() {
  const [isPending, startTransition] = useTransition()
  const [filter, setFilter] = useState('')

  const handleFilterChange = (value: string) => {
    startTransition(() => {
      setFilter(value)
    })
  }

  return (
    <div>
      <Input
        value={filter}
        onChange={(e) => handleFilterChange(e.target.value)}
        className={isPending ? 'opacity-50' : ''}
      />
      {isPending && <Spinner size="sm" />}
      <FilteredResults filter={filter} />
    </div>
  )
}
```

### When NOT to Use
- Apollo `useMutation` already provides `loading` state
- Apollo `useQuery` already provides `loading` state
- Simple button clicks with immediate feedback

---

## 6. Cleanup Tasks

### ✅ COMPLETED: Remove Custom GraphQL Wrapper
The `graphqlClient` wrapper in `src/lib/graphql/client.ts` can be simplified once Apollo hooks are adopted:
- Keep `apolloClient` export for provider
- Remove or deprecate `graphqlClient` wrapper object
- Keep subscription helper if needed for non-hook contexts

### Remove Custom Hooks
Once Apollo hooks are used consistently:
- Deprecate `src/hooks/useGraphQL.ts` (useGraphQLQuery, useMutation, useLazyQuery)
- These are reimplementing what Apollo provides

### ✅ COMPLETED: Remove Template Data & ShimmerLoader
The ShimmerLoader pattern with template data has been removed:
- ✅ `src/lib/template-data.ts` - **DELETED**
- ✅ `src/components/shared/ShimmerLoader.tsx` - **DELETED**
- ✅ All route pages updated to use simple `<Spinner />` loading states
- ✅ Apollo's `previousData` prevents flash better than shimmer approach

Files cleaned up:
- `routes/shows/$showId.tsx` - Removed ShimmerLoader, using direct data access
- `routes/movies/$movieId.tsx` - Removed ShimmerLoader, using direct data access
- `routes/albums/$albumId.tsx` - Imports removed
- `routes/audiobooks/$audiobookId.tsx` - Imports removed
- `routes/libraries/$libraryId/collections.tsx` - Removed ShimmerLoader
- `routes/libraries/$libraryId/settings.tsx` - Removed ShimmerLoader
- `routes/settings/rss.tsx` - Removed ShimmerLoader
- `components/shared/LoadingState.tsx` - Removed deprecation notices

### Simplify Type Exports
`src/lib/graphql/index.ts` has many manual type exports that could be simplified:
- Export generated types directly
- Remove duplicate type definitions from `types.ts`

---

## 7. Migration Order

### Phase 1: Schema Alignment
1. Regenerate GraphQL types from updated backend schema
2. Remove `AutoDownload`/`AutoHunt` from library components
3. Verify these exist on content entities (Show, Movie, Album, Audiobook)

### Phase 2: Apollo Hooks
1. Add `@apollo/client` hooks to existing imports
2. Migrate route pages one at a time
3. Use `previousData` pattern to prevent flashes
4. Remove old `useState` + `useEffect` + `graphqlClient` patterns

### Phase 3: Forms
1. Add react-hook-form + zod dependencies
2. Create shared zod schemas matching GraphQL inputs
3. Migrate forms starting with LibrarySettingsForm
4. Use Controller with HeroUI components

### Phase 4: Cleanup ✅ PARTIALLY COMPLETE
1. ✅ Remove deprecated hooks and utilities (pending)
2. ✅ **DONE:** Remove ShimmerLoader and template data
3. Simplify type exports (pending)
4. Update documentation (ongoing)

---

## 8. Code Style Summary

```tsx
// ✅ Good - Apollo hook with previousData
const { data, previousData, loading } = useQuery(QUERY)
const item = data?.Item ?? previousData?.Item

// ❌ Bad - Manual state management
const [data, setData] = useState(null)
const [loading, setLoading] = useState(true)
useEffect(() => { fetchData().then(setData) }, [])

// ✅ Good - useMutation with loading
const [mutate, { loading }] = useMutation(MUTATION)
<Button isLoading={loading} onPress={() => mutate(...)}>Save</Button>

// ❌ Bad - Manual action loading state
const [isLoading, setIsLoading] = useState(false)
const handleClick = async () => {
  setIsLoading(true)
  await graphqlClient.mutation(...).toPromise()
  setIsLoading(false)
}

// ✅ Good - react-hook-form with zod
const { control, handleSubmit } = useForm({ resolver: zodResolver(schema) })

// ❌ Bad - Manual form state
const [name, setName] = useState('')
const [error, setError] = useState('')

// ✅ Good - Generated types only
import type { Library } from '../lib/graphql/generated/graphql'

// ❌ Bad - Extended/custom types
type Library = GeneratedLibrary & { AutoDownload?: boolean }
```

---

## 9. Files Reference

### Files Likely Needing Changes
| File | Changes Needed |
|------|----------------|
| `routes/libraries/$libraryId.tsx` | Apollo hooks, remove AutoDownload |
| `routes/shows/$showId.tsx` | Apollo hooks |
| `routes/movies/$movieId.tsx` | Apollo hooks |
| `components/library/LibrarySettingsForm.tsx` | react-hook-form, remove AutoDownload/AutoHunt |
| `components/library/AddLibraryModal.tsx` | react-hook-form |
| `components/shared/SettingBadges.tsx` | Verify usage context |
| `lib/graphql/client.ts` | Simplify after migration |
| `hooks/useGraphQL.ts` | Deprecate |
| `lib/template-data.ts` | Delete |
| `components/shared/ShimmerLoader.tsx` | Delete |

### Files to Regenerate
- `lib/graphql/generated/graphql.ts` - After backend schema changes
- `lib/graphql/generated/schema.json` - After backend schema changes
- `lib/graphql/generated/types.ts` - After backend schema changes
