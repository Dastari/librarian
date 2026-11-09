import { render, screen } from "@testing-library/react";
import { ActionProvider } from "glass-ux/actions";
import { TooltipProvider } from "glass-ux/tooltip";
import type { ReactNode } from "react";
import { describe, expect, it } from "vitest";

import { TorrentIdentity } from "../TorrentIdentity";
import type { DownloadRow } from "../useDownloads";

const wrapper = ({ children }: { children: ReactNode }) => <TooltipProvider><ActionProvider>{children}</ActionProvider></TooltipProvider>;

const row = (state: string, statusDetail: string | null) => ({
  id: "abc",
  record: { id: "abc", name: "Some.Release", infoHash: "abc", userId: "owner", state, progress: 0.4, totalBytes: 1024, downloadedBytes: 400, uploadedBytesTotal: 0, addedAt: "2026-11-09T00:00:00Z", libraryId: null, library: null, postProcessStatus: null, postProcessError: null, savePath: "smb://10.0.1.190/media/Downloads", minimumRatio: null, minimumSeedTimeMinutes: null, completedAt: null, season: null },
  live: { id: 1, infoHash: "abc", name: "Some.Release", state, progress: 0.4, size: 1024, downloaded: 400, uploaded: 0, downloadSpeed: 0, uploadSpeed: 0, peers: 0, savePath: "smb://10.0.1.190/media/Downloads/Some.Release", statusDetail, files: [] },
}) as unknown as DownloadRow;

describe("a download's identity", () => {
  it("says it is waiting for its share, and why, while the share is down", () => {
    render(<TorrentIdentity row={row("paused", "Waiting for the download share smb://10.0.1.190/media, unreachable since 2026-11-09 05:00 UTC. It resumes by itself when the share answers: Could not reach 10.0.1.190:445")} />, { wrapper });
    expect(screen.getByRole("img", { name: "Waiting for share" })).toBeInTheDocument();
    expect(screen.getByText(/It resumes by itself when the share answers/)).toBeInTheDocument();
  });

  it("shows the client's reason for an error", () => {
    render(<TorrentIdentity row={row("error", "The download share is unavailable: connection refused")} />, { wrapper });
    expect(screen.getByText(/connection refused/)).toBeInTheDocument();
  });

  it("shows its usual facts otherwise", () => {
    render(<TorrentIdentity row={row("downloading", null)} />, { wrapper });
    expect(screen.queryByRole("img", { name: "Waiting for share" })).not.toBeInTheDocument();
    expect(screen.getByText(/No library/)).toBeInTheDocument();
  });
});
