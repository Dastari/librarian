# Glass UX patch

`glass-ux@0.16.1.patch` is registered in `pnpm-workspace.yaml` and the lockfile.
It contains two bounded DataTable corrections and one media player addition:

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

On upgrade, inspect upstream for all three, regenerate the patch from the published
archive if needed, and run the real Glass table/Logs/URL-history regressions plus a
populated browser scroll/focus check before removing it.
