import { useMutation, useQuery } from "@apollo/client/react";
import { toast } from "@heroui/react";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { Button, Dialog, FieldGroup, FormSelectField, FormTextField } from "@/components/ui";
import { AddTorrentDocument, NavLibrariesDocument } from "@/graphql/generated/graphql";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

const schema = z.object({
  source: z.string().trim().min(1, "Paste a magnet link or a .torrent URL").refine((value) => value.startsWith("magnet:") || /^https?:\/\//.test(value), "Must be a magnet link or an http(s) URL"),
  libraryId: z.string().nullable(),
});
type Values = z.infer<typeof schema>;

interface AddTorrentDialogProps {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  onAdded?: () => void;
}

export function AddTorrentDialog({ isOpen, onOpenChange, onAdded }: AddTorrentDialogProps) {
  const libraries = useQuery(NavLibrariesDocument, { skip: !isOpen });
  const [addTorrent, { loading }] = useMutation(AddTorrentDocument);
  const form = useForm<Values>({ resolver: zodResolver(schema), defaultValues: { source: "", libraryId: null } });

  const submit = form.handleSubmit(async (values) => {
    try {
      const isMagnet = values.source.startsWith("magnet:");
      const { data } = await addTorrent({ variables: { input: { magnet: isMagnet ? values.source : null, url: isMagnet ? null : values.source, libraryId: values.libraryId } } });
      const result = assertSuccess(data?.addTorrent, "Could not add the torrent");
      toast.success(`Added ${result.torrent?.name ?? "torrent"}`);
      form.reset();
      onAdded?.();
      onOpenChange(false);
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  });

  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      title="Add torrent"
      size="md"
      footer={
        <>
          <Button variant="ghost" onPress={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button variant="primary" onPress={() => void submit()} isPending={loading}>
            Add
          </Button>
        </>
      }
    >
      <form onSubmit={submit} noValidate>
        <FieldGroup>
          <FormTextField control={form.control} name="source" label="Magnet link or .torrent URL" placeholder="magnet:?xt=urn:btih:…" mono autoFocus isRequired />
          <FormSelectField
            control={form.control}
            name="libraryId"
            label="Import into"
            placeholder="Decide by matching"
            description="Completed files are matched against this library; leave empty to consider every library."
            options={(libraries.data?.libraries.edges ?? []).map(({ node }) => ({ key: node.id, label: node.name, description: node.libraryType }))}
          />
        </FieldGroup>
      </form>
    </Dialog>
  );
}
