# RBAC Implementation Plan

## Current State

### Backend Authentication
- JWT-based local auth with access/refresh tokens.
- `AuthUser { user_id, email, role }` extracted from JWT claims into GraphQL context.
- `AuthGuard` (requires authentication) and `RoleGuard` (requires specific role string) already exist in `backend/src/services/graphql/auth.rs`.
- Roles are stored as a freeform `String` on the `users` table (`role` column). The auth service mints tokens with `role: "admin" | "member"`.
- A `user_library_access` table exists with `(user_id, library_id, access_level)` and helpers `has_library_access`, `grant_library_access`, `revoke_library_access`.

### Frontend Authentication
- `frontend/src/lib/auth.ts` — token storage (cookies), `AuthUser` and `AuthSession` types, helpers for get/set/clear tokens.
- `frontend/src/hooks/useAuth.ts` — `useAuth()` hook providing `user`, `session`, `signIn`, `signUp`, `signOut`, `isAuthenticated`. The `user` object includes `role: string` from the JWT.
- `frontend/src/lib/auth-context.ts` — `AuthContext` interface used by TanStack Router for route-level auth state.
- `frontend/src/lib/graphql/client.ts` — Apollo Client with `errorLink` that currently silences `UNAUTHORIZED` / `Authentication required` errors but has no handling for `FORBIDDEN`.
- `frontend/src/lib/graphql/documents/*.graphql` — 22 operation documents consumed by codegen.
- `frontend/codegen.ts` — GraphQL codegen config producing:
  - `generated/graphql.ts` — `TypedDocumentNode` + operation types (primary).
  - `generated/types.ts` — standalone TypeScript types from schema + operations.
  - `generated/schema.json` — introspection snapshot.
- No role-based UI gating exists today — all nav items, settings pages, and actions are visible to every authenticated user.

### Authorization Gaps
- **Every generated resolver** (List, GetById, Create, Update, Delete, DeleteMany) only calls `ctx.auth_user()?` — any authenticated user can perform any operation on any entity.
- Sensitive entities (`users`, `app_settings`, `refresh_tokens`, `invite_tokens`) have full CRUD exposed.
- No ownership filtering — users can read/modify other users' data.
- No per-field read/write restrictions — e.g., `password_hash` is hidden via `#[graphql(skip)]` but there's no macro-level system for field-level authorization.
- Library-scoped entities (movies, shows, albums, etc.) have a `user_id` column but it is never checked in generated resolvers.
- Frontend shows all UI to all users regardless of role.

---

## Design Goals

1. **Entity-level RBAC** via macro attributes — control which roles can List/Read/Create/Update/Delete each entity.
2. **Ownership scoping** via macro attributes — automatically filter queries by `user_id` and reject mutations on entities the caller doesn't own.
3. **Library-scoped access** — for entities under a library, verify `user_library_access` membership.
3a. **Relation-aware scoping** — support entities that do not have direct `user_id`/`library_id` columns (e.g., episodes via shows, chapters via audiobooks, tracks via albums).
4. **Per-field read/write guards** via macro attributes — control which roles can read or write specific fields.
5. **Backward compatible** — entities without new attributes behave exactly as today (authenticated-only, no role check).
6. **Zero runtime cost when unused** — the macro emits guard code only when attributes are present.
7. **Consistent error contract** — backend returns structured `FORBIDDEN` error codes; frontend handles them uniformly via Apollo error link and role-aware UI.
8. **Codegen-driven frontend types** — all schema changes (removed operations, nullable field changes) propagate through `graphql-codegen`; frontend never defines custom types that duplicate or override generated ones.

---

## Phase 1: Role Definitions & Core Policy Types

### 1a. Formalize roles as an enum (backend)

In `backend/src/services/graphql/auth.rs`, add:

```rust
/// Application roles, ordered by privilege level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Guest,   // Read-only access to permitted libraries
    Member,  // Standard user — owns libraries, can download
    Admin,   // Full access to all entities and settings
}

impl Role {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "admin" => Role::Admin,
            "guest" => Role::Guest,
            _ => Role::Member,
        }
    }
}
```

Add a helper on `AuthUser`:

```rust
impl AuthUser {
    pub fn role_enum(&self) -> Role {
        Role::from_str(self.role.as_deref().unwrap_or("member"))
    }

    pub fn is_admin(&self) -> bool {
        self.role_enum() == Role::Admin
    }
}
```

### 1b. Mirror roles in the frontend

