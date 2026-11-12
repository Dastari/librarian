import { useApolloClient, useMutation } from "@apollo/client/react";
import { formBehaviour, useAppForm } from "glass-ux/form";
import { Surface } from "glass-ux/surface";
import { toast } from "glass-ux/toast";
import { useState } from "react";

import { Button } from "@/components/ui";
import { GlassSwitch } from "@/components/ui/GlassControls";
import {
  LibraryMovePlanDocument,
  MoveLibraryDocument,
  ScanLibraryDocument,
  type LibraryFieldsFragment,
  type LibraryMovePlanQuery,
} from "@/graphql/generated/graphql";
import { errorMessage } from "@/lib/graphql/errors";

import { LibraryLocationField } from "./LibraryLocationField";

type Plan = LibraryMovePlanQuery["libraryMovePlan"];

/** `n` of something, in words. */
const files = (count: number) => `${count.toLocaleString()} ${count === 1 ? "file" : "files"}`;

/**
 * Move a library to a folder its files were copied to, keeping every record: the files keep their
 * matches, progress and playlists, and nothing is re-matched. The person copies the files first;
 * this compares, then switches the library over and runs a scan to verify.
 */
export function MoveLibraryPanel({ library, onMoved }: { library: Pick<LibraryFieldsFragment, "id" | "name" | "path" | "scanning">; onMoved?: () => void }) {
  const client = useApolloClient();
  const [applyMove] = useMutation(MoveLibraryDocument);
  const [scan] = useMutation(ScanLibraryDocument);
  const [plan, setPlan] = useState<Plan | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);
  const [moving, setMoving] = useState(false);
  const [anyway, setAnyway] = useState(false);

  const form = useAppForm({ ...formBehaviour, defaultValues: { path: "" } });
  const forget = () => { setPlan(null); setProblem(null); setAnyway(false); };

  const check = async (path: string) => {
    forget();
    setChecking(true);
    try {
      const { data, error } = await client.query({ query: LibraryMovePlanDocument, variables: { libraryId: library.id, newPath: path }, fetchPolicy: "network-only" });
      if (!data) throw error ?? new Error("The folder could not be checked");
      setPlan(data.libraryMovePlan);
    } catch (failure) {
      setProblem(errorMessage(failure));
    } finally {
      setChecking(false);
    }
  };

  const move = async (path: string) => {
    if (!plan) return;
    setMoving(true);
    try {
      const { data } = await applyMove({ variables: { input: { libraryId: library.id, newPath: path, allowMissing: anyway } } });
      const result = data?.moveLibrary;
      if (!result?.success) throw new Error(result?.error ?? "The library could not be moved");
      toast.success(`${library.name} now reads from ${plan.newPath}. ${files(result.movedFiles)} moved.`);
      // The verifying scan: it should find every file where it now expects it.
      const started = (await scan({ variables: { id: library.id } })).data?.scanLibrary;
      if (started?.success) toast.success("A scan has started to verify the move.");
      forget();
      form.reset({ path: "" });
      onMoved?.();
    } catch (failure) {
      setProblem(errorMessage(failure));
    } finally {
      setMoving(false);
    }
  };

  const incomplete = plan !== null && plan.blocked === null && !plan.canApply;

  return (
    <div className="flex min-w-0 flex-col gap-4">
      <div className="flex flex-col gap-2">
        <p className="text-body-sm text-muted">
          Moving changes where {library.name} reads its files, and keeps everything Librarian knows about them. <strong className="text-foreground">Librarian does not copy your files.</strong> Copy
          this library&rsquo;s folder to the new location yourself first, keeping file dates (for example <code className="font-mono">rsync -a</code> or <code className="font-mono">cp -a</code>), then check it here.
        </p>
        <p className="text-label-sm text-muted">Now at <span className="font-mono text-foreground [overflow-wrap:anywhere]">{library.path}</span></p>
      </div>

      <form.AppForm>
        <form.AppField name="path">{() => <LibraryLocationField required />}</form.AppField>
        <form.Subscribe selector={(state) => state.values.path}>
          {(path) => (
            <div className="flex min-w-0 flex-col gap-3">
              <div className="flex flex-wrap items-center gap-3">
                <Button variant="secondary" isPending={checking} isDisabled={!path.trim() || moving} onPress={() => void check(path.trim())}>Check the files</Button>
                {plan && plan.newPath !== path.trim() && <p className="text-label-sm text-muted">The location changed since this was checked.</p>}
              </div>

              {problem && <p role="alert" className="text-body-sm text-danger [overflow-wrap:anywhere]">{problem}</p>}

              {plan && (
                <Surface material="inset" padding="tile" radius="tile" className="flex min-w-0 flex-col gap-3">
                  {plan.blocked ? (
                    <p role="alert" className="text-body-sm text-danger">{plan.blocked}</p>
                  ) : (
                    <>
                      <p className="text-body-sm text-foreground">
                        {plan.matched.toLocaleString()} of {files(plan.fileCount)} found at <span className="font-mono [overflow-wrap:anywhere]">{plan.newPath}</span> with the same size.
                      </p>
                      <ul className="flex flex-col gap-1 text-body-sm text-muted">
                        {plan.missing > 0 && <li><strong className="text-danger">{files(plan.missing)} missing</strong> from the new location.</li>}
                        {plan.changed > 0 && <li><strong className="text-danger">{files(plan.changed)} a different size</strong>: the copy may not have finished.</li>}
                        {plan.extra > 0 && <li>{files(plan.extra)} on the new location that the library does not have yet. A scan will add them.</li>}
                        {plan.canApply && plan.extra === 0 && <li>Every file is there. Nothing else is on the new location.</li>}
                      </ul>
                      <PlanList title="Missing" items={plan.missingFiles} total={plan.missing} />
                      <PlanList title="Different size" items={plan.changedFiles} total={plan.changed} />
                      <PlanList title="Not in the library" items={plan.extraFiles} total={plan.extra} />
                    </>
                  )}
                </Surface>
              )}

              {plan && !plan.blocked && (
                <div className="flex min-w-0 flex-col gap-3">
                  {incomplete && (
                    <label className="flex items-start gap-3 text-body-sm text-muted">
                      <GlassSwitch checked={anyway} onChange={setAnyway} ariaLabel="Move even though files are missing" />
                      <span>Move anyway. Files that are not on the new location will show as missing after the next scan.</span>
                    </label>
                  )}
                  <div className="flex flex-wrap items-center gap-3">
                    <Button variant="primary" isPending={moving} isDisabled={(incomplete && !anyway) || plan.newPath !== path.trim() || library.scanning} onPress={() => void move(path.trim())}>
                      Move library
                    </Button>
                    <p className="min-w-0 text-label-sm text-muted">
                      {library.scanning ? "A scan is running; wait for it to finish." : moving ? "Moving. Keep this page open until it finishes." : "Scans and organizing pause for this library while it moves. A scan then verifies the result."}
                    </p>
                  </div>
                </div>
              )}
            </div>
          )}
        </form.Subscribe>
      </form.AppForm>
    </div>
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
