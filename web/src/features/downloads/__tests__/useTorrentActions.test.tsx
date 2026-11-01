import { renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { PauseTorrentDocument, ResumeTorrentDocument, RemoveTorrentDocument, ProcessSourceDocument } from "@/graphql/generated/graphql";
import type { ActionContext } from "glass-ux/actions";
import type { DownloadRow } from "../useDownloads";
import { useTorrentActions } from "../useTorrentActions";
const calls=vi.hoisted(()=>({pause:vi.fn(),resume:vi.fn(),remove:vi.fn(),process:vi.fn(),changed:vi.fn()}));
vi.mock("@apollo/client/react",()=>({useMutation:(doc:unknown)=>[
  doc===PauseTorrentDocument?calls.pause:doc===ResumeTorrentDocument?calls.resume:doc===RemoveTorrentDocument?calls.remove:doc===ProcessSourceDocument?calls.process:()=>{throw new Error("Unexpected mutation");},
]}));
const row=(id:number,state="seeding",loaded=true)=>({id:String(id),record:{id:String(id),name:`Torrent ${id}`,state,progress:1},live:loaded?{id,state,progress:1}:null}) as unknown as DownloadRow;
const setup=()=>renderHook(()=>useTorrentActions({onLink:()=>{},onChanged:calls.changed},true)).result.current.actions;
beforeEach(()=>{vi.clearAllMocks();calls.pause.mockResolvedValue({data:{pauseTorrent:{success:true}}});calls.resume.mockResolvedValue({data:{resumeTorrent:{success:true}}});calls.remove.mockResolvedValue({data:{removeTorrent:{success:true}}});calls.process.mockResolvedValue({data:{processSource:{success:true}}});});
describe("shared torrent bulk actions",()=>{
 it("continues after a failed item and reports partial failure while refreshing once",async()=>{
  calls.pause.mockRejectedValueOnce(new Error("Receiver unavailable"));const pause=setup().find(a=>a.id==="pause")!;
  await expect(pause.run!([row(1),row(2)],{} as ActionContext)).rejects.toThrow("Pause succeeded for 1 of 2 downloads");
  expect(calls.pause).toHaveBeenCalledTimes(2);expect(calls.changed).toHaveBeenCalledOnce();
 });
 it("blocks mixed loaded/unloaded deletion instead of silently skipping selected rows",()=>{
  const remove=setup().find(a=>a.id==="remove")!;expect(remove.disabledReason!([row(1),row(2,"seeding",false)])).toMatch(/not loaded/);
 });
 it("pauses only applicable targets and accurately counts the success message",async()=>{
  const pause=setup().find(a=>a.id==="pause")!;const rows=[row(1),row(2,"paused")];await pause.run!(rows,{} as ActionContext);
  expect(calls.pause).toHaveBeenCalledExactlyOnceWith({variables:{id:1}});expect(pause.successMessage!(rows)).toBe("Paused");
 });
 it("imports persisted identities and leaves destructive file removal separately confirmed",async()=>{
  const actions=setup(),imports=actions.find(a=>a.id==="import")!,deletes=actions.find(a=>a.id==="remove-files")!;
  await imports.run!([row(1),row(2)],{} as ActionContext);expect(calls.process).toHaveBeenCalledWith({variables:{sourceId:"1",sourceType:"torrent"}});
  expect(deletes.confirm!([row(1),row(2)])).toMatchObject({tone:"danger",confirmLabel:"Delete files"});
 });
});
