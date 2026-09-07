# Source credential key migration and rotation

Source/indexer credentials use AES-256-GCM with an exact 32-byte key. New values are stored in a
versioned envelope containing a non-secret key identifier, nonce, and ciphertext. The active key
must not be stored in SQLite.

Configure exactly one of:

- `LIBRARIAN_SOURCE_CREDENTIAL_KEY_FILE` (preferred), pointing to a regular file with Unix mode
  `0600` or stricter; or
- `LIBRARIAN_SOURCE_CREDENTIAL_KEY`, containing a base64-encoded 32-byte key.

`INDEXER_ENCRYPTION_KEY` remains a compatibility alias for the environment-variable form.
Malformed, short, long, unreadable, or over-permissive file keys fail closed. If no external key
is configured, the application remains available but the source-acquisition service is degraded
and credential-dependent source operations are disabled. It never generates a replacement key.

Generate a new key file without printing the key:

```sh
umask 077
openssl rand -base64 32 > /run/secrets/librarian-source-key.next
```

## Plan a migration or rotation

Stop normal application traffic, configure the current external key if the database already
contains `v2` envelopes, then run:

```sh
librarian credentials plan \
  --new-key-file /run/secrets/librarian-source-key.next
```

Planning decrypts and validates every credential and verifies that the new key can round-trip each
value. It does not mutate the database and never prints credential values or key IDs.

## Apply after a verified backup

Create a full backup through the Backup settings/API and copy its snapshot ID. Then run:

```sh
librarian credentials rotate \
  --new-key-file /run/secrets/librarian-source-key.next \
  --backup-snapshot SNAPSHOT_UUID
```

The command refuses incremental/unverified/wrong-application snapshots. It preflights every row,
rewrites through generated entity operations, reads each value back, and decrypts it with the new
key. If a write or verification fails, already-written rows are restored to their previous
envelopes; the verified full backup remains the final recovery path.

Only after all rows verify does the command remove the historical `sources_encryption_key`
application setting. Replace the active key configuration with the new file before normal startup,
restart, and test each credentialed source. Retain the old key and full backup until that
qualification succeeds.
