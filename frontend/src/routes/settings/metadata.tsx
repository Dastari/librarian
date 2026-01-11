import { createFileRoute } from '@tanstack/react-router'
import { useState } from 'react'
import { Button } from '@heroui/button'
import { Card, CardBody, CardHeader } from '@heroui/card'
import { Input, Textarea } from '@heroui/input'
import { Divider } from '@heroui/divider'
import { addToast } from '@heroui/toast'
import { Chip } from '@heroui/chip'
import {
  graphqlClient,
  PARSE_AND_IDENTIFY_QUERY,
  type ParseAndIdentifyResult,
} from '../../lib/graphql'

export const Route = createFileRoute('/settings/metadata')({
  component: MetadataSettingsPage,
})

function MetadataSettingsPage() {
  const [testInput, setTestInput] = useState('')
  const [testResult, setTestResult] = useState<ParseAndIdentifyResult | null>(null)
  const [testing, setTesting] = useState(false)

  const handleTest = async () => {
    if (!testInput.trim()) {
      addToast({
        title: 'Error',
        description: 'Please enter a filename to test',
        color: 'danger',
      })
      return
    }

    try {
      setTesting(true)
      setTestResult(null)

      const { data, error } = await graphqlClient
        .query<{ parseAndIdentifyMedia: ParseAndIdentifyResult }>(
          PARSE_AND_IDENTIFY_QUERY,
          { title: testInput }
        )
        .toPromise()

      if (error) {
        addToast({
          title: 'Error',
          description: `Test failed: ${error.message}`,
          color: 'danger',
        })
        return
      }

      setTestResult(data?.parseAndIdentifyMedia || null)
      addToast({
        title: 'Success',
        description: 'Filename parsed and identified',
        color: 'success',
      })
    } catch (err) {
      console.error('Test failed:', err)
      addToast({
        title: 'Error',
        description: 'Failed to parse filename',
        color: 'danger',
      })
    } finally {
      setTesting(false)
    }
  }

  return (
    <>
      {/* Metadata Providers */}
      <Card className="mb-6">
        <CardHeader>
          <h2 className="font-semibold">Metadata Providers</h2>
        </CardHeader>
        <Divider />
        <CardBody className="space-y-4">
          <div className="flex items-center justify-between p-3 bg-content2 rounded-lg">
            <div className="flex items-center gap-3">
              <span className="text-2xl">🎭</span>
              <div>
                <p className="font-medium">TVMaze</p>
                <p className="text-sm text-default-500">
                  Free, no API key required
                </p>
              </div>
            </div>
            <Chip color="success" variant="flat">
              Active
            </Chip>
          </div>

          <div className="flex items-center justify-between p-3 bg-content2 rounded-lg">
            <div className="flex items-center gap-3">
              <span className="text-2xl">🎬</span>
              <div>
                <p className="font-medium">TMDB</p>
                <p className="text-sm text-default-500">API key required</p>
              </div>
            </div>
            <Chip color="default" variant="flat">
              Coming Soon
            </Chip>
          </div>

          <div className="flex items-center justify-between p-3 bg-content2 rounded-lg">
            <div className="flex items-center gap-3">
              <span className="text-2xl">📺</span>
              <div>
                <p className="font-medium">TheTVDB</p>
                <p className="text-sm text-default-500">API key required</p>
              </div>
            </div>
            <Chip color="default" variant="flat">
              Coming Soon
            </Chip>
          </div>
        </CardBody>
      </Card>

      {/* Test Parser */}
      <Card className="mb-6">
        <CardHeader>
          <h2 className="font-semibold">Test Filename Parser</h2>
        </CardHeader>
        <Divider />
        <CardBody className="space-y-4">
          <p className="text-sm text-default-500">
            Enter a torrent name or filename to test how it will be parsed and
            matched to TV shows.
          </p>

          <div className="flex gap-2">
            <Input
              placeholder="e.g., Chicago Fire S14E08 1080p WEB h264-ETHEL"
              value={testInput}
              onChange={(e) => setTestInput(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleTest()}
              className="flex-1"
            />
            <Button
              color="primary"
              onPress={handleTest}
              isLoading={testing}
            >
              Test
            </Button>
          </div>

          <div className="text-xs text-default-400">
            <p className="font-medium mb-1">Example inputs:</p>
            <ul className="list-disc list-inside space-y-1">
              <li>Chicago Fire S14E08 1080p WEB h264-ETHEL</li>
              <li>The.Daily.Show.2026.01.07.Stephen.J.Dubner.720p.WEB.h264-EDITH</li>
              <li>Corner Gas S06E12 Super Sensitive 1080p AMZN WEB-DL DDP2.0 H.264-QOQ</li>
            </ul>
          </div>
        </CardBody>
      </Card>

      {/* Test Result */}
      {testResult && (
        <Card className="mb-6">
          <CardHeader>
            <h2 className="font-semibold">Parse Result</h2>
          </CardHeader>
          <Divider />
          <CardBody className="space-y-4">
            {/* Parsed Info */}
            <div>
              <h3 className="text-sm font-medium text-default-500 mb-2">
                Parsed Information
              </h3>
              <div className="grid grid-cols-2 gap-2 text-sm">
                {testResult.parsed.showName && (
                  <div>
                    <span className="text-default-500">Show:</span>
                    <span className="ml-2 font-medium">
                      {testResult.parsed.showName}
                    </span>
                  </div>
                )}
                {testResult.parsed.season != null && (
                  <div>
                    <span className="text-default-500">Season:</span>
                    <span className="ml-2 font-medium">
                      {testResult.parsed.season}
                    </span>
                  </div>
                )}
                {testResult.parsed.episode != null && (
                  <div>
                    <span className="text-default-500">Episode:</span>
                    <span className="ml-2 font-medium">
                      {testResult.parsed.episode}
                    </span>
                  </div>
                )}
                {testResult.parsed.year && (
                  <div>
                    <span className="text-default-500">Year:</span>
                    <span className="ml-2 font-medium">
                      {testResult.parsed.year}
                    </span>
                  </div>
                )}
                {testResult.parsed.date && (
                  <div>
                    <span className="text-default-500">Date:</span>
                    <span className="ml-2 font-medium">
                      {testResult.parsed.date}
                    </span>
                  </div>
                )}
                {testResult.parsed.resolution && (
                  <div>
                    <span className="text-default-500">Resolution:</span>
                    <Chip size="sm" variant="flat" className="ml-2">
                      {testResult.parsed.resolution}
                    </Chip>
                  </div>
                )}
                {testResult.parsed.source && (
                  <div>
                    <span className="text-default-500">Source:</span>
                    <span className="ml-2">{testResult.parsed.source}</span>
                  </div>
                )}
                {testResult.parsed.codec && (
                  <div>
                    <span className="text-default-500">Codec:</span>
                    <span className="ml-2">{testResult.parsed.codec}</span>
                  </div>
                )}
                {testResult.parsed.hdr && (
                  <div>
                    <span className="text-default-500">HDR:</span>
                    <Chip size="sm" color="secondary" variant="flat" className="ml-2">
                      {testResult.parsed.hdr}
                    </Chip>
                  </div>
                )}
                {testResult.parsed.audio && (
                  <div>
                    <span className="text-default-500">Audio:</span>
                    <span className="ml-2">{testResult.parsed.audio}</span>
                  </div>
                )}
                {testResult.parsed.releaseGroup && (
                  <div>
                    <span className="text-default-500">Release Group:</span>
                    <span className="ml-2">{testResult.parsed.releaseGroup}</span>
                  </div>
                )}
              </div>
              {(testResult.parsed.isProper || testResult.parsed.isRepack) && (
                <div className="flex gap-2 mt-2">
                  {testResult.parsed.isProper && (
                    <Chip size="sm" color="success" variant="flat">
                      PROPER
                    </Chip>
                  )}
                  {testResult.parsed.isRepack && (
                    <Chip size="sm" color="success" variant="flat">
                      REPACK
                    </Chip>
                  )}
                </div>
              )}
            </div>

            <Divider />

            {/* Matches */}
            <div>
              <h3 className="text-sm font-medium text-default-500 mb-2">
                Show Matches ({testResult.matches.length})
              </h3>
              {testResult.matches.length > 0 ? (
                <div className="space-y-2">
                  {testResult.matches.slice(0, 5).map((match, i) => (
                    <div
                      key={`${match.provider}-${match.providerId}`}
                      className="flex items-center justify-between p-2 bg-content2 rounded-lg"
                    >
                      <div className="flex items-center gap-3">
                        <span className="text-default-400 text-sm">
                          #{i + 1}
                        </span>
                        <div>
                          <p className="font-medium">
                            {match.name}
                            {match.year && (
                              <span className="text-default-500 ml-1">
                                ({match.year})
                              </span>
                            )}
                          </p>
                          <p className="text-xs text-default-500">
                            {match.network} • {match.status}
                          </p>
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        <Chip size="sm" variant="flat">
                          {match.provider}
                        </Chip>
                        <span className="text-sm text-default-500">
                          {(match.score * 100).toFixed(0)}%
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="text-default-500 text-sm">
                  No matches found. Try a different filename or check that the
                  show name is correct.
                </p>
              )}
            </div>
          </CardBody>
        </Card>
      )}

      {/* Raw JSON output for debugging */}
      {testResult && (
        <Card>
          <CardHeader>
            <h2 className="font-semibold">Raw JSON Output</h2>
          </CardHeader>
          <Divider />
          <CardBody>
            <Textarea
              isReadOnly
              value={JSON.stringify(testResult, null, 2)}
              minRows={10}
              maxRows={20}
              classNames={{
                input: 'font-mono text-xs',
              }}
            />
          </CardBody>
        </Card>
      )}
    </>
  )
}
