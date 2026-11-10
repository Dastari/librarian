# Glass UX patch

`glass-ux@0.19.5.patch` is registered in `pnpm-workspace.yaml` and the lockfile.
It contains three bounded DataTable additions, one media player addition, one EntityIdentity
correction, one EntitySettingsDialog addition and the TV remote's table context.

Checked against 0.19.2–0.19.5 on 2026-11-10: none of these was adopted upstream, and none of the
files the patch touches changed between 0.19.1 and 0.19.5, so the regenerated patch is byte-for-byte
the 0.19.1 one (same `patch_hash`). 0.19.1 had already adopted the settings dialog's `identity`
header, which was dropped then. Still patched:

- Native focus inside a grid synchronizes its active row, so virtualization retains
  the focused alphabet-jump target or row control. Portaled controls are excluded by
  the library's existing row lookup.
- The additive `DataTableInfinite.revision?: string | number` identifies a replacement
  snapshot for **continuation only**. Changing it rearms an equal-count batch without
  resetting scroll, active row or selection. `queryKey` continues to identify changed
  search/filter/sort/scope and retains the library's intentional scroll reset there.

- The additive `MediaStreamContext.setDuration(seconds)` reports a file's real length. The server's
  transcodes are growing HLS playlists, so the element under-reports the duration until the
  transcode finishes, and the seek bar and clock would otherwise show only what exists so far.

The compatibility adapter passes its `loadRevision`; Logs passes the connection's
`revision`. Append leaves this generation unchanged. No private modules are imported
by application code, and no table geometry or material recipes are overridden.

- `EntityIdentity` wraps its facts in a `<div>` instead of a `<p>` when they are more than text, because `facts`
  accepts a node and a Metric or Sparkline can't validly sit inside a paragraph. The download dialog's stats need it.

- `EntitySettingsTab.count` puts a count at the end of a settings tab (Files 29). To ask upstream for.

`DataTable` also takes `expanderColumn`, the column that holds a row detail's expander (default: the first visible). The Identified table puts it near the end of the row. To ask upstream for.

`DataTableRemoteContext` (exported from `glass-ux/data-table`) is true where a remote control drives the tables. A remote
never stops on column headers, so the table's toolbar then carries the sort menu that its card view already offers, in
table mode too. Nothing is folded into the header's settings menu there. `InputModeProvider` provides it in TV mode
(design.md Q141). To ask upstream for.

## Regenerating

On upgrade, inspect upstream for each of these, regenerate the patch from the published
archive if needed, and run the real Glass table/Logs/URL-history regressions plus a
populated browser scroll/focus check before removing it.

`pnpm patch` (11.25) cannot open a package installed from a release URL: it looks the name up in
the npm registry and fails with a 404. Regenerate the patch the way `patch-commit` writes it, from
the verified archive:

```sh
mkdir gen && tar -xzf glass-ux-X.Y.Z.tgz -C gen && cp -r gen/package gen/a && mv gen/package gen/b
(cd gen/b && git apply --reject /path/to/web/patches/glass-ux@OLD.patch)   # fix any *.rej by hand in gen/b
(cd gen && git diff --no-index --no-color --src-prefix=a/ --dst-prefix=b/ a b) \
  | sed -e 's#^diff --git a/a/#diff --git a/#' -e 's# b/b/# b/#' -e 's#^--- a/a/#--- a/#' -e 's#^+++ b/b/#+++ b/#' \
  > web/patches/glass-ux@X.Y.Z.patch
```

Then rename the `patchedDependencies` entry in `pnpm-workspace.yaml` and run `pnpm install`.