In `frontend/src/lib/auth.ts`, add role constants and helpers that match the backend enum. These are the **single source of truth** for role checks on the frontend — never hard-code role strings in components.

```typescript
// ============================================================================
// Role Definitions (must match backend Role enum in auth.rs)
// ============================================================================

export const ROLES = {
  GUEST: "guest",
  MEMBER: "member",
  ADMIN: "admin",
} as const;

export type RoleName = (typeof ROLES)[keyof typeof ROLES];

/** Privilege ordering — higher index = more privilege. */
const ROLE_LEVEL: Record<string, number> = {
  guest: 0,
  member: 1,
  admin: 2,
};

/** Check if a user's role meets the minimum required role. */
export function hasRole(user: AuthUser | null | undefined, minimum: RoleName): boolean {
  if (!user) return false;
  return (ROLE_LEVEL[user.role] ?? 0) >= (ROLE_LEVEL[minimum] ?? 999);
}

/** Convenience: is the user an admin? */
export function isAdmin(user: AuthUser | null | undefined): boolean {
  return hasRole(user, ROLES.ADMIN);
}
```

### 1c. Expose role on the GraphQL `Me` query

The existing `Me` query already returns `Role`. No schema change needed. The frontend `AuthUser` type already stores `role: string`. No change needed.

### 1d. Define an `EntityPolicy` trait (backend)

```rust
/// Policy checked by generated resolvers before executing operations.
/// Implement per-entity to customize access rules beyond role guards.
#[async_trait::async_trait]
pub trait EntityPolicy: Send + Sync {
    /// Called before List/GetById queries. Return a WHERE clause fragment
    /// to scope results, or Err to deny entirely.
    async fn scope_read(
        &self,
        ctx: &Context<'_>,
        user: &AuthUser,
    ) -> Result<Option<(String, Vec<SqlValue>)>>;

    /// Called before Create/Update/Delete mutations on a specific entity ID.
    /// Return Ok(()) to allow, Err to deny.
    async fn authorize_write(
        &self,
        ctx: &Context<'_>,
        user: &AuthUser,
        entity_id: Option<&str>,
    ) -> Result<()>;
}
```

This trait is the extension point for ownership checks, library-scoping, and custom business rules. Most entities will use one of the built-in implementations (see below).

---

## Phase 2: Macro Attributes for Entity-Level RBAC

### 2a. New `#[graphql_entity(...)]` attributes

Extend `EntityMetadata` in `macros/src/lib.rs`:

```rust
struct EntityMetadata {
    // ... existing fields ...

    /// Role required for read operations (List, GetById).
    /// Default: any authenticated user.
    auth_read: Option<String>,    // e.g., "member"

    /// Role required for write operations (Create, Update, Delete).
    /// Default: any authenticated user.
    auth_write: Option<String>,   // e.g., "admin"

    /// Column name for ownership scoping. When set, List/GetById are
    /// filtered by `<column> = auth_user.user_id`, and mutations verify
    /// the caller owns the entity. Admins bypass ownership checks.
    owner_field: Option<String>,  // e.g., "user_id"

    /// When set, the entity is scoped through library access. The value
    /// is the column name holding the library_id. Queries are filtered
    /// to libraries the user has access to. Admins bypass.
    library_scope: Option<String>, // e.g., "library_id"

    /// Optional relation-based scope for entities without direct owner/library columns.
    /// Example value: "show:show_id->id->library_id" for Episode
    /// (join from entity.show_id to shows.id, then scope on shows.library_id).
    scope_via: Option<String>,

    /// Completely disable generated mutations. The entity is read-only
    /// through the generated API. Custom mutations can still be added
    /// via the CustomOperations struct.
    read_only: bool,

    /// Disable specific generated operations.
    /// e.g., skip = "create,delete,delete_many"
    skip_operations: Vec<String>,
}
```

Parse them in `parse_entity_metadata`:

```rust
} else if meta.path.is_ident("auth_read") {
    let value = meta.value()?;
    let lit: syn::LitStr = value.parse()?;
    metadata.auth_read = Some(lit.value());
} else if meta.path.is_ident("auth_write") {
    let value = meta.value()?;
    let lit: syn::LitStr = value.parse()?;
    metadata.auth_write = Some(lit.value());
} else if meta.path.is_ident("owner_field") {
    let value = meta.value()?;
    let lit: syn::LitStr = value.parse()?;
    metadata.owner_field = Some(lit.value());
} else if meta.path.is_ident("library_scope") {
    let value = meta.value()?;
    let lit: syn::LitStr = value.parse()?;
    metadata.library_scope = Some(lit.value());
} else if meta.path.is_ident("read_only") {
    metadata.read_only = true;
} else if meta.path.is_ident("skip") {
    let value = meta.value()?;
    let lit: syn::LitStr = value.parse()?;
    metadata.skip_operations = lit.value()
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
}
```

