import type { ActionSet } from "glass-ux/actions";
import { EntityIdentity } from "glass-ux/entity";
import { FileIcon } from "glass-ux/file-icon";

import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { fileName } from "@/lib/entities/linked";
import { formatBytes } from "@/lib/format";

import { fileFormat, isFileLinked, type FileEntity } from "./useFileActions";

/**
 * A media file wherever it is named: its type's glyph, its name, whether it belongs to a title
 * and what it is. The table, the dialog and the title pages all draw this one contract.
 */
export function FileIdentity({ file, actions, className, onOpen, tier = "row" }: {
  file: FileEntity;
  tier?: "row" | "header";
  actions?: ActionSet<FileEntity>;
  className?: string;
  onOpen?: () => void;
}) {
  const linked = isFileLinked(file);
  const name = fileName(file.path);
  return (
    <EntityIdentity
      tier={tier}
      heading="h2"
      className={className}
      kind={ENTITY_KINDS.mediaFile.noun}
      name={name}
      icon={<FileIcon name={name} mime={file.contentType} />}
      state={{ id: linked ? "linked" : "unmatched", label: linked ? "Linked" : "Unmatched", tone: linked ? "success" : "warning" }}
      secondary={file.relativePath ?? file.path}
      facts={[fileFormat(file), formatBytes(file.size)].filter(Boolean)}
      target={file}
      actions={actions}
      onOpen={onOpen}
    />
  );
}
