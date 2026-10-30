import { act, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useReducer } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { renderWithProviders } from "@/test";
import { MetadataSettings } from "../MetadataSettings";

const state = vi.hoisted(() => ({
  metadata: { values: new Map<string, string>(), loading: false, save: vi.fn() },
  llm: { values: new Map<string, string>(), loading: false, save: vi.fn() },
  testTmdb: vi.fn(), testOllama: vi.fn(), testParser: vi.fn(),
}));
vi.mock("../useAppSettings", async (original) => ({
  ...await original<typeof import("../useAppSettings")>(),
  useAppSettings: (category: "metadata" | "llm") => state[category],
}));
vi.mock("@apollo/client/react", async (original) => {
  const docs = await import("@/graphql/generated/graphql");
  return {
    ...await original<typeof import("@apollo/client/react")>(),
    useMutation: (document: unknown) => [document === docs.TestTmdbConnectionDocument ? state.testTmdb : document === docs.TestOllamaConnectionDocument ? state.testOllama : state.testParser, { loading: false }],
  };
});
let refresh: () => void;
function Harness() {
  const [, rerender] = useReducer((value: number) => value + 1, 0);
  refresh = rerender;
  return <MetadataSettings />;
}
const mount = () => renderWithProviders(<Harness />, { router: false });
const input = (label: string) => screen.getAllByLabelText(label).find((node) => node.tagName === "INPUT")!;
const edit = async (label: string, value: string) => {
  const field = input(label);
  await userEvent.clear(field);
  if (value) await userEvent.type(field, value);
  await userEvent.tab();
  return field;
};
const save = () => screen.getByRole("button", { name: "Save" });

beforeEach(() => {
  vi.clearAllMocks();
  state.metadata.values = new Map([["metadata.tmdb_api_key", "stored-key"], ["metadata.cache_days.tmdb", "14"], ["metadata.tmdb_enabled", "false"]]);
  state.llm.values = new Map([["llm.ollama_url", "http://ollama:11434"], ["llm.ollama_model", "stored-model"]]);
  state.metadata.loading = state.llm.loading = false;
  state.metadata.save.mockReset().mockResolvedValue(undefined);
  state.llm.save.mockReset().mockResolvedValue(undefined);
  state.testTmdb.mockReset().mockResolvedValue({ data: { testTmdbConnection: { success: true, message: "TMDB connected" } } });
  state.testOllama.mockReset().mockResolvedValue({ data: { testOllamaConnection: { success: true, models: ["model-a"] } } });
  state.testParser.mockReset().mockResolvedValue({ data: undefined });
});