### 2b. Inject guards into generated resolvers

The macro will emit different authorization preambles based on the attributes.

**Queries (List, GetById):**

```rust
// Current (all generated queries):
let _user = ctx.auth_user()?;

// New (when auth_read = "admin"):
let _user = ctx.auth_user()?;
crate::graphql::auth::require_role(ctx, crate::graphql::auth::Role::Admin)?;

// New (when owner_field = "user_id"):
let _user = ctx.auth_user()?;
// Auto-append WHERE clause: "user_id = ?" with _user.user_id
// (admins bypass)

// New (when library_scope = "library_id"):
let _user = ctx.auth_user()?;
// Auto-append WHERE clause: "library_id IN (SELECT library_id FROM user_library_access WHERE user_id = ?)"
// (admins bypass)

// New (when scope_via is set for child entities):
let _user = ctx.auth_user()?;
// Auto-append relation-aware EXISTS or IN subquery that scopes through parent table.
// Example for Episode:
// EXISTS (
//   SELECT 1 FROM shows s
//   WHERE s.id = episodes.show_id
//     AND s.library_id IN (SELECT library_id FROM user_library_access WHERE user_id = ?)
// )
```

**Mutations (Create, Update, Delete):**

```rust
// Current:
let _user = ctx.auth_user()?;

// New (when auth_write = "admin"):
let _user = ctx.auth_user()?;
crate::graphql::auth::require_role(ctx, crate::graphql::auth::Role::Admin)?;

// New (when owner_field = "user_id" on Create):
// Auto-set the owner_field column to _user.user_id in the INSERT

// New (when owner_field = "user_id" on Update/Delete):
// Fetch entity first, verify entity.user_id == _user.user_id (or admin)
```

**Skipped operations:**

When `read_only = true`, the macro should still emit the mutations struct type, but omit generated mutation methods.
When `skip = "create,delete"`, those specific resolver methods are omitted.

### 2c. Example entity annotations after implementation

```rust
// Sensitive — admin-only CRUD
#[graphql_entity(
    table = "app_settings",
    plural = "AppSettings",
    default_sort = "key",
    auth_read = "admin",
    auth_write = "admin"
)]
pub struct AppSetting { ... }

// Security — no generated mutations, admin-only read
#[graphql_entity(
    table = "refresh_tokens",
    plural = "RefreshTokens",
    default_sort = "created_at",
    auth_read = "admin",
    read_only
)]
pub struct RefreshToken { ... }

// User-owned, library-scoped content
#[graphql_entity(
    table = "movies",
    plural = "Movies",
    default_sort = "title",
    owner_field = "user_id",
    library_scope = "library_id"
)]
pub struct Movie { ... }

// Multi-user but not library-scoped
#[graphql_entity(
    table = "users",
    plural = "Users",
    default_sort = "username",
    auth_write = "admin",
    skip = "delete_many"
)]
pub struct User { ... }
```

### 2d. Schema impact — codegen consequences

When `read_only` or `skip` removes operations from the backend schema:
- The corresponding `Create*Input`, `Update*Input`, `Delete*Result` types **disappear from the introspected schema**.
- After running `pnpm codegen`, those types are removed from `generated/graphql.ts` and `generated/types.ts`.
- Any frontend code referencing removed mutation document nodes or input types will get **compile-time TypeScript errors** from `pnpm exec tsc --noEmit`.
- This is intentional — it makes forbidden operations impossible to call from the frontend.

---

## Phase 3: Per-Field Read/Write Guards

### 3a. New field-level attributes

Add two new field-level attributes processed by the macro:

```rust
/// Only users with this role can see the field value in query results.
/// Other users receive the field's default/null value.
#[field_read_role = "admin"]

/// Only users with this role can set the field via Create/Update inputs.
/// The field is omitted from the generated input types for lower roles,
/// or rejected at runtime.
#[field_write_role = "admin"]
```

### 3b. Read guard — conditional field masking

For fields marked with `#[field_read_role = "..."]`, the macro modifies the `FromSqlRow` / field-assignment code to check the caller's role and return a masked value when unauthorized.

