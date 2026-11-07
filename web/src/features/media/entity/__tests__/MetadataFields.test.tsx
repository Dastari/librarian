import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useAppForm } from "glass-ux/form";
import { describe, expect, it } from "vitest";

import { renderWithProviders } from "@/test";

import { MetadataTagsField, MetadataTextareaField, MetadataTextField } from "../MetadataFields";

function Fixture({ locks, seen }: { locks?: string[]; seen: { current?: string[] } }) {
  const form = useAppForm({ defaultValues: { title: "Old", overview: "Told", genres: ["Drama"], ...(locks ? { lockedFields: locks } : {}) } });
  return (
    <form.AppForm>
      <form.AppField name="title">{() => <MetadataTextField label="Title" />}</form.AppField>
      <form.AppField name="overview">{() => <MetadataTextareaField label="Summary" />}</form.AppField>
      <form.AppField name="genres">{() => <MetadataTagsField label="Genres" />}</form.AppField>
      <form.Subscribe selector={(state) => (state.values as { lockedFields?: string[] }).lockedFields}>
        {(current) => { seen.current = current; return null; }}
      </form.Subscribe>
    </form.AppForm>
  );
}

describe("metadata fields", () => {
  it("locks a field as soon as it's edited", async () => {
    const seen: { current?: string[] } = {};
    await renderWithProviders(<Fixture locks={[]} seen={seen} />);
    await userEvent.type(screen.getByLabelText("Title"), "er");
    await waitFor(() => expect(seen.current).toEqual(["title"]));
  });

  it("toggles a lock without changing the value", async () => {
    const seen: { current?: string[] } = {};
    await renderWithProviders(<Fixture locks={["overview"]} seen={seen} />);
    expect(screen.getAllByRole("button", { name: "Unlock this field" })).toHaveLength(1);
    await userEvent.click(screen.getByRole("button", { name: "Unlock this field" }));
    await waitFor(() => expect(seen.current).toEqual([]));
    expect(screen.getByLabelText("Summary")).toHaveValue("Told");
    await userEvent.click(screen.getAllByRole("button", { name: "Lock this field" })[0]!);
    await waitFor(() => expect(seen.current).toEqual(["title"]));
  });

  it("shows no locks on a form that doesn't keep them", async () => {
    const seen: { current?: string[] } = {};
    await renderWithProviders(<Fixture seen={seen} />);
    expect(screen.queryByRole("button", { name: /lock this field/i })).toBeNull();
    await userEvent.type(screen.getByLabelText("Title"), "er");
    expect(screen.getByLabelText("Title")).toHaveValue("Older");
  });
});
