import { renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { EntityTorrentDeleteDocument, EntityTorrentUpdateDocument, PauseTorrentDocument, ResumeTorrentDocument, ProcessSourceDocument } from "@/graphql/generated/graphql";
import type { ActionContext } from "glass-ux/actions";
import type { DownloadRow } from "../useDownloads";
import { useTorrentActions } from "../useTorrentActions";
const calls=vi.hoisted(()=>({pause:vi.fn(),resume:vi.fn(),remove:vi.fn(),update:vi.fn(),process:vi.fn(),changed:vi.fn()}));
vi.mock("@apollo/client/react",()=>({useMutation:(doc:unknown)=>[
  doc===PauseTorrentDocument?calls.pause:doc===ResumeTorrentDocument?calls.resume:doc===EntityTorrentDeleteDocument?calls.remove:doc===EntityTorrentUpdateDocument?calls.update:doc===ProcessSourceDocument?calls.process:()=>{throw new Error("Unexpected mutation");},
]}));
const row=(id:number,state="seeding",loaded=true,deleteFilesOnRemove=false)=>({id:String(id),record:{id:`entity-${id}`,name:`Torrent ${id}`,state,progress:1,deleteFilesOnRemove},live:loaded?{id,state,progress:1}:null}) as unknown as DownloadRow;
const setup=()=>renderHook(()=>useTorrentActions({onLink:()=>{},onChanged:calls.changed},true)).result.current.actions;
const actionContext={} as ActionContext;
beforeEach(()=>{vi.clearAllMocks();calls.pause.mockResolvedValue({data:{pauseTorrent:{success:true}}});calls.resume.mockResolvedValue({data:{resumeTorrent:{success:true}}});calls.remove.mockResolvedValue({data:{deleteTorrent:{success:true}}});calls.update.mockResolvedValue({data:{updateTorrent:{success:true}}});calls.process.mockResolvedValue({data:{processSource:{success:true}}});});
describe("shared torrent bulk actions",()=>{
 it("continues after a failed item and reports partial failure while refreshing once",async()=>{
  calls.pause.mockRejectedValueOnce(new Error("Receiver unavailable"));const pause=setup().find(a=>a.id==="pause")!;
  await expect(pause.run!([row(1),row(2)],actionContext)).rejects.toThrow("Pause succeeded for 1 of 2 downloads");
  expect(calls.pause).toHaveBeenCalledTimes(2);expect(calls.changed).toHaveBeenCalledOnce();
 });
 it("removes a stored download that is not loaded in the client, by entity id",async()=>{
  const remove=setup().find(a=>a.id==="remove")!;const unloaded=row(2,"error",false);
  expect(remove.disabledReason).toBeUndefined();
  await remove.run!([unloaded],actionContext);
  expect(calls.remove).toHaveBeenCalledExactlyOnceWith({variables:{id:"entity-2"}});expect(calls.update).not.toHaveBeenCalled();
 });
 it("sets deleteFilesOnRemove before deleting when files are removed too",async()=>{
  const order:string[]=[];calls.update.mockImplementation(async()=>{order.push("update");return {data:{updateTorrent:{success:true}}};});calls.remove.mockImplementation(async()=>{order.push("delete");return {data:{deleteTorrent:{success:true}}};});
  await setup().find(a=>a.id==="remove-files")!.run!([row(1)],actionContext);
  expect(order).toEqual(["update","delete"]);
  expect(calls.update).toHaveBeenCalledWith({variables:{id:"entity-1",input:{deleteFilesOnRemove:true}}});
  expect(calls.remove).toHaveBeenCalledWith({variables:{id:"entity-1"}});
 });
 it("surfaces a refused delete and puts the file flag back so a later plain remove cannot delete files",async()=>{
  calls.remove.mockResolvedValueOnce({data:{deleteTorrent:{success:false,error:"Cannot delete files: completed-file post-processing is still running."}}});
  await expect(setup().find(a=>a.id==="remove-files")!.run!([row(1)],actionContext)).rejects.toThrow(/post-processing is still running/);
  expect(calls.update).toHaveBeenNthCalledWith(1,{variables:{id:"entity-1",input:{deleteFilesOnRemove:true}}});
  expect(calls.update).toHaveBeenNthCalledWith(2,{variables:{id:"entity-1",input:{deleteFilesOnRemove:false}}});
 });
 it("clears a stale file flag before a plain remove so files stay on disk",async()=>{
  await setup().find(a=>a.id==="remove")!.run!([row(1,"seeding",true,true)],actionContext);
  expect(calls.update).toHaveBeenCalledExactlyOnceWith({variables:{id:"entity-1",input:{deleteFilesOnRemove:false}}});
  expect(calls.remove).toHaveBeenCalledOnce();
 });
 it("pauses only applicable targets and accurately counts the success message",async()=>{
  const pause=setup().find(a=>a.id==="pause")!;const rows=[row(1),row(2,"paused")];await pause.run!(rows,actionContext);
  expect(calls.pause).toHaveBeenCalledExactlyOnceWith({variables:{id:1}});expect(pause.successMessage!(rows)).toBe("Paused");
 });
 it("imports persisted identities and leaves destructive file removal separately confirmed",async()=>{
  const actions=setup(),imports=actions.find(a=>a.id==="import")!,deletes=actions.find(a=>a.id==="remove-files")!;
  await imports.run!([row(1),row(2)],actionContext);expect(calls.process).toHaveBeenCalledWith({variables:{sourceId:"entity-1",sourceType:"torrent"}});
  expect(deletes.confirm!([row(1),row(2)])).toMatchObject({tone:"danger",confirmLabel:"Delete files"});
 });
});