This works by generating a **custom `ComplexObject` field** instead of a `SimpleObject` field:
- The field is marked `#[graphql(skip)]` on the `SimpleObject` derive.
- A `ComplexObject` resolver is generated that checks the role from context before returning the real value.

```rust
// Generated for: #[field_read_role = "admin"] pub email: Option<String>
#[ComplexObject]
impl User {
    #[graphql(name = "Email")]
    async fn email_guarded(&self, ctx: &Context<'_>) -> Option<String> {
        let user = ctx.data_opt::<AuthUser>();
        match user {
            Some(u) if u.is_admin() => self.email.clone(),
            _ => None, // masked for non-admin
        }
    }
}
```

For non-nullable fields, the macro emits a compile error — field read guards are only valid on `Option<T>` fields (since we need a safe "hidden" value).

**Codegen impact:** The GraphQL schema type for the field remains `String` (nullable). The generated TypeScript type stays `Maybe<string>`. Frontend code already handles `null` via optional chaining. No type breakage.

### 3c. Write guard — conditional input inclusion

For fields marked with `#[field_write_role = "..."]`, the macro:

1. Still includes the field in the generated `CreateXInput` / `UpdateXInput` (as `Option<T>` in the update input).
2. In the mutation resolver, before processing the field:
   ```rust
   // Generated check for #[field_write_role = "admin"] on `role` field
   if input.role.is_some() {
       crate::graphql::auth::require_role(ctx, crate::graphql::auth::Role::Admin)?;
   }
   ```
3. If the caller doesn't have the required role and provides the field, the mutation returns a FORBIDDEN error.
4. If the caller doesn't provide the field (None in update), it silently skips — no error.

**Codegen impact:** The input type still includes the field in the schema. The generated TypeScript input type is unchanged. The enforcement is purely runtime — the frontend may submit the field, but the backend rejects it for unauthorized users.

### 3d. Example per-field annotations

```rust
pub struct User {
    #[primary_key]
    pub id: String,

    pub username: String,

    /// Only admins can see other users' emails
    #[field_read_role = "admin"]
    pub email: Option<String>,

    #[graphql(skip)]
    pub password_hash: String,

    /// Only admins can change roles
    #[field_write_role = "admin"]
    pub role: String,

    pub display_name: Option<String>,

    /// Only admins can deactivate users
    #[field_write_role = "admin"]
    pub is_active: bool,

    // ...
}
```

---

## Phase 4: Runtime Authorization Helpers (Backend)

Add to `backend/src/services/graphql/auth.rs`:

```rust
/// Require the caller to have at least the given role.
pub fn require_role(ctx: &Context<'_>, minimum: Role) -> Result<()> {
    let user = ctx.auth_user()?;
    if user.role_enum() >= minimum {
        Ok(())
    } else {
        Err(async_graphql::Error::new(format!(
            "Insufficient permissions: {:?} role required",
            minimum
        ))
        .extend_with(|_, e| e.set("code", "FORBIDDEN")))
    }
}

/// Check library access for the current user.
/// Admins bypass the check.
pub async fn require_library_access(
    ctx: &Context<'_>,
    library_id: &str,
) -> Result<()> {
    let user = ctx.auth_user()?;
    if user.is_admin() {
        return Ok(());
    }
    let db = ctx.data_unchecked::<Database>();
    let has_access = has_library_access(db, &user.user_id, library_id).await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
    if has_access {
        Ok(())
    } else {
        Err(async_graphql::Error::new("Access denied to this library")
            .extend_with(|_, e| e.set("code", "FORBIDDEN")))
    }
}

/// Build a WHERE clause fragment that scopes to the user's accessible libraries.
/// Returns None for admins (no scoping needed).
pub async fn library_scope_clause(
    ctx: &Context<'_>,
    column: &str,
) -> Result<Option<(String, Vec<SqlValue>)>> {
    let user = ctx.auth_user()?;
    if user.is_admin() {
        return Ok(None);
    }
    // Subquery approach — no extra round trip
    Ok(Some((
        format!(
            "{} IN (SELECT library_id FROM user_library_access WHERE user_id = ?)",
            column
        ),
        vec![SqlValue::String(user.user_id.clone())],
    )))
}

/// Build a WHERE clause fragment that scopes to the current user's owned entities.
/// Returns None for admins (no scoping needed).
pub fn owner_scope_clause(
    ctx: &Context<'_>,
    column: &str,
) -> Result<Option<(String, Vec<SqlValue>)>> {
    let user = ctx.auth_user()?;
    if user.is_admin() {
        return Ok(None);
    }
    Ok(Some((
        format!("{} = ?", column),
        vec![SqlValue::String(user.user_id.clone())],
    )))
}
```

