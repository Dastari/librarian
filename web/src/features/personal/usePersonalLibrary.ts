import { useMutation } from "@apollo/client/react";
import {
  EntityHomeShelfCreateDocument, EntityHomeShelfListDocument, EntityHomeShelfUpdateDocument,
  EntityLibraryPreferenceCreateDocument, EntityLibraryPreferenceListDocument, EntityLibraryPreferenceUpdateDocument,
  EntityPersonalListCreateDocument, EntityPersonalListListDocument,
  type PersonalListKind,
} from "@/graphql/generated/graphql";
import { useCompleteConnection } from "@/features/libraries/browser/useCompleteConnection";
import { useSession } from "@/lib/auth/useSession";
import { assertSuccess } from "@/lib/graphql/errors";

export const DEFAULT_SHELVES = [
  { key: "continue", label: "Continue watching" }, { key: "libraries", label: "Libraries" },
  { key: "movies", label: "Recently added movies" }, { key: "shows", label: "Recently added shows" },
  { key: "albums", label: "New music" }, { key: "audiobooks", label: "New audiobooks" },
  { key: "guide", label: "TV guide" }, { key: "releases", label: "Movie releases" },
] as const;

export function usePersonalLists() {
  const { user } = useSession();
  const list = useCompleteConnection(EntityPersonalListListDocument, { where: { userId: { eq: user?.id ?? "" } }, orderBy: [{ name: "ASC" }] }, (data) => data.personalLists, { skip: !user });
  const [create] = useMutation(EntityPersonalListCreateDocument);
  const createList = async (name: string, kind: PersonalListKind = "MANUAL", savedViewId?: string) => {
    if (!user) throw new Error("Sign in to save a list");
    const { data } = await create({ variables: { input: { userId: user.id, name: name.trim(), kind, savedViewId: savedViewId ?? null, description: null } } });
    assertSuccess(data?.createPersonalList, "Could not create list");
    const node = data?.createPersonalList?.personalList;
    if (!node) throw new Error("The list was not returned");
    await list.refetch();
    return node;
  };
  return { ...list, createList };
}

export function useLibraryPreferences() {
  const { user } = useSession();
  const list = useCompleteConnection(EntityLibraryPreferenceListDocument, { where: { userId: { eq: user?.id ?? "" } }, orderBy: [{ position: "ASC" }, { id: "ASC" }] }, (data) => data.libraryPreferences, { skip: !user });
  const [create] = useMutation(EntityLibraryPreferenceCreateDocument);
  const [update] = useMutation(EntityLibraryPreferenceUpdateDocument);
  const toggle = async (libraryId: string) => {
    if (!user) return;
    const previous = list.rows.find((row) => row.libraryId === libraryId);
    if (previous) {
      const { data } = await update({ variables: { id: previous.id, input: { favourite: !previous.favourite } } });
      assertSuccess(data?.updateLibraryPreference, "Could not save favourite");
    } else {
      const { data } = await create({ variables: { input: { userId: user.id, libraryId, favourite: true, position: list.rows.length } } });
      assertSuccess(data?.createLibraryPreference, "Could not save favourite");
    }
    await list.refetch();
  };
  return { ...list, toggle };
}

export function useHomeShelves() {
  const { user } = useSession();
  const list = useCompleteConnection(EntityHomeShelfListDocument, { where: { userId: { eq: user?.id ?? "" } }, orderBy: [{ position: "ASC" }, { id: "ASC" }] }, (data) => data.homeShelves, { skip: !user });
  const [create] = useMutation(EntityHomeShelfCreateDocument);
  const [update] = useMutation(EntityHomeShelfUpdateDocument);
  const shelves = [
    ...DEFAULT_SHELVES.map((item, index) => {
      const saved = list.rows.find((row) => row.shelfKey === item.key);
      return { ...item, id: saved?.id, position: saved?.position ?? index * 10, visible: saved?.visible ?? true, listId: saved?.listId, libraryId: saved?.libraryId };
    }),
    ...list.rows.filter((row) => !DEFAULT_SHELVES.some((item) => item.key === row.shelfKey)).map((row) => ({ key: row.shelfKey, label: "Personal list", id: row.id, position: row.position, visible: row.visible, listId: row.listId, libraryId: row.libraryId })),
  ].sort((a, b) => a.position - b.position || a.key.localeCompare(b.key));
  const save = async (key: string, values: { position: number; visible: boolean; listId?: string | null; libraryId?: string | null }) => {
    if (!user) throw new Error("Sign in to customize Home");
    const previous = list.rows.find((row) => row.shelfKey === key);
    if (previous) {
      const { data } = await update({ variables: { id: previous.id, input: { position: values.position, visible: values.visible, listId: values.listId, libraryId: values.libraryId } } });
      assertSuccess(data?.updateHomeShelf, "Could not update Home");
    } else {
      const { data } = await create({ variables: { input: { userId: user.id, shelfKey: key, position: values.position, visible: values.visible, listId: values.listId, libraryId: values.libraryId } } });
      assertSuccess(data?.createHomeShelf, "Could not update Home");
    }
    await list.refetch();
  };
  return { ...list, shelves, save };
}
