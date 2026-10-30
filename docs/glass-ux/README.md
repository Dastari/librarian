# Glass UX reference installation

Reviewed on 2026-10-29 against **0.11.7**. Release API metadata is retained in
`tmp/glass-ux-0.11.7/`; archive checksums were verified before installation.

- [Release and assets](https://github.com/Dastari/glass-ux/releases/tag/v0.11.7)
- [Documentation site](https://dastari.github.io/glass-ux/)
- [Migration plan](../glass-ux-migration-plan.md)
- [Complete library catalogue](component-catalogue.md)
- [Site inventory](site-inventory.md)

## Installed files

`web/package.json` and `web/pnpm-lock.yaml` pin the release archive. `pnpm install
--frozen-lockfile` restores the package, declaration files, offline Markdown/API
documentation, fonts, assets and CLI under `web/node_modules/glass-ux/`.
Global CSS, the Vite plugin and providers are now integrated. See the
[foundation progress record](foundation-progress.md) for implemented scope and checks.

All six release skills are committed under `.agents/skills/` (Codex and other
agents) and `.claude/skills/` (Claude Code), installed with the release CLI's
`agents` and `claude` targets:

- `glass-ux-adopt`
- `glass-ux-components`
- `glass-ux-forms`
- `glass-ux-data`
- `glass-ux-assistant`
- `glass-ux-author`

The project skills were refreshed using the 0.11.7 release CLI and are available
to skill discovery on the next turn. The refresh command, run from the repository
root, is:

```sh
web/node_modules/.bin/glass-ux skills install --target agents --update
web/node_modules/.bin/glass-ux skills install --target claude --update
```

Run that only when deliberately updating the installed Glass UX skill copies.
Project instructions remain authoritative. Library authoring instructions apply
to generic upstream components; consuming-app adapters stay in Librarian.

The full shared [UI rules](ui-rules.md), [layout contract](layout-contract.md),
[icon specification](icon-spec.md), and
[MIT license](LICENSE) are copied verbatim from the release package for review in
git. `.cursor/rules/glass-ux.mdc` connects them to the host integration rules;
`frontend-ui.mdc` defines the legacy-to-Glass migration boundary. On upgrade,
refresh these copies and skills together with the package, review its changelog,
and update this provenance record. Installed docs take precedence if versions differ.

## Downloaded offline documentation

The current download is in the ignored `tmp/glass-ux-0.11.7/` directory:

- `glass-ux-0.11.7.tgz`: the published runtime package.
- `glass-ux-docs.tar.gz`: the entire built documentation site.
- `site/`: extracted documentation site, including demos and API reference.
- `SHA256SUMS`: publisher-provided checksums; both archives passed verification.

These are local reference files, not application assets or vendored source. The
application should never precache or ship the docs site. To reproduce the download:

```sh
mkdir -p tmp/glass-ux-0.11.7/site
cd tmp/glass-ux-0.11.7
curl -fLO https://github.com/Dastari/glass-ux/releases/download/v0.11.7/glass-ux-0.11.7.tgz
curl -fLO https://github.com/Dastari/glass-ux/releases/download/v0.11.7/glass-ux-docs.tar.gz
curl -fLO https://github.com/Dastari/glass-ux/releases/download/v0.11.7/SHA256SUMS
sha256sum -c SHA256SUMS
tar -xzf glass-ux-docs.tar.gz -C site
```

The initial 0.9.10 source checkout at `/tmp/librarian-glass-ux` is historical
implementation-review material, not a current-release reference. Use installed release docs and declarations when writing
consumer calls, rather than depending on that temporary checkout.

The local table focus-capture patch remains necessary in 0.11.7 and is rebased
in `web/patches/glass-ux@0.11.7.patch`, registered by pnpm and covered by tests.
It also adds a continuation-only `infinite.revision` to preserve scroll/focus during
equal-count live refreshes; [patch notes](../../web/patches/README.md) describe the contract.
See [the upgrade audit](upgrade-audit-2026-10-29.md) for route coverage and outstanding gaps.
