import { useMemo, useState } from "react";
import { Button } from "@heroui/button";
import { Modal, ModalBody, ModalContent, ModalFooter, ModalHeader } from "@heroui/modal";
import { Input } from "@heroui/input";
import { Switch } from "@heroui/switch";
import { Chip } from "@heroui/chip";
import { Image } from "@heroui/image";
import { addToast } from "@heroui/toast";
import { IconSearch, IconStack } from "@tabler/icons-react";
import { DataTable, type DataTableColumn } from "../data-table";
import { sanitizeError } from "../../lib/format";
import { useLazyQuery, useMutation } from "../../lib/graphql/client";
import {
  AddMovieCollectionDocument,
  type AddMovieCollectionMutation,
  SearchMovieCollectionsDocument,
  type SearchMovieCollectionsQuery,
} from "../../lib/graphql/generated/graphql";

export interface AddCollectionModalProps {
  isOpen: boolean;
  onClose: () => void;
  libraryId: string;
  onAdded: () => void;
}

export function AddCollectionModal({
  isOpen,
  onClose,
  libraryId,
  onAdded,
}: AddCollectionModalProps) {
  type CollectionResult = SearchMovieCollectionsQuery["SearchMovieCollections"][number];

  const [query, setQuery] = useState("");
  const [results, setResults] = useState<CollectionResult[]>([]);
  const [selectedCollectionId, setSelectedCollectionId] = useState<number | null>(null);
  const [wantedMissing, setWantedMissing] = useState(true);

  const [searchCollections, { loading: searching }] = useLazyQuery(SearchMovieCollectionsDocument);
  const [addCollection, { loading: adding }] = useMutation<AddMovieCollectionMutation>(
    AddMovieCollectionDocument,
  );

  const selectedCollection = useMemo(
    () =>
      selectedCollectionId != null
        ? results.find((item) => item.CollectionId === selectedCollectionId) ?? null
        : null,
    [results, selectedCollectionId],
  );

  const handleSearch = async () => {
    if (!query.trim()) return;
    try {
      const { data, error } = await searchCollections({
        variables: { Query: query.trim() },
      });
      if (error) {
        addToast({
          title: "Error",
          description: sanitizeError(error),
          color: "danger",
        });
        return;
      }
      setResults(data?.SearchMovieCollections ?? []);
      if (!data?.SearchMovieCollections?.some((c) => c.CollectionId === selectedCollectionId)) {
        setSelectedCollectionId(null);
      }
    } catch (error) {
      addToast({
        title: "Error",
        description: sanitizeError(error),
        color: "danger",
      });
    }
  };

  const handleAdd = async () => {
    if (!selectedCollection) return;
    try {
      const { data, error } = await addCollection({
        variables: {
          LibraryId: libraryId,
          Input: {
            CollectionId: selectedCollection.CollectionId,
            WantedMissing: wantedMissing,
          },
        },
      });

      if (error || !data?.AddMovieCollection.Success) {
        addToast({
          title: "Error",
          description: sanitizeError(
            data?.AddMovieCollection.Error || error || "Failed to add collection",
          ),
          color: "danger",
        });
        return;
      }

      const importedCount = data.AddMovieCollection.ImportedCount;
      const existingCount = data.AddMovieCollection.ExistingCount;
      const wantedUpdatedCount = data.AddMovieCollection.WantedUpdatedCount;

      addToast({
        title: "Collection Imported",
        description: `${importedCount} added, ${existingCount} already in library, ${wantedUpdatedCount} wanted updated`,
        color: "success",
      });
      handleClose();
      onAdded();
    } catch (error) {
      addToast({
        title: "Error",
        description: sanitizeError(error),
        color: "danger",
      });
    }
  };

  const handleClose = () => {
    setQuery("");
    setResults([]);
    setSelectedCollectionId(null);
    setWantedMissing(true);
    onClose();
  };

  const columns: DataTableColumn<CollectionResult>[] = [
    {
      key: "name",
      label: "Collection",
      sortable: true,
      render: (collection) => (
        <div className="flex items-center gap-3">
          {collection.PosterUrl ? (
            <Image
              src={collection.PosterUrl}
              alt={collection.Name}
              className="w-10 h-14 object-cover rounded"
              loading="lazy"
            />
          ) : (
            <div className="w-10 h-14 bg-default-200 rounded flex items-center justify-center">
              <IconStack size={18} className="text-purple-400" />
            </div>
          )}
          <div>
            <p className="font-medium">{collection.Name}</p>
            <p className="text-xs text-default-500">TMDB #{collection.CollectionId}</p>
          </div>
        </div>
      ),
    },
    {
      key: "overview",
      label: "Overview",
      sortable: false,
      render: (collection) => (
        <p className="text-sm text-default-500 line-clamp-2">
          {collection.Overview || "No description available"}
        </p>
      ),
    },
  ];

  const selectedKeys = useMemo<Set<string>>(
    () => (selectedCollectionId != null ? new Set([String(selectedCollectionId)]) : new Set()),
    [selectedCollectionId],
  );

  return (
    <Modal isOpen={isOpen} onClose={handleClose} size="4xl">
      <ModalContent>
        <ModalHeader>Add Collection</ModalHeader>
        <ModalBody className="gap-4">
          <div className="flex items-center gap-2">
            <Input
              label="Search Collections"
              labelPlacement="inside"
              variant="flat"
              placeholder="Search TMDB collections..."
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") void handleSearch();
              }}
              endContent={<IconSearch size={16} className="text-default-400" />}
            />
            <Button color="primary" onPress={() => void handleSearch()} isLoading={searching}>
              Search
            </Button>
          </div>

          <DataTable
            stateKey="add-collection-modal-search"
            data={results}
            columns={columns}
            getRowKey={(collection) => String(collection.CollectionId)}
            ariaLabel="Collection search results"
            searchPlaceholder="Filter results..."
            isLoading={searching}
            selectionMode="single"
            selectedKeys={selectedKeys}
            onSelectionChange={(keys) => {
              const first = Array.from(keys)[0];
              if (first == null) {
                setSelectedCollectionId(null);
                return;
              }
              const parsed = Number(first);
              setSelectedCollectionId(Number.isFinite(parsed) ? parsed : null);
            }}
            emptyContent={
              <div className="py-10 text-center text-default-500">
                Search TMDB collections to import into this library.
              </div>
            }
            showItemCount
          />

          <div className="flex items-center justify-between p-3 bg-content2 rounded-lg">
            <div>
              <p className="font-medium">Mark Missing As Wanted</p>
              <p className="text-xs text-default-500">
                Set imported missing movies to wanted so download automation can pick them up.
              </p>
            </div>
            <Switch isSelected={wantedMissing} onValueChange={setWantedMissing} />
          </div>

          {selectedCollection && (
            <div className="flex items-center gap-2">
              <Chip color="primary" variant="flat">
                Selected: {selectedCollection.Name}
              </Chip>
            </div>
          )}
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={handleClose}>
            Cancel
          </Button>
          <Button
            color="primary"
            isLoading={adding}
            onPress={() => void handleAdd()}
            isDisabled={!selectedCollection}
          >
            Import Collection
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
}