---

## Phase 5: Entity Classification & Migration

### Entity security tiers

| Tier | Entities | auth_read | auth_write | Scoping |
|------|----------|-----------|------------|---------|
| **System (admin-only)** | `app_settings`, `app_logs`, `schedule_cache`, `schedule_sync_state`, `naming_pattern` | `admin` | `admin` | — |
| **Auth (admin read, no generated mutations)** | `refresh_tokens`, `invite_tokens`, `users` | `admin` | `admin` | `read_only` for refresh_tokens; `skip = "delete_many"` for users |
| **User-owned + library-scoped (direct)** | `libraries`, `movies`, `shows`, `albums`, `artists`, `audiobooks`, `media_files`, `pending_file_matches` | — | — | `owner_field = "user_id"`, `library_scope = "library_id"` |
| **Library-scoped via relation (child entities)** | `episodes`, `tracks`, `chapters` | — | — | `scope_via` relation strategy (episode->show, track->album, chapter->audiobook) |
| **User-owned (no library)** | `torrents`, `torrent_files`, `usenet_downloads`, `usenet_servers`, `rss_feeds`, `rss_feed_items`, `notifications`, `playback_sessions`, `playback_progress` | — | — | `owner_field = "user_id"` |
| **Shared content** | `sources`, `source_priority_rules`, `torznab_categories`, `cast_devices`, `cast_sessions`, `cast_settings` | — | `admin` | — |
| **Derived/readonly** | `video_streams`, `audio_streams`, `subtitles`, `media_chapters`, `artwork_cache` | — | — | `read_only` or `skip = "create,update,delete"` |

### Migration steps (backend)

1. Add the new attributes to `EntityMetadata` and parser (macro crate).
2. Update the `GraphQLOperations` derive to emit guard code.
3. Annotate each entity file with the appropriate tier attributes.
4. Run `cargo check` to verify compilation.
5. Test each entity tier with admin and non-admin JWTs.

---

## Phase 6: Frontend Changes

All frontend work follows from the backend schema changes. The workflow is:
1. Backend entity annotations change the GraphQL schema.
2. Run `pnpm codegen` to regenerate `generated/graphql.ts`, `generated/types.ts`, `generated/schema.json`.
3. Run `pnpm exec tsc --noEmit` to find all compile errors from removed/changed types.
4. Fix each error — either remove the UI path or gate it behind a role check.

### 6a. Apollo error link — handle FORBIDDEN

Update `frontend/src/lib/graphql/client.ts` `errorLink` to recognize `FORBIDDEN` alongside the existing `UNAUTHORIZED` handling:

```typescript
// In the errorLink handler, inside the graphqlErrors loop:
const isForbiddenError =
  err.extensions?.code === "FORBIDDEN" ||
  message.toLowerCase().includes("insufficient permissions");

if (isAuthError) {
  // Silently ignore — expected when not logged in
} else if (isForbiddenError) {
  // User is authenticated but lacks permission
  console.warn(
    `[GraphQL forbidden]: ${message}, Operation: ${operationName}`
  );
  notifyError(`Permission denied: ${message}`, false);
} else {
  console.error(
    `[GraphQL error]: Message: ${message}, Operation: ${operationName}`
  );
  notifyError(message, false);
}
```

This ensures FORBIDDEN errors show a toast via `addToast` instead of being silently swallowed or treated as unexpected errors.

### 6b. Role-aware UI utilities

Add a reusable component and hook in the frontend. Use the role helpers from `auth.ts` (Phase 1b) — never duplicate role logic.

**`frontend/src/components/shared/RequireRole.tsx`** — declarative gate:

```tsx
import type { ReactNode } from "react";
import { useAuth } from "../../hooks/useAuth";
import { hasRole, type RoleName } from "../../lib/auth";

interface RequireRoleProps {
  /** Minimum role needed to render children */
  minimum: RoleName;
  /** Content shown when the user lacks the role (default: nothing) */
  fallback?: ReactNode;
  children: ReactNode;
}

/**
 * Renders children only if the current user has at least the given role.
 * Use this to hide admin-only UI sections from members/guests.
 */
export function RequireRole({ minimum, fallback = null, children }: RequireRoleProps) {
  const { user } = useAuth();
  if (!hasRole(user, minimum)) return <>{fallback}</>;
  return <>{children}</>;
}
```

