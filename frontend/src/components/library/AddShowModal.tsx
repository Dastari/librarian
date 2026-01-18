import { useState } from 'react'
import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { Image } from '@heroui/image'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Input } from '@heroui/input'
import { Select, SelectItem } from '@heroui/select'
import { Chip } from '@heroui/chip'
import { Spinner } from '@heroui/spinner'
import { addToast } from '@heroui/toast'
import {
  graphqlClient,
  SEARCH_TV_SHOWS_QUERY,
  ADD_TV_SHOW_MUTATION,
  type TvShow,
  type TvShowSearchResult,
  type MonitorType,
} from '../../lib/graphql'
import { IconDeviceTv } from '@tabler/icons-react'
import { sanitizeError } from '../../lib/format'


export interface AddShowModalProps {
  isOpen: boolean
  onClose: () => void
  libraryId: string
  onAdded: () => void
}

export function AddShowModal({
  isOpen,
  onClose,
  libraryId,
  onAdded,
}: AddShowModalProps) {
  const [searchQuery, setSearchQuery] = useState('')
  const [searchResults, setSearchResults] = useState<TvShowSearchResult[]>([])
  const [searching, setSearching] = useState(false)
  const [adding, setAdding] = useState(false)
  const [selectedShow, setSelectedShow] = useState<TvShowSearchResult | null>(null)
  const [monitorType, setMonitorType] = useState<MonitorType>('ALL')

  const handleSearch = async () => {
    if (!searchQuery.trim()) return

    try {
      setSearching(true)
      const { data, error } = await graphqlClient
        .query<{ searchTvShows: TvShowSearchResult[] }>(SEARCH_TV_SHOWS_QUERY, {
          query: searchQuery,
        })
        .toPromise()

      if (error) {
        addToast({
          title: 'Error',
          description: sanitizeError(error),
          color: 'danger',
        })
        return
      }

      setSearchResults(data?.searchTvShows || [])
    } catch (err) {
      console.error('Search failed:', err)
    } finally {
      setSearching(false)
    }
  }

  const handleAdd = async () => {
    if (!selectedShow) return

    try {
      setAdding(true)
      const { data, error } = await graphqlClient
        .mutation<{
          addTvShow: {
            success: boolean
            tvShow: TvShow | null
            error: string | null
          }
        }>(ADD_TV_SHOW_MUTATION, {
          libraryId,
          input: {
            provider: selectedShow.provider,
            providerId: selectedShow.providerId,
            monitorType,
          },
        })
        .toPromise()

      if (error || !data?.addTvShow.success) {
        addToast({
          title: 'Error',
          description: sanitizeError(data?.addTvShow.error || 'Failed to add show'),
          color: 'danger',
        })
        return
      }

      addToast({
        title: 'Success',
        description: `Added "${selectedShow.name}" to library`,
        color: 'success',
      })

      // Reset and close
      setSearchQuery('')
      setSearchResults([])
      setSelectedShow(null)
      onClose()
      onAdded()
    } catch (err) {
      console.error('Failed to add show:', err)
    } finally {
      setAdding(false)
    }
  }

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="2xl">
      <ModalContent>
        <ModalHeader>Add TV Show</ModalHeader>
        <ModalBody>
          {!selectedShow ? (
            <div className="space-y-4">
              <Input
                label="Search TV Shows"
                labelPlacement="inside"
                variant="flat"
                placeholder="Search for a TV show..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
                className="flex-1"
                classNames={{
                  label: 'text-sm font-medium text-primary!',
                }}
                endContent={
                  <Button
                    size="sm"
                    variant="light"
                    color="primary"
                    className="font-semibold"
                    onPress={handleSearch}
                    isLoading={searching}
                  >
                    Search
                  </Button>
                }
              />

              {searching ? (
                <div className="flex justify-center py-8">
                  <Spinner size="lg" />
                </div>
              ) : searchResults.length > 0 ? (
                <div className="space-y-2 max-h-96 overflow-auto">
                  {searchResults.map((result) => (
                    <Card
                      key={`${result.provider}-${result.providerId}`}
                      isPressable
                      className="bg-content2 w-full hover:bg-content3"
                      onPress={() => setSelectedShow(result)}
                    >
                      <CardBody className="flex flex-row gap-3 p-2">
                        <div className="shrink-0 w-10">
                          {result.posterUrl ? (
                            <Image
                              src={result.posterUrl}
                              alt={result.name}
                              classNames={{
                                wrapper: "w-full",
                                img: "w-full aspect-[2/3] object-cover"
                              }}
                              radius="sm"
                            />
                          ) : (
                            <div className="w-full aspect-[2/3] bg-default-200 flex items-center justify-center rounded-sm">
                              <IconDeviceTv size={16} className="text-blue-400" />
                            </div>
                          )}
                        </div>
                        <div className="flex-1 min-w-0">
                          <h4 className="font-medium">
                            {result.name}
                            {result.year && (
                              <span className="text-default-500 ml-1">
                                ({result.year})
                              </span>
                            )}
                          </h4>
                          <p className="text-xs text-default-500 line-clamp-2">
                            {result.network && `${result.network} • `}
                            {result.status}
                          </p>
                        </div>
                        <div className="flex items-center">
                          <Chip size="sm" variant="flat">
                            {result.provider}
                          </Chip>
                        </div>
                      </CardBody>
                    </Card>
                  ))}
                </div>
              ) : searchQuery && !searching ? (
                <p className="text-center text-default-500 py-8">
                  No results found
                </p>
              ) : null}
            </div>
          ) : (
            <div className="space-y-4">
              <Card className="bg-content2">
                <CardBody className="flex flex-row gap-4 p-3">
                  <div className="flex-shrink-0 w-24">
                    {selectedShow.posterUrl ? (
                      <Image
                        src={selectedShow.posterUrl}
                        alt={selectedShow.name}
                        classNames={{
                          wrapper: "w-full",
                          img: "w-full aspect-[2/3] object-cover"
                        }}
                        radius="md"
                      />
                    ) : (
                      <div className="w-full aspect-[2/3] bg-default-200 flex items-center justify-center rounded-md">
                        <IconDeviceTv size={32} className="text-blue-400" />
                      </div>
                    )}
                  </div>
                  <div className="flex-1 min-w-0">
                    <h4 className="font-semibold text-lg">
                      {selectedShow.name}
                      {selectedShow.year && (
                        <span className="text-default-500 ml-1">
                          ({selectedShow.year})
                        </span>
                      )}
                    </h4>
                    <p className="text-sm text-default-500">
                      {selectedShow.network && `${selectedShow.network} • `}
                      {selectedShow.status}
                    </p>
                    {selectedShow.overview && (
                      <p className="text-sm text-default-400 mt-2 line-clamp-3">
                        {selectedShow.overview}
                      </p>
                    )}
                  </div>
                </CardBody>
              </Card>

              <Select
                label="Monitor Type"
                selectedKeys={[monitorType]}
                onChange={(e) => setMonitorType(e.target.value as MonitorType)}
                description="Which episodes to track for download"
              >
                <SelectItem key="ALL" textValue="All Episodes">
                  All Episodes - Track all missing episodes
                </SelectItem>
                <SelectItem key="FUTURE" textValue="Future Episodes">
                  Future Only - Only track new episodes going forward
                </SelectItem>
                <SelectItem key="NONE" textValue="Don't Monitor">
                  Don't Monitor - Track but don't download
                </SelectItem>
              </Select>

              <p className="text-xs text-default-400">
                Quality settings will be inherited from the library. You can customize them after adding the show.
              </p>

              <Button
                variant="flat"
                onPress={() => {
                  setSelectedShow(null)
                  setSearchResults([])
                }}
              >
                ← Back to Search
              </Button>
            </div>
          )}
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={onClose}>
            Cancel
          </Button>
          {selectedShow && (
            <Button color="primary" onPress={handleAdd} isLoading={adding}>
              Add Show
            </Button>
          )}
        </ModalFooter>
      </ModalContent>
    </Modal>
  )
}
