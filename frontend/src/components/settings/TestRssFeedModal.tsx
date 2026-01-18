import { useState, useEffect } from 'react'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Button } from '@heroui/button'
import { Input } from '@heroui/input'
import { Card, CardBody } from '@heroui/card'
import { Chip } from '@heroui/chip'
import { Divider } from '@heroui/divider'
import { Spinner } from '@heroui/spinner'
import type { RssFeedTestResult } from '../../lib/graphql'
import { sanitizeError } from '../../lib/format'
import { IconCheck } from '@tabler/icons-react'

export interface TestRssFeedModalProps {
  isOpen: boolean
  onClose: () => void
  initialUrl?: string
  onTest: (url: string) => Promise<RssFeedTestResult>
}

export function TestRssFeedModal({
  isOpen,
  onClose,
  initialUrl = '',
  onTest,
}: TestRssFeedModalProps) {
  const [url, setUrl] = useState(initialUrl)
  const [isTesting, setIsTesting] = useState(false)
  const [testResult, setTestResult] = useState<RssFeedTestResult | null>(null)

  // Reset/initialize when modal opens
  useEffect(() => {
    if (isOpen) {
      setUrl(initialUrl)
      setTestResult(null)
    }
  }, [isOpen, initialUrl])

  const handleTest = async () => {
    setIsTesting(true)
    setTestResult(null)
    try {
      const result = await onTest(url)
      setTestResult(result)
    } catch (e) {
      setTestResult({
        success: false,
        itemCount: 0,
        sampleItems: [],
        error: sanitizeError(e),
      })
    } finally {
      setIsTesting(false)
    }
  }

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="2xl">
      <ModalContent>
        <ModalHeader>Test RSS Feed</ModalHeader>
        <ModalBody className="gap-4">
          <div className="flex gap-2">
            <Input
              label="Feed URL"
              placeholder="https://example.com/rss"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              className="flex-1"
            />
            <Button
              color="primary"
              onPress={handleTest}
              isLoading={isTesting}
              isDisabled={!url}
              className="self-end"
            >
              Test
            </Button>
          </div>

          {isTesting && (
            <div className="flex justify-center py-8">
              <Spinner />
            </div>
          )}

          {testResult && (
            <Card className={testResult.success ? 'bg-success-50/20' : 'bg-danger-50/20'}>
              <CardBody>
                {testResult.success ? (
                  <div>
                    <p className="font-medium text-success mb-2 flex items-center gap-1">
                      <IconCheck size={16} className="text-green-400" /> Feed parsed successfully - {testResult.itemCount} items found
                    </p>
                    <Divider className="my-3" />
                    <p className="text-sm text-default-500 mb-2">Sample items:</p>
                    <div className="space-y-2 max-h-80 overflow-y-auto">
                      {testResult.sampleItems.map((item, idx) => (
                        <div key={idx} className="bg-content2 rounded-lg p-3">
                          <p className="font-medium text-sm">{item.title}</p>
                          {item.parsedShowName && (
                            <div className="flex gap-2 mt-1 flex-wrap">
                              <Chip size="sm" variant="flat">
                                {item.parsedShowName}
                              </Chip>
                              {item.parsedSeason && item.parsedEpisode && (
                                <Chip size="sm" variant="flat" color="primary">
                                  S{item.parsedSeason.toString().padStart(2, '0')}E
                                  {item.parsedEpisode.toString().padStart(2, '0')}
                                </Chip>
                              )}
                              {item.parsedResolution && (
                                <Chip size="sm" variant="flat" color="secondary">
                                  {item.parsedResolution}
                                </Chip>
                              )}
                            </div>
                          )}
                        </div>
                      ))}
                    </div>
                  </div>
                ) : (
                  <p className="text-danger">✗ {testResult.error}</p>
                )}
              </CardBody>
            </Card>
          )}
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={onClose}>
            Close
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  )
}