**Usage in components:**

```tsx
import { RequireRole } from "../shared/RequireRole";
import { ROLES } from "../../lib/auth";

// Hide entire settings nav item
<RequireRole minimum={ROLES.ADMIN}>
  <NavbarItem>
    <Link to="/settings">Settings</Link>
  </NavbarItem>
</RequireRole>

// Hide a delete button
<RequireRole minimum={ROLES.ADMIN}>
  <Button color="danger" onPress={handleDelete}>Delete User</Button>
</RequireRole>
```

### 6c. Navbar — role-gated navigation

Update `frontend/src/components/Navbar.tsx` to conditionally show nav items based on role:

```typescript
import { hasRole, ROLES, isAdmin } from "../lib/auth";

// Split nav items into always-visible and role-gated:
const navItems = [
  { to: "/", label: "Home" },
  { to: "/libraries", label: "Libraries" },
  { to: "/downloads", label: "Downloads" },
];

const adminNavItems = [
  { to: "/settings", label: "Settings" },
];

// In JSX — render adminNavItems only when isAdmin(user):
{adminNavItems.map((item) =>
  isAdmin(user) ? <NavbarItem key={item.to}>...</NavbarItem> : null
)}
```

Also gate the "Settings" item in the user dropdown menu.

### 6d. Settings routes — route-level guards

Add a route-level `beforeLoad` guard to the settings layout route. This prevents non-admins from navigating to `/settings/*` even via direct URL:

```typescript
// frontend/src/routes/settings/index.tsx (or settings layout route)
import { createFileRoute, redirect } from "@tanstack/react-router";
import { isAdmin } from "../../lib/auth";

export const Route = createFileRoute("/settings")({
  beforeLoad: ({ context }) => {
    if (!isAdmin(context.auth?.user)) {
      throw redirect({ to: "/" });
    }
  },
  component: SettingsPage,
});
```

### 6e. Codegen regeneration after schema changes

After annotating backend entities with `read_only` or `skip`, the GraphQL schema changes. The codegen workflow:

1. Start the backend (or export schema): `cargo run` or use `CODEGEN_SCHEMA_FILE=1` with saved `schema.json`.
2. Run codegen: `cd frontend && pnpm codegen`.
3. Regenerated files:
   - `generated/graphql.ts` — removed `TypedDocumentNode` exports for deleted mutations.
   - `generated/types.ts` — removed input/result types for deleted mutations.
   - `generated/schema.json` — updated introspection.
4. Run `pnpm exec tsc --noEmit` — any frontend code referencing removed types/documents fails at compile time.
5. Fix each error by removing the dead code path or gating it behind `RequireRole`.

**Key principle:** Never suppress codegen errors by adding manual type stubs. If a mutation is removed from the schema, remove the frontend code that called it.

### 6f. GraphQL document updates

Some `.graphql` documents in `frontend/src/lib/graphql/documents/` may reference operations that no longer exist after schema changes. For each removed operation:

1. Delete the operation from the `.graphql` file.
2. Run `pnpm codegen` to regenerate.
3. Fix TypeScript references to the removed document node.

For admin-only operations that still exist in the schema but are now gated:
- The `.graphql` document stays unchanged.
- The generated `TypedDocumentNode` still exists.
- The frontend wraps the call site in `RequireRole` or checks `isAdmin(user)` before calling.

### 6g. Handling masked fields (field_read_role)

When a field has `#[field_read_role = "admin"]`, non-admin users receive `null` for that field. The generated TypeScript type already marks it as `Maybe<string>` (nullable), so no type change occurs.

Frontend components that display these fields should handle `null` gracefully (they likely already do via `?.` or `?? "—"`). No special handling needed beyond what already exists for optional fields.

### 6h. Handling forbidden field writes (field_write_role)

When a mutation includes a field protected by `#[field_write_role = "admin"]` and the caller is not an admin, the backend returns a `FORBIDDEN` error. The frontend should:

1. **Proactively hide** the input field from non-admin users using `RequireRole`:
   ```tsx
   <RequireRole minimum={ROLES.ADMIN}>
     <Select label="Role" ...>
       ...
     </Select>
   </RequireRole>
   ```
2. The error link (Phase 6a) catches any `FORBIDDEN` that slips through as a toast.