describe("MetadataSettings bound form", () => {
  it("retains stored values on unrelated rerenders and restores them on discard", async () => {
    await mount();
    expect(input("API key")).toHaveValue("stored-key");
    expect(screen.getByRole("switch", { name: "Use TMDB for movies and collections" })).not.toBeChecked();
    expect(save()).toBeDisabled();
    act(() => refresh());
    expect(input("API key")).toHaveValue("stored-key");
    expect(input("Model")).toHaveValue("stored-model");
    await edit("Model", "draft-model");
    await userEvent.click(screen.getByRole("button", { name: "Discard" }));
    expect(input("Model")).toHaveValue("stored-model");
    expect(save()).toBeDisabled();
  });

  it("preserves dirty fields while independent category snapshots update untouched values", async () => {
    await mount();
    await edit("API key", "draft-key");
    act(() => {
      state.metadata.values = new Map([...state.metadata.values, ["metadata.tmdb_api_key", "new-key"]]);
      state.llm.values = new Map([...state.llm.values, ["llm.ollama_model", "new-model"]]);
      refresh();
    });
    expect(input("API key")).toHaveValue("draft-key");
    expect(input("Model")).toHaveValue("new-model");
    act(() => refresh());
    expect(input("API key")).toHaveValue("draft-key");
    await userEvent.click(screen.getByRole("button", { name: "Discard" }));
    expect(input("API key")).toHaveValue("new-key");
    expect(input("Model")).toHaveValue("new-model");
    expect(save()).toBeDisabled();
  });

  it("trims text, keeps optional keys nullable and saves zero cache days", async () => {
    await mount();
    await edit("API key", "  ");
    await edit("Model", "  ");
    await edit("Ollama URL", " http://local:11434 ");
    const cache = screen.getAllByLabelText("Cache results for (days)").filter((node) => node.tagName === "INPUT")[0]!;
    await userEvent.clear(cache);
    await userEvent.type(cache, "0");
    await userEvent.tab();
    await userEvent.click(save());
    await screen.findByText("Settings saved");
    expect(state.metadata.save).toHaveBeenCalledWith(expect.objectContaining({ "metadata.tmdb_api_key": null, "metadata.tvdb_api_key": null, "metadata.cache_days.tmdb": 0 }));
    expect(state.llm.save).toHaveBeenCalledWith(expect.objectContaining({ "llm.ollama_model": null, "llm.ollama_url": "http://local:11434" }));
    act(() => refresh());
    expect(input("API key")).toHaveValue("");
    expect(input("Ollama URL")).toHaveValue("http://local:11434");
    expect(screen.getByRole("button", { name: "Discard" })).toBeDisabled();
  });

  it("rejects a cleared number rather than silently saving zero", async () => {
    await mount();
    const timeout = await edit("Timeout (seconds)", "");
    await userEvent.click(save());
    expect(await screen.findByText("Enter a number")).toBeInTheDocument();
    expect(timeout).toHaveValue("");
    await waitFor(() => expect(timeout).toHaveFocus());
    expect(state.metadata.save).not.toHaveBeenCalled();
    expect(state.llm.save).not.toHaveBeenCalled();
  });

  it("keeps submission pending across live snapshots and preserves the draft on second-category failure", async () => {
    let resolve!: () => void;
    state.metadata.save.mockImplementationOnce(() => new Promise<void>((done) => { resolve = done; }));
    state.llm.save.mockRejectedValueOnce(new Error("LLM settings unavailable"));
    await mount();
    await edit("Model", "draft-model");
    const submit = save();
    await userEvent.click(submit);
    await waitFor(() => expect(state.metadata.save).toHaveBeenCalled());
    expect(input("Model")).toBeDisabled();
    expect(screen.getByRole("button", { name: "Discard" })).toBeDisabled();
    act(() => {
      state.llm.values = new Map([...state.llm.values, ["llm.ollama_model", "remote-model"]]);
      refresh();
    });
    expect(input("Model")).toHaveValue("draft-model");
    expect(input("Model")).toBeDisabled();
    await act(async () => resolve());
    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("LLM settings unavailable"));
    expect(input("Model")).toHaveValue("draft-model");
    await userEvent.click(screen.getByRole("button", { name: "Discard" }));
    expect(input("Model")).toHaveValue("remote-model");
    // Glass holds the failed-save feedback before returning the action to idle.
    await waitFor(() => expect(submit).toBeDisabled(), { timeout: 3000 });
  });

  it("tests connections with current draft values and parses samples independently of form validation", async () => {
    await mount();
    await edit("API key", "draft-key");
    await edit("Ollama URL", "http://draft:11434");
    const tests = screen.getAllByRole("button", { name: "Test" });
    await userEvent.click(tests[0]!);
    await waitFor(() => expect(state.testTmdb).toHaveBeenCalledWith({ variables: { input: { apiKey: "draft-key" } } }));
    await userEvent.click(tests[1]!);
    await waitFor(() => expect(state.testOllama).toHaveBeenCalledWith({ variables: { input: { ollamaUrl: "http://draft:11434" } } }));
    await edit("Timeout (seconds)", "");
    await edit("Sample filename", "Example.2026.mkv");
    await userEvent.click(screen.getByRole("button", { name: "Parse" }));
    expect(state.testParser).toHaveBeenCalledWith({ variables: { input: { filename: "Example.2026.mkv", libraryType: "movies" } } });
    expect(state.metadata.save).not.toHaveBeenCalled();
    expect(state.llm.save).not.toHaveBeenCalled();
  });
});
