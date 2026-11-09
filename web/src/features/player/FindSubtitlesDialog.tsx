import { useMutation } from "@apollo/client/react";
import { Badge } from "glass-ux/badge";
import { Button } from "glass-ux/button";
import { Dialog } from "glass-ux/dialog";
import { IconDownload } from "glass-ux/icons";
import { Notice } from "glass-ux/notice";
import { Select } from "glass-ux/select";
import { Spinner } from "glass-ux/spinner";
import { toast } from "glass-ux/toast";
import { useEffect, useState } from "react";

import { DownloadSubtitleDocument, SearchSubtitlesDocument, type SearchSubtitlesMutation } from "@/graphql/generated/graphql";
import { errorMessage } from "@/lib/graphql/errors";
import { LANGUAGES } from "@/lib/languages";

type Candidate = SearchSubtitlesMutation["searchSubtitles"]["candidates"][number];

const PREFERRED = "preferred";
const LANGUAGE_OPTIONS = [{ value: PREFERRED, label: "Preferred languages" }, ...LANGUAGES.map((language) => ({ value: language.code, label: language.name }))];

interface FindSubtitlesDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  mediaFileId: string;
  title: string;
  /** Called with the new subtitle's id once it is saved, to show it. */
  onDownloaded: (subtitleId: string) => void;
}

/** What a result is, in a few words: the flags a person chooses by. */
function CandidateFacts({ candidate }: { candidate: Candidate }) {
  return (
    <span className="flex flex-wrap items-center gap-1.5">
      {candidate.hashMatch ? <Badge tone="success">Matches this file</Badge> : null}
      {candidate.hearingImpaired ? <Badge>SDH</Badge> : null}
      {candidate.forced ? <Badge>Forced</Badge> : null}
      {candidate.machineTranslated ? <Badge tone="warning">Machine translated</Badge> : null}
      {candidate.trusted ? <Badge tone="info">Trusted</Badge> : null}
    </span>
  );
}

/**
 * Search OpenSubtitles for the playing file and download one beside it (Q119). Results come best
 * first: those made for this very file, then the preferred languages, then by downloads and rating.
 */
export function FindSubtitlesDialog({ open, onOpenChange, mediaFileId, title, onDownloaded }: FindSubtitlesDialogProps) {
  const [language, setLanguage] = useState<string>(PREFERRED);
  const [search, searchState] = useMutation(SearchSubtitlesDocument);
  const [download] = useMutation(DownloadSubtitleDocument);
  const [result, setResult] = useState<SearchSubtitlesMutation["searchSubtitles"] | null>(null);
  const [downloading, setDownloading] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    setResult(null);
    void search({ variables: { mediaFileId, languages: language === PREFERRED ? null : [language] } })
      .then(({ data }) => { if (!cancelled && data) setResult(data.searchSubtitles); })
      .catch((error: unknown) => { if (!cancelled) setResult({ success: false, error: errorMessage(error), candidates: [], languages: [], autoSkipReason: null }); });
    return () => { cancelled = true; };
  }, [open, mediaFileId, language, search]);

  const take = async (candidate: Candidate) => {
    setDownloading(candidate.fileId);
    try {
      const { data } = await download({ variables: { input: {
        mediaFileId,
        fileId: candidate.fileId,
        language: candidate.language,
        hearingImpaired: candidate.hearingImpaired,
        forced: candidate.forced,
        release: candidate.release,
        uploader: candidate.uploader,
      } } });
      const payload = data?.downloadSubtitle;
      if (!payload?.success || !payload.subtitle) {
        toast.danger(payload?.error ?? "The subtitle could not be downloaded");
        return;
      }
      const left = payload.remainingDownloads;
      toast.success(`Downloaded ${candidate.languageName ?? "the"} subtitles${typeof left === "number" ? ` · ${left} download${left === 1 ? "" : "s"} left today` : ""}`);
      onDownloaded(payload.subtitle.id);
      onOpenChange(false);
    } catch (error) {
      toast.danger(errorMessage(error));
    } finally {
      setDownloading(null);
    }
  };

  const candidates = result?.candidates ?? [];
  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Popup size="lg" className="min-h-0">
        <Dialog.Header title="Find subtitles" subtitle={title} />
        <Dialog.Body className="flex min-h-0 flex-col gap-4">
          <div className="flex flex-wrap items-center gap-3">
            <span className="text-label text-muted">Language</span>
            <Select className="min-w-56" options={LANGUAGE_OPTIONS} value={language} onValueChange={(value) => setLanguage(String(value ?? PREFERRED))} aria-label="Language" />
          </div>
          {result?.autoSkipReason ? (
            <p className="text-body-sm text-muted">{result.autoSkipReason}, so subtitles are not fetched for it automatically.</p>
          ) : null}
          {searchState.loading || (open && !result) ? (
            <div className="flex items-center gap-3 py-6 text-muted"><Spinner size="sm" /> Searching OpenSubtitles…</div>
          ) : result && !result.success ? (
            <Notice tone="danger" role="alert">{result.error ?? "The search failed"}</Notice>
          ) : candidates.length === 0 ? (
            <p className="py-6 text-body-sm text-muted">OpenSubtitles has nothing for this in that language.</p>
          ) : (
            <ul className="flex min-h-0 flex-col divide-y divide-border overflow-y-auto" aria-label="Subtitles found">
              {candidates.map((candidate) => (
                <li key={candidate.fileId} className="flex items-center gap-3 py-3">
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-body-sm text-foreground">
                      <span className="font-medium">{candidate.languageName ?? candidate.language ?? "Unknown language"}</span>
                      {candidate.release ? <span className="text-muted"> · {candidate.release}</span> : null}
                    </p>
                    <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-label-sm text-muted">
                      <CandidateFacts candidate={candidate} />
                      <span className="text-numeric">{candidate.downloads.toLocaleString()} downloads</span>
                      {candidate.votes > 0 ? <span className="text-numeric">Rated {candidate.rating.toFixed(1)}</span> : null}
                      {candidate.uploader ? <span>by {candidate.uploader}</span> : null}
                    </div>
                  </div>
                  <Button size="sm" variant="secondary" icon={<IconDownload />} onClick={() => void take(candidate)} disabled={Boolean(downloading)} aria-label={`Download ${candidate.languageName ?? ""} ${candidate.release ?? ""}`.trim()}>
                    {downloading === candidate.fileId ? "Downloading…" : "Download"}
                  </Button>
                </li>
              ))}
            </ul>
          )}
        </Dialog.Body>
      </Dialog.Popup>
    </Dialog.Root>
  );
}