### 6i. User management page adjustments

After RBAC, the `Users` entity list/detail queries are gated to admin-only. If a user management page exists (or will exist):
- Wrap the entire route in `RequireRole minimum="admin"`.
- The codegen types for `User`, `UserConnection`, `CreateUserInput`, etc. remain in `generated/types.ts` (they still exist in the schema for admin use).
- Non-admin frontends simply never call those operations.

### 6j. Library access — frontend scoping

After `library_scope` is applied to content entities, the backend automatically filters results. The frontend does **not** need to add `UserId` or `LibraryId` filters to queries — the backend handles scoping transparently.

What the frontend should do:
- Remove any manual `UserId` filters from queries if they exist (redundant with backend scoping).
- Trust that list queries return only what the user is allowed to see.
- On FORBIDDEN errors from mutations (e.g., trying to edit someone else's library), show the toast from the error link.

---

## Phase 7: Implementation Order

### Step 1 — Foundation (backend auth.rs + frontend auth.ts)
- Backend: Add `Role` enum, `AuthUser::role_enum()`, `AuthUser::is_admin()`, `require_role`, `owner_scope_clause`, `library_scope_clause`, `require_library_access`.
- Frontend: Add `ROLES`, `hasRole()`, `isAdmin()` to `frontend/src/lib/auth.ts`.
- Frontend: Create `frontend/src/components/shared/RequireRole.tsx`.
- Backend: Add tests for role hierarchy and scope clause generation.

### Step 2 — Frontend error handling
- Update `errorLink` in `frontend/src/lib/graphql/client.ts` to handle `FORBIDDEN` error code.
- Verify with manual testing (temporarily add a role guard to one resolver, confirm toast appears for non-admin).

### Step 3 — Macro: entity-level attributes
- Extend `EntityMetadata` with new fields.
- Update `parse_entity_metadata` to parse them.
- Modify generated query resolvers to inject role checks and scope clauses.
- Modify generated mutation resolvers to inject role checks and ownership validation.
- Implement `read_only` and `skip` to suppress generated operations.
- Verify with `cargo check`.

### Step 3.5 — Pilot rollout before broad annotation
- Apply RBAC to one admin-only entity first (e.g., `app_settings`).
- Apply scope to one direct library entity (e.g., `movies`).
- Apply relation-scoped guard to one child entity (e.g., `episodes`).
- Run backend tests + frontend codegen/typecheck before tier-wide rollout.

### Step 4 — Macro: per-field attributes
- Add `FieldMetadata` parsing for `field_read_role` and `field_write_role`.
- For read guards: convert guarded fields to `#[graphql(skip)]` + generated `ComplexObject` resolver.
- For write guards: inject role check in mutation before processing the field.
- Verify with `cargo check`.

### Step 5 — Annotate entities (backend, then codegen, then frontend)

For each entity tier (start with highest-risk):

1. **Backend:** Add annotations to entity struct.
2. **Backend:** `cargo check && cargo test`.
3. **Codegen:** With backend running, `cd frontend && pnpm codegen`.
4. **Frontend:** `pnpm exec tsc --noEmit` — fix any type errors.
5. **Frontend:** Add `RequireRole` gates where UI references admin-only operations.

Order:
1. `users`, `app_settings`, `refresh_tokens`, `invite_tokens` (admin-only / auth tier).
2. `libraries`, `movies`, `shows`, `episodes`, `albums`, `tracks`, `artists`, `audiobooks`, `chapters`, `media_files` (user-owned + library-scoped).
3. `torrents`, `torrent_files`, `notifications`, `playback_sessions`, etc. (user-owned).
4. `sources`, `cast_*`, `torznab_categories` (shared / admin-write).
5. `video_streams`, `audio_streams`, `subtitles`, `media_chapters`, `artwork_cache` (derived / read-only).

### Step 6 — Frontend navigation and route guards
- Gate settings nav items behind `isAdmin(user)` in `Navbar.tsx`.
- Add `beforeLoad` redirect guard to `/settings/*` routes.
- Gate admin-only buttons (user management, source management) with `RequireRole`.
- Remove any now-dead frontend code that called removed mutations.

### Step 7 — Integration testing
- Write integration tests that exercise:
  - Admin can CRUD all entities.
  - Member can only CRUD owned entities.
  - Member cannot access admin-only entities.
  - Library scoping filters results correctly.
  - Per-field write guards reject unauthorized field changes.
  - Per-field read guards mask values for non-admins.
- Test the `execute_mutation` internal path (used by services/background jobs) still works — it passes an `AuthUser` with admin role.
- Frontend: verify `pnpm exec tsc --noEmit` passes with zero errors after all changes.
- Frontend: verify `pnpm codegen` produces stable output (running twice yields no diff).

### Step 8 — Cleanup
- Remove any manual `UserId` filters from frontend queries that are now redundant.
- Remove any commented-out admin-only mutations from `.graphql` documents.
- Verify all `.graphql` documents reference only operations that exist in the schema.

---

## Consistency Checklist

These rules ensure the RBAC system is consistent across the stack:

| Rule | Backend | Frontend |
|------|---------|----------|
| Role names | `Role` enum: `Guest`, `Member`, `Admin` | `ROLES` const: `"guest"`, `"member"`, `"admin"` |
| Role check helper | `require_role(ctx, Role::Admin)?` | `hasRole(user, ROLES.ADMIN)` / `isAdmin(user)` |
| Error code for denied access | `.extend_with(\|_, e\| e.set("code", "FORBIDDEN"))` | `err.extensions?.code === "FORBIDDEN"` in error link |
| Removed operations | Macro `read_only` / `skip` omits resolver from schema | Codegen removes `TypedDocumentNode` + input types; `tsc` catches dead references |
| Masked fields | `#[field_read_role]` returns `null` to unauthorized | Field already `Maybe<T>` in generated types; no change needed |
| Guarded field writes | `#[field_write_role]` returns FORBIDDEN at runtime | `RequireRole` hides the input; error link catches fallthrough |
| UI visibility | N/A | `RequireRole` component wraps admin-only UI |
| Route protection | N/A | `beforeLoad` in TanStack Router redirects unauthorized |
| Types source of truth | Entity struct + macro → GraphQL schema | `pnpm codegen` → `generated/graphql.ts` + `generated/types.ts` (never hand-written) |
| Naming | PascalCase in GraphQL schema (enforced by existing rules) | PascalCase in `.graphql` documents and generated types (enforced by codegen config) |
| Bulk update result casing | `Update<Plural>Result` uses `success`, `error`, `affectedCount` | Documents and UI should query/use lowercase `success/error` and `affectedCount` |

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Breaking internal service calls that use `execute_mutation` with a non-admin user | Audit all `execute_mutation` callers; ensure they construct `AuthUser` with `role: "admin"`. The `get_default_user_id` helper should return the admin user. |
| Child entities cannot be scoped by direct `owner_field`/`library_scope` | Add relation-aware scoping (`scope_via`) and test Episode/Track/Chapter access explicitly before enabling broad policy defaults. |
| GraphQL casing drift (`Success/Error` vs `success/error`) causes frontend/codegen breakage | Standardize result field naming in generated types and examples; validate all docs via `pnpm codegen` in CI. |
| Per-field read guards changing field nullability in the GraphQL schema | Only apply `field_read_role` to already-`Option<T>` fields. Document this constraint. Codegen types already use `Maybe<T>` for these. |
| Performance impact of library scope subqueries | The `user_library_access` table is small and indexed. For hot paths, consider caching user library IDs in context at request start. |
| Macro complexity increase | Keep the guard injection as simple token insertion. Add macro-level integration tests using `trybuild`. |
| Guest role behavior undefined | Guests get read-only access to libraries they're granted. No write operations. Define clearly in role enum ordering. |
| Codegen drift — frontend types out of sync with backend schema | Always run `pnpm codegen` after backend entity changes. CI should run codegen and fail if output differs from committed files. |
| Frontend references to removed operations cause runtime errors | Run `pnpm exec tsc --noEmit` after every codegen. TypeScript catches all references to removed document nodes at compile time. |
| Apollo cache contains stale data after role changes (e.g., user promoted to admin) | Existing `resetApolloCache()` is called on login/logout. Role changes require re-login (new JWT), which already triggers a cache reset. |

---

## Non-Goals (Out of Scope)

- **External SSO/OAuth** — design doc says JWT-only local auth.
- **Per-row ACLs** — ownership + library-scope + role is sufficient for a personal media server.
- **Field-level encryption at rest** — already handled by the `#[transform]` system for credentials.
- **API rate limiting by role** — separate concern, not part of RBAC.
- **REST endpoint auth** — tracked in the code review as a separate workstream (middleware on `/api/*`).
- **Custom frontend type overrides** — all types come from codegen; no manual type stubs for RBAC changes.
