import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const root = resolve(__dirname, "../..");

function read(path: string) {
  return readFileSync(resolve(root, path), "utf8");
}

describe("media workflow GraphQL documents", () => {
  it("uses camelCase custom resolver fields", () => {
    const documents = [
      read("src/lib/graphql/documents/media-add.graphql"),
      read("src/lib/graphql/documents/torrents.graphql"),
      read("src/lib/graphql/documents/libraries.graphql"),
      read("src/lib/graphql/documents/manual-match.graphql"),
      read("src/lib/graphql/documents/app_settings.graphql"),
    ].join("\n");

    expect(documents).toContain("addTorrent(input:");
    expect(documents).toContain("processSource(sourceType:");
    expect(documents).toContain("rematchSource(");
    expect(documents).toContain("analyzeMediaFile(mediaFileId:");
    expect(documents).toContain("scanLibrary(id:");
    expect(documents).toContain("matchMediaFile(input:");
    expect(documents).toContain("testOllamaConnection(input:");
    expect(documents).toContain("testLlmParser(input:");

    expect(documents).not.toMatch(/^\s*(AddTorrent|ProcessSource|RematchSource|AnalyzeMediaFile|ScanLibrary|MatchMediaFile)\s*\(/m);
  });

  it("passes selected library/source intent when adding from search", () => {
    const modal = read("src/components/search/AddToLibraryModal.tsx");

    expect(modal).toContain("libraryId: selectedLibraryId");
    expect(modal).toContain("sourceUrl: torrentUrl || magnetUri");
    expect(modal).toContain("sourceIndexerId: release.indexerName");
  });
});
