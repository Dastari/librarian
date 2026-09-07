# Phase 4a Progress: RBAC ownership scoping + invite redemption

Baseline: tree compiles, `cargo test` 64/64 passes (52 unit + 12 integration in backend_migration_contract.rs).

## Checklist

- [x] Item 1: Ownership RowPolicy for user-private entities
  - [x] Verify user_id column on PlaybackProgress, PlaybackSession, Notification, CastSession
        -> CastSession has NO user_id column (only device_id/media_file_id/episode_id);
           cast sessions belong to a shared cast device, not an individual user. Deviates
           from the task's premise ("all have a user_id column"). Left unscoped
           (falls through to `Ok(true)`), documented in row_policy.rs module docs.
  - [x] Write row_policy.rs with pure decision helper `ownership_allowed(role, row_user_id, requester_user_id)` + unit tests
  - [x] Wire installation alongside AppEntityPolicy (services/database.rs install_hooks)
  - [x] cargo check / cargo test (3 new unit tests: admin_can_access_any_row, member_can_access_only_own_row, unauthenticated_is_denied)
- [x] Item 2: Invite-gated registration
  - [x] Add inviteToken arg to register mutation (backend/src/services/graphql/mutations/auth.rs RegisterUserInput.invite_token)
  - [x] Implement pure validation helper `validate_invite_token(needs_setup, Option<&InviteTokenState>, now)` in backend/src/services/auth.rs + 8 unit tests (invite_token_tests module): first_user_bypasses_invite_requirement, missing_token_is_required_once_setup_is_complete, valid_token_is_accepted, expired_token_is_rejected, exhausted_token_is_rejected, inactive_token_is_rejected, unlimited_and_no_expiry_token_is_accepted, zero_max_uses_means_unlimited
  - [x] Wire into AuthService::register: looks up InviteToken by value (InviteToken.token is `#[graphql_orm(private)]` -> not filterable via WhereInput, so fetch_all + in-memory match, mirroring the existing RefreshToken.token_hash lookup pattern already in this file)
  - [x] Increment use_count / deactivate (is_active=false) on exhaustion via InviteToken::update_by_id; assign role from token.role parsed via Role::parse, default Member
  - [x] Frontend: SignInModal.tsx is the sole register-mutation UI (via useAuth.signUp). Added optional "Invite Code" input (required in sign-up branch, hidden during first-admin setup) threaded through useAuth.signUp -> RegisterDocument variables.inviteToken.
        GraphQL codegen has since been run successfully; `RegisterUserInput.inviteToken` and the auth operation artifacts are generated from the schema snapshot rather than hand-patched.
  - [x] cargo check / cargo test; frontend `pnpm exec tsc --noEmit` clean; `pnpm test` 19/19 passed
  - DEFERRED (explicitly out of scope): library_ids / access_level / restrictions_template on InviteToken are not applied to the new user (no library-scoping logic). Only role assignment + use-count/active bookkeeping implemented. No admin "create invite" mutation was added or required by this item.
- [x] Item 3: Login rate limiting
  - [x] `LoginRateLimiter` struct in new file backend/src/services/login_rate_limit.rs (parking_lot::Mutex<HashMap<String, Vec<Instant>>>, sliding window, default 10 attempts / 15 min, keys normalized via trim+lowercase, opportunistic pruning on every is_blocked/record_failure call, removes empty map entries). 6 unit tests: unknown_key_is_not_blocked, blocks_after_reaching_max_attempts, successful_login_clears_recorded_failures, keys_are_normalized_by_case_and_whitespace, distinct_keys_are_tracked_independently, stale_attempts_outside_the_window_are_pruned
  - [x] Wired into AuthService::login (backend/src/services/auth.rs): pre-check `is_blocked` -> generic "Too many attempts, try again later" (RATE_LIMITED_MESSAGE) before ever touching credentials/DB (doesn't reveal user existence); failure -> record_failure; success -> clear
  - [x] cargo check / cargo test (all green, see below)

## Deferred (explicitly out of scope for Phase 4a)
- library_ids / access_level / restrictions_template on InviteToken: not implemented (no library-scoping logic added). Token role assignment only.

## Notes / Design decisions
(filled in as work proceeds)

## Final verification
- [x] cargo check clean
- [x] cargo test: 81/81 passed (baseline 64 + 17 new: 3 row_policy + 8 invite_token + 6 login_rate_limit)
- [x] cargo clippy: pre-existing warnings only in untouched files (entities/chapter.rs, episode.rs, movie.rs, track.rs, torrent.rs, services/library_scan.rs); zero warnings in any file touched by this work
- [x] frontend: `pnpm exec tsc --noEmit` clean; `pnpm test` 19/19 passed

## Files touched
- backend/src/services/graphql/row_policy.rs (new)
- backend/src/services/graphql/mod.rs
- backend/src/services/database.rs (install_hooks: +2 lines)
- backend/src/services/auth.rs
- backend/src/services/graphql/mutations/auth.rs
- backend/src/services/login_rate_limit.rs (new)
- backend/src/services/mod.rs
- frontend/src/hooks/useAuth.ts
- frontend/src/components/SignInModal.tsx
- frontend/src/lib/graphql/generated/types.ts (hand-patched, see Item 2 notes re: regen)
- frontend/src/lib/graphql/generated/graphql.ts (hand-patched, see Item 2 notes re: regen)

Note: the repo tree had large pre-existing unrelated uncommitted changes in several of
these files (e.g. database.rs, auth.rs, the frontend generated files) before this task
began, per the process rules. `git diff` on these files shows much more than this task's
changes; the additions listed in the checklist above are the actual surgical diff
contributed here.

## STATUS: All 3 items complete.
