# Object Storage and Backup

## Scope

Librarian stores artwork bytes through `graphql-orm-storage`. Media library files remain normal filesystem paths owned by the configured library roots.

Structured metadata stays in GraphQL ORM entities. No application service should read or write storage or backup metadata through direct SQL.

## Dependency Baseline

Librarian resolves its ORM companion packages from the consolidated
`https://github.com/Dastari/graphql-orm.git` monorepo at the reviewed revision
`55d0bd255ce0ed913be86f510773a8ba1baa6eed`:

- `graphql-orm` `0.16.0`, with the application `sqlite` feature forwarding to
  `graphql-orm/sqlite`
- `graphql-orm-storage` `0.6.0`, with only the `local` provider enabled
- `graphql-orm-backup` `0.7.0`, with only the `local` repository enabled

All three packages must remain pinned to one exact monorepo revision so their
public ORM and storage types share one Cargo source identity. `agql-auth`
remains an independent dependency.

## Storage Metadata Contract

`StorageObject` is the internal metadata row for bytes stored outside the database.

- `Id`: application row id
- `ObjectId`: provider-neutral object id returned by `graphql-orm-storage`
- `Namespace`: storage namespace such as `derivatives`
- `Backend`: storage backend such as `local`
- `StorageKey`: stable provider-neutral object key
- `OriginalFileName`: source filename metadata only
- `MimeType`: MIME metadata
- `SizeBytes`: byte length
- `Sha256Hex`: content checksum
- `CreatedAt` / `UpdatedAt`: ORM timestamps

`ArtworkCache` stores the cache lookup key and a `StorageObjectId` reference. It does not store image bytes.

## Artwork Flow

1. Metadata providers request artwork caching with an entity type, entity id, artwork type, and source URL.
2. `ArtworkService` checks for an existing `ArtworkCache` row.
3. If missing, it downloads the image with the configured max-size guard.
4. It detects MIME type and image dimensions.
5. It writes bytes using `StorageService::put_object` in the `Derivatives` namespace.
6. It persists `StorageObject` metadata through generated GraphQL ORM helpers.
7. It persists `ArtworkCache` metadata through generated GraphQL ORM helpers.
8. Consumers continue using `/api/artwork/{entity_type}/{entity_id}/{artwork_type}`.

If database persistence fails after object write, Librarian deletes the object bytes immediately and logs the cleanup result with object id, storage key, and cache context.

## Artwork API

`GET /api/artwork/{entity_type}/{entity_id}/{artwork_type}` resolves `ArtworkCache`, loads the linked `StorageObject`, reads bytes through the configured storage service, and returns:

- `Content-Type`: stored MIME type or `application/octet-stream`
- `Cache-Control`: `public, max-age=86400`
- `ETag`: stored SHA-256 checksum

Missing metadata or missing object bytes returns `404`.

## Backup Repository

Backups use `graphql-orm-backup` manifest repositories. The default local repository path is `./data/backups`.

Snapshot manifests live under:

```text
snapshots/{snapshot_id}/manifest.json
```

Object payload blobs are content addressed:

```text
objects/sha256/{sha[0..2]}/{sha[2..4]}/{sha}
```

## Capability Matrix

| Capability | Current state |
| --- | --- |
| Object backup index | Available |
| Object payload backup | Available |
| Snapshot manifest listing | Available |
| Snapshot verification | Available |
| Full logical database backup | Available |
| Incremental backup | Unavailable until `graphql-orm` change journal support is available |
| Empty-database logical restore | Available, with fail-closed backend/schema validation |
| Object rehydration during restore | Not yet wired |

`BackupCapabilities` reports this state to the Settings backup page.

## Unsupported States

- Librarian compiles only the local storage and backup providers; `s3`,
  `azure_blob`, and `smb` configuration is rejected by the application.
- Incremental backup and restore remain unavailable until Librarian enables
  and integrates the ORM change journal.
- Logical restore accepts only an empty database and does not yet rehydrate
  primary object-storage bytes.

## Consolidation Migration Impact

The repository move does not change storage keys, backup repository layout,
snapshot manifest format, ORM tables, or stored rows. No database or
persistent-data migration is required.

The consumer upgrade does make two source/API compatibility changes:

- generated GraphQL root object names are now `Query`, `Mutation`, and
  `Subscription`; the Rust root types remain `QueryRoot`, `MutationRoot`, and
  `SubscriptionRoot`
- the backup adapter repeats the source-manifest backend/schema-hash preflight
  before importing rows

The consolidated ORM's secure pagination default is lower than Librarian's
previous runtime behavior. Librarian explicitly selects
`PaginationConfig::legacy()` so existing requests retain the prior
1,000-row cap while callers are migrated to bounded pagination.

## Direct SQL Rule

Storage, artwork, and backup metadata must use generated GraphQL ORM query and mutation helpers. Do not add `sqlx::query*` or raw `SELECT`, `INSERT`, `UPDATE`, or `DELETE` paths for these domains.
