import { useApolloClient, useMutation } from "@apollo/client/react";
import { formBehaviour, useAppForm } from "glass-ux/form";
import { Surface } from "glass-ux/surface";
import { toast } from "glass-ux/toast";
import { useEffect, useState } from "react";

import { Button, Dialog } from "@/components/ui";
import { GlassSwitch } from "@/components/ui/GlassControls";
import { StorageLocationField } from "@/features/servers/StorageLocationField";
import { DownloadsMovePlanDocument, MoveDownloadsDocument, type DownloadsMovePlanQuery } from "@/graphql/generated/graphql";
import { pluralize } from "@/lib/format";
import { errorMessage } from "@/lib/graphql/errors";

type Plan = DownloadsMovePlanQuery["downloadsMovePlan"];

const files = (count: number) => pluralize(count, "file");

/**
 * Move the download folder to a place its files were copied to, a folder here or on a saved
 * share. Like moving a library: copy first, check, then move. Every torrent keeps seeding from
 * its copy and nothing is downloaded again; the old files are left alone. "New downloads only"
 * changes where new torrents go and leaves the existing ones where they are.
 */
export function MoveDownloadsDialog({ open, onOpenChange, current }: { open: boolean; onOpenChange: (open: boolean) => void; current: string }) {
  const client = useApolloClient();
  const [move] = useMutation(MoveDownloadsDocument);
  const [plan, setPlan] = useState<Plan | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);
  const [moving, setMoving] = useState<"all" | "new" | null>(null);
  const [anyway, setAnyway] = useState(false);
  const form = useAppForm({ ...formBehaviour, defaultValues: { path: "" } });
  const forget = () => { setPlan(null); setProblem(null); setAnyway(false); };
  useEffect(() => { if (open) { forget(); form.reset({ path: "" }); } }, [open]); // eslint-disable-line react-hooks/exhaustive-deps

  const check = async (path: string) => {
    forget();
    setChecking(true);
    try {
      const { data, error } = await client.query({ query: DownloadsMovePlanDocument, variables: { newPath: path }, fetchPolicy: "network-only" });
      if (!data) throw error ?? new Error("The folder could not be checked");
      setPlan(data.downloadsMovePlan);
    } catch (failure) {
      setProblem(errorMessage(failure));
    } finally {
      setChecking(false);
    }
  };

  const apply = async (path: string, moveTorrents: boolean) => {
    setMoving(moveTorrents ? "all" : "new");
    setProblem(null);
    try {
      const { data } = await move({ variables: { input: { newPath: path, moveTorrents, allowMissing: anyway } } });
      const result = data?.moveDownloads;
      if (!result?.success) throw new Error(result?.error ?? "The download folder could not be moved");
      toast.success(moveTorrents
        ? `Downloads now read from ${path}: ${pluralize(result.movedTorrents, "torrent")} moved. The client is checking their files.`
        : `New downloads now go to ${path}.`);
      onOpenChange(false);
    } catch (failure) {
      setProblem(errorMessage(failure));
    } finally {
      setMoving(null);
    }
  };

  const incomplete = plan !== null && plan.blocked === null && !plan.canApply;

  return (
    <Dialog isOpen={open} onOpenChange={onOpenChange} title="Move the download folder" size="lg">
      <div className="flex min-w-0 flex-col gap-4">
        <p className="text-body-sm text-muted">
          <strong className="text-foreground">Librarian does not copy your downloads.</strong> Copy the download folder to the new place yourself first, keeping file dates (for
          example <code className="font-mono">rsync -rt</code>), then check it here. Torrents keep seeding from the old folder until they are moved.
        </p>
        <p className="text-label-sm text-muted">Now at <span className="font-mono text-foreground [overflow-wrap:anywhere]">{current || "the default folder"}</span></p>
        <form.AppForm>
          <form.AppField name="path">{() => <StorageLocationField required folderLabel="Folder in the share" allowWholeShare localPlaceholder="/data/downloads" />}</form.AppField>
          <form.Subscribe selector={(state) => state.values.path}>
            {(path) => (
              <div className="flex min-w-0 flex-col gap-3">
                <div className="flex flex-wrap items-center gap-3">
                  <Button variant="secondary" isPending={checking} isDisabled={!path.trim() || moving !== null} onPress={() => void check(path.trim())}>Check the files</Button>
                  {plan && plan.newPath !== path.trim() && <p className="text-label-sm text-muted">The location changed since this was checked.</p>}
                </div>
                {problem && <p role="alert" className="text-body-sm text-danger [overflow-wrap:anywhere]">{problem}</p>}
                {plan && (
                  <Surface material="inset" padding="tile" radius="tile" className="flex min-w-0 flex-col gap-3">
                    {plan.blocked ? <p role="alert" className="text-body-sm text-danger">{plan.blocked}</p> : (
                      <>
                        <p className="text-body-sm text-foreground">
                          {plan.torrentCount === 0 ? "No torrents are in the download folder, so nothing needs moving." : `${plan.matched.toLocaleString()} of ${files(plan.fileCount)} of ${pluralize(plan.torrentCount, "torrent")} found at the new folder with the same size.`}
                        </p>
                        <ul className="flex flex-col gap-1 text-body-sm text-muted">
                          {plan.missing > 0 && <li><strong className="text-danger">{files(plan.missing)} missing</strong> from the new folder.</li>}
                          {plan.changed > 0 && <li><strong className="text-danger">{files(plan.changed)} a different size</strong>: the copy may not have finished.</li>}
                        </ul>
                        <PlanList title="Missing" items={plan.missingFiles} total={plan.missing} />
                        <PlanList title="Different size" items={plan.changedFiles} total={plan.changed} />
                      </>
                    )}
                  </Surface>
                )}
                {plan && !plan.blocked && (
                  <div className="flex min-w-0 flex-col gap-3">
                    {incomplete && (
                      <label className="flex items-start gap-3 text-body-sm text-muted">
                        <GlassSwitch checked={anyway} onChange={setAnyway} ariaLabel="Move even though files are missing" />
                        <span>Move anyway. The client downloads what is missing again.</span>
                      </label>
                    )}
                    <div className="flex flex-wrap items-center gap-3">
                      <Button variant="primary" isPending={moving === "all"} isDisabled={moving !== null || (incomplete && !anyway) || plan.newPath !== path.trim()} onPress={() => void apply(path.trim(), true)}>Move downloads</Button>
                      <Button variant="secondary" isPending={moving === "new"} isDisabled={moving !== null || plan.newPath !== path.trim()} onPress={() => void apply(path.trim(), false)}>New downloads only</Button>
                      <p className="min-w-0 text-label-sm text-muted">The torrent client restarts briefly. The old files are not deleted.</p>
                    </div>
                  </div>
                )}
              </div>
            )}
          </form.Subscribe>
        </form.AppForm>
      </div>
    </Dialog>
  );
}

function PlanList({ title, items, total }: { title: string; items: string[]; total: number }) {
  if (items.length === 0) return null;
  return (
    <div className="flex min-w-0 flex-col gap-1">
      <p className="text-label-sm text-muted">{title}{total > items.length ? ` (first ${items.length} of ${total.toLocaleString()})` : ""}</p>
      <ul className="max-h-40 overflow-y-auto font-mono text-label-sm text-foreground">
        {items.map((item) => <li key={item} className="[overflow-wrap:anywhere]">{item}</li>)}
      </ul>
    </div>
  );
}
