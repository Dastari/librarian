# Schema Migration Safety

Librarian treats Rust entity metadata as the target schema, but application startup is not permission to
transform or discard existing data.

## Startup policy

On every ordinary start, Librarian:

1. validates the live database against entity metadata;
2. builds the complete migration plan before executing a statement;
3. classifies every step;
4. automatically applies the plan only when every step is additive.

Additive changes include new tables, nullable/defaulted columns, and new indexes. Required columns without
defaults, constraint changes, index removal, type conversions, column removal, and table removal do not run
at startup. Any SQL type conversion is treated as destructive until migration-specific transformation code
proves how values are preserved. A field rename without `db_column` compatibility metadata plans as
add-plus-drop and is therefore destructive.

When startup blocks a plan, its error contains a sanitized step list, source and target schema hashes, the
stable plan hash, and the explicit command shape. Migration SQL is not printed.

`LIBRARIAN_ALLOW_DESTRUCTIVE_MIGRATIONS` is intentionally unsupported. There is no environment-variable
bypass.

## Explicit non-additive workflow

Run these steps with the current/old application version still available:

1. Create a full logical backup from **Settings → Backup** or the admin backup mutation.
2. Verify that snapshot and record its snapshot ID.
3. Stop Librarian so no request or worker can mutate the database during migration.
4. Install the new binary and print its proposed plan:

   ```text
   librarian schema plan
   ```

5. Review the classification and every listed table/column operation. For a rename or conversion, add and
   test migration-specific transformation support instead of approving an unintended drop/add.
6. Apply exactly the reviewed plan:

   ```text
   librarian schema apply \
     --plan-hash <PLAN_HASH> \
     --backup-snapshot <SNAPSHOT_ID>
   ```

The apply command fails closed unless:

- the plan hash still matches the current live-to-target plan;
- the snapshot is a full or synthetic-full Librarian snapshot;
- its manifest and every referenced table/object payload pass checksum verification;
- its database backend is SQLite;
- its schema hash exactly matches the plan's live source schema.

The ORM also rechecks the source schema hash immediately before application and records a successful
migration in its migration history. Re-run `librarian schema plan` after application; it must report no
pending plan before ordinary startup.

## Recovery

If application fails, do not start the new binary repeatedly. Preserve its error and:

1. restore the verified snapshot into an empty database using the previous binary/schema;
2. restore the matching object-storage payloads;
3. verify the restored snapshot and application health;
4. investigate or correct the migration-specific transformation before retrying.

Restore is deliberately empty-database-only. Never overlay a logical snapshot onto a partially migrated
database.

## Infrastructure exception

Schema planning/application and logical backup internals use the typed primitives supplied by
`graphql-orm` and `graphql-orm-backup`. This is infrastructure plumbing, not application-domain CRUD.
Services, resolvers, and jobs must continue to use generated entity queries and mutations and must not add
ad hoc SQL.
