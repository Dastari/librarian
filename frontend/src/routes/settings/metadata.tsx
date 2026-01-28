import { createFileRoute } from '@tanstack/react-router'
import { useState, useEffect, useMemo } from 'react'
import { Button } from '@heroui/button'
// Card is used for test results display when needed
import { Input, Textarea } from '@heroui/input'
import { Switch } from '@heroui/switch'
import { Accordion, AccordionItem } from '@heroui/accordion'
import { Divider } from '@heroui/divider'
import { addToast } from '@heroui/toast'
import { Chip } from '@heroui/chip'
import { Spinner } from '@heroui/spinner'
import {
  graphqlClient,
  PARSE_AND_IDENTIFY_QUERY,
  type ParseAndIdentifyResult,
} from '../../lib/graphql'
import {
  MetadataAppSettingsDocument,
  UpdateAppSettingDocument,
  CreateAppSettingDocument,
  type MetadataAppSettingsQuery,
  type UpdateAppSettingMutation,
  type CreateAppSettingMutation,
} from "../../lib/graphql/generated/graphql";
import {
  IconMovie,
  IconDeviceTv,
  IconMusic,
  IconHeadphones,
  IconSubtask,
  IconKey,
  IconTestPipe,
} from "@tabler/icons-react";
import { sanitizeError } from '../../lib/format'
import { SettingsHeader } from '../../components/shared'

export const Route = createFileRoute('/settings/metadata')({
  component: MetadataSettingsPage,
})

const METADATA_KEYS = {
  tmdb_api_key: "metadata.tmdb_api_key",
  tmdb_enabled: "metadata.tmdb_enabled",
  tvmaze_enabled: "metadata.tvmaze_enabled",
  musicbrainz_enabled: "metadata.musicbrainz_enabled",
  openlibrary_enabled: "metadata.openlibrary_enabled",
  opensubtitles_api_key: "metadata.opensubtitles_api_key",
  opensubtitles_username: "metadata.opensubtitles_username",
  opensubtitles_password: "metadata.opensubtitles_password",
  opensubtitles_enabled: "metadata.opensubtitles_enabled",
} as const;

/** Shape used by the form and for change detection (from AppSettings key/value store). */
interface MetadataSettingsShape {
  tmdbApiKey: string;
  tmdbEnabled: boolean;
  tvmazeEnabled: boolean;
  musicbrainzEnabled: boolean;
  openlibraryEnabled: boolean;
  opensubtitlesApiKey: string;
  opensubtitlesUsername: string;
  opensubtitlesPassword: string;
  opensubtitlesEnabled: boolean;
}

function appSettingsToMetadataSettings(
  edges: MetadataAppSettingsQuery["AppSettings"]["Edges"],
): MetadataSettingsShape {
  const map = new Map(edges.map((e) => [e.Node.Key, e.Node.Value]));
  const get = (k: string, def: string) => map.get(k) ?? def;
  const getBool = (k: string, def: boolean) => {
    const val = map.get(k);
    if (val === "true") return true;
    if (val === "false") return false;
    return def;
  };
  return {
    tmdbApiKey: get(METADATA_KEYS.tmdb_api_key, ""),
    tmdbEnabled: getBool(METADATA_KEYS.tmdb_enabled, true),
    tvmazeEnabled: getBool(METADATA_KEYS.tvmaze_enabled, true),
    musicbrainzEnabled: getBool(METADATA_KEYS.musicbrainz_enabled, true),
    openlibraryEnabled: getBool(METADATA_KEYS.openlibrary_enabled, true),
    opensubtitlesApiKey: get(METADATA_KEYS.opensubtitles_api_key, ""),
    opensubtitlesUsername: get(METADATA_KEYS.opensubtitles_username, ""),
    opensubtitlesPassword: get(METADATA_KEYS.opensubtitles_password, ""),
    opensubtitlesEnabled: getBool(METADATA_KEYS.opensubtitles_enabled, false),
  };
}

/** Map from app setting key to node Id (for updates). */
function keyToIdMap(edges: MetadataAppSettingsQuery['AppSettings']['Edges']): Map<string, string> {
  return new Map(edges.map((e) => [e.Node.Key, e.Node.Id]))
}

function MetadataSettingsPage() {
  const [settings, setSettings] = useState<MetadataSettingsShape>({
    tmdbApiKey: "",
    tmdbEnabled: true,
    tvmazeEnabled: true,
    musicbrainzEnabled: true,
    openlibraryEnabled: true,
    opensubtitlesApiKey: "",
    opensubtitlesUsername: "",
    opensubtitlesPassword: "",
    opensubtitlesEnabled: false,
  });
  const [initialSettings, setInitialSettings] =
    useState<MetadataSettingsShape | null>(null);
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [keyToId, setKeyToId] = useState<Map<string, string>>(new Map());
  
  // Parser test state
  const [testInput, setTestInput] = useState('')
  const [testResult, setTestResult] = useState<ParseAndIdentifyResult | null>(null)
  const [testing, setTesting] = useState(false)

  // Fetch settings on mount
  useEffect(() => {
    fetchSettings()
  }, [])

  const fetchSettings = async () => {
    try {
      setLoading(true)
      const { data, error } = await graphqlClient
        .query<MetadataAppSettingsQuery>(MetadataAppSettingsDocument, {})
        .toPromise();

      if (error) {
        throw error;
      }

      if (!data?.AppSettings?.Edges) {
        throw new Error("No settings data returned");
      }

      const newSettings = appSettingsToMetadataSettings(data.AppSettings.Edges);
      const idMap = keyToIdMap(data.AppSettings.Edges);
      
      setSettings(newSettings)
      setKeyToId(idMap);
      setInitialSettings({ ...newSettings })
    } catch (err) {
      // Silently ignore auth errors - they can happen during login race conditions
      const errorMsg = err instanceof Error ? err.message : String(err);
      if (!errorMsg.toLowerCase().includes('authentication')) {
        console.error('Failed to fetch settings:', err)
        addToast({
          title: 'Error',
          description: 'Failed to load metadata settings',
          color: 'danger',
        })
      }
      // Still set initial settings to allow editing even on error
      setInitialSettings({ ...settings })
    } finally {
      setLoading(false)
    }
  }

  const hasChanges = useMemo(() => {
    if (!initialSettings) return false
    return (
      settings.tmdbApiKey !== initialSettings.tmdbApiKey ||
      settings.tmdbEnabled !== initialSettings.tmdbEnabled ||
      settings.tvmazeEnabled !== initialSettings.tvmazeEnabled ||
      settings.musicbrainzEnabled !== initialSettings.musicbrainzEnabled ||
      settings.openlibraryEnabled !== initialSettings.openlibraryEnabled ||
      settings.opensubtitlesApiKey !== initialSettings.opensubtitlesApiKey ||
      settings.opensubtitlesUsername !== initialSettings.opensubtitlesUsername ||
      settings.opensubtitlesPassword !== initialSettings.opensubtitlesPassword ||
      settings.opensubtitlesEnabled !== initialSettings.opensubtitlesEnabled
    )
  }, [settings, initialSettings])

  const handleSave = async () => {
    setSaving(true)
    try {
      const settingsToSave = [
        { key: METADATA_KEYS.tmdb_api_key, value: settings.tmdbApiKey },
        {
          key: METADATA_KEYS.tmdb_enabled,
          value: String(settings.tmdbEnabled),
        },
        {
          key: METADATA_KEYS.tvmaze_enabled,
          value: String(settings.tvmazeEnabled),
        },
        {
          key: METADATA_KEYS.musicbrainz_enabled,
          value: String(settings.musicbrainzEnabled),
        },
        {
          key: METADATA_KEYS.openlibrary_enabled,
          value: String(settings.openlibraryEnabled),
        },
        {
          key: METADATA_KEYS.opensubtitles_api_key,
          value: settings.opensubtitlesApiKey,
        },
        {
          key: METADATA_KEYS.opensubtitles_username,
          value: settings.opensubtitlesUsername,
        },
        {
          key: METADATA_KEYS.opensubtitles_password,
          value: settings.opensubtitlesPassword,
        },
        {
          key: METADATA_KEYS.opensubtitles_enabled,
          value: String(settings.opensubtitlesEnabled),
        },
      ];

      const now = new Date().toISOString();

      for (const { key, value } of settingsToSave) {
        const existingId = keyToId.get(key);

        if (existingId) {
          // Update existing setting using UpdateAppSetting mutation
          const res = await graphqlClient
            .mutation(UpdateAppSettingDocument, {
              Id: existingId,
              Input: { Value: value },
            })
            .toPromise();

          const data = res.data as {
            UpdateAppSetting?: { Success: boolean; Error?: string | null };
          };
          if (!data?.UpdateAppSetting?.Success) {
            addToast({
              title: "Error",
              description: sanitizeError(
                data?.UpdateAppSetting?.Error ?? `Failed to save ${key}`,
              ),
              color: "danger",
            });
            return;
          }
        } else {
          // Create new setting
          const res = await graphqlClient
            .mutation(CreateAppSettingDocument, {
              Input: {
                Key: key,
                Value: value,
                Category: "metadata",
                CreatedAt: now,
                UpdatedAt: now,
              },
            })
            .toPromise();

          const data = res.data as {
            CreateAppSetting?: {
              Success: boolean;
              Error?: string | null;
              AppSetting?: { Id: string } | null;
            };
          };
          if (!data?.CreateAppSetting?.Success) {
            addToast({
              title: "Error",
              description: sanitizeError(
                data?.CreateAppSetting?.Error ?? `Failed to create ${key}`,
              ),
              color: "danger",
            });
            return;
          }
        }
      }

      // Refetch settings to get updated IDs and values
      await fetchSettings();
      
      addToast({
        title: 'Success',
        description: 'Metadata settings saved',
        color: 'success',
      })
    } catch (err) {
      console.error('Failed to save settings:', err)
      addToast({
        title: "Error",
        description: sanitizeError(err),
        color: "danger",
      });
    } finally {
      setSaving(false)
    }
  }

  const handleReset = () => {
    if (initialSettings) {
      setSettings({ ...initialSettings })
    }
  }

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
          description: `Test failed: ${sanitizeError(error)}`,
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

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Spinner size="lg" />
      </div>
    )
  }

  return (
    <div className="grow overflow-y-auto overflow-x-hidden pb-8" style={{ scrollbarGutter: 'stable' }}>
      <SettingsHeader
        title="Metadata Providers"
        subtitle="Configure metadata sources for TV shows, movies, music, and audiobooks"
        onSave={handleSave}
        onReset={handleReset}
        isSaveDisabled={!hasChanges}
        isResetDisabled={!hasChanges}
        isSaving={saving}
        hasChanges={hasChanges || false}
      />

      <Accordion
        selectionMode="multiple"
        variant="splitted"
      >
        {/* TV Shows Section */}
        <AccordionItem
          key="tv"
          aria-label="TV Shows"
          title={
            <div className="flex items-center gap-2">
              <IconDeviceTv size={18} className="text-blue-400" />
              <span className="font-semibold">TV Shows</span>
            </div>
          }
          subtitle="Episode and series metadata"
        >
          <div className="space-y-4 pb-2">
            <div className="flex items-center justify-between p-3 bg-content2 rounded-lg">
              <div>
                <p className="font-medium">TVMaze</p>
                <p className="text-sm text-default-500">
                  Free API, no key required. Primary source for TV show metadata.
                </p>
              </div>
              <div className="flex items-center gap-3">
                <Chip
                  color={settings.tvmazeEnabled ? 'success' : 'default'}
                  variant="flat"
                  size="sm"
                >
                  {settings.tvmazeEnabled ? 'Enabled' : 'Disabled'}
                </Chip>
                <Switch
                  isSelected={settings.tvmazeEnabled}
                  onValueChange={(v) => setSettings(prev => ({ ...prev, tvmazeEnabled: v }))}
                />
              </div>
            </div>
          </div>
        </AccordionItem>

        {/* Movies Section */}
        <AccordionItem
          key="movies"
          aria-label="Movies"
          title={
            <div className="flex items-center gap-2">
              <IconMovie size={18} className="text-purple-400" />
              <span className="font-semibold">Movies</span>
            </div>
          }
          subtitle="Film metadata and artwork"
        >
          <div className="space-y-4 pb-2">
            <div className="p-3 bg-content2 rounded-lg space-y-3">
              <div className="flex items-center justify-between">
                <div>
                  <p className="font-medium">TMDB (The Movie Database)</p>
                  <p className="text-sm text-default-500">
                    Comprehensive movie database. Requires free API key.
                  </p>
                </div>
                <div className="flex items-center gap-3">
                  <Chip
                    color={settings.tmdbEnabled && settings.tmdbApiKey ? 'success' : settings.tmdbEnabled ? 'warning' : 'default'}
                    variant="flat"
                    size="sm"
                  >
                    {settings.tmdbEnabled ? (settings.tmdbApiKey ? 'Configured' : 'No API Key') : 'Disabled'}
                  </Chip>
                <Switch
                  isSelected={settings.tmdbEnabled}
                  onValueChange={(v) => setSettings(prev => ({ ...prev, tmdbEnabled: v }))}
                />
                </div>
              </div>
              {settings.tmdbEnabled && (
                <Input
                  label="TMDB API Key"
                  labelPlacement="inside"
                  variant="flat"
                  placeholder="Enter your TMDB API key"
                  value={settings.tmdbApiKey}
                  onValueChange={(v) => setSettings(prev => ({ ...prev, tmdbApiKey: v }))}
                  type="password"
                  startContent={<IconKey size={16} className="text-default-400" />}
                  classNames={{
                    label: 'text-sm font-medium text-primary!',
                  }}
                  description={
                    <span>
                      Get a free API key at{' '}
                      <a
                        href="https://www.themoviedb.org/settings/api"
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-primary hover:underline"
                      >
                        themoviedb.org
                      </a>
                    </span>
                  }
                />
              )}
            </div>
          </div>
        </AccordionItem>

        {/* Music Section */}
        <AccordionItem
          key="music"
          aria-label="Music"
          title={
            <div className="flex items-center gap-2">
              <IconMusic size={18} className="text-green-400" />
              <span className="font-semibold">Music</span>
            </div>
          }
          subtitle="Artist, album, and track metadata"
        >
          <div className="space-y-4 pb-2">
            <div className="flex items-center justify-between p-3 bg-content2 rounded-lg">
              <div>
                <p className="font-medium">MusicBrainz</p>
                <p className="text-sm text-default-500">
                  Free, open music database. No API key required.
                </p>
              </div>
              <div className="flex items-center gap-3">
                <Chip
                  color={settings.musicbrainzEnabled ? 'success' : 'default'}
                  variant="flat"
                  size="sm"
                >
                  {settings.musicbrainzEnabled ? 'Enabled' : 'Disabled'}
                </Chip>
                <Switch
                  isSelected={settings.musicbrainzEnabled}
                  onValueChange={(v) => setSettings(prev => ({ ...prev, musicbrainzEnabled: v }))}
                />
              </div>
            </div>
          </div>
        </AccordionItem>

        {/* Audiobooks Section */}
        <AccordionItem
          key="audiobooks"
          aria-label="Audiobooks"
          title={
            <div className="flex items-center gap-2">
              <IconHeadphones size={18} className="text-amber-400" />
              <span className="font-semibold">Audiobooks</span>
            </div>
          }
          subtitle="Book and author metadata"
        >
          <div className="space-y-4 pb-2">
            <div className="flex items-center justify-between p-3 bg-content2 rounded-lg">
              <div>
                <p className="font-medium">OpenLibrary</p>
                <p className="text-sm text-default-500">
                  Free, open book database. No API key required.
                </p>
              </div>
              <div className="flex items-center gap-3">
                <Chip
                  color={settings.openlibraryEnabled ? 'success' : 'default'}
                  variant="flat"
                  size="sm"
                >
                  {settings.openlibraryEnabled ? 'Enabled' : 'Disabled'}
                </Chip>
                <Switch
                  isSelected={settings.openlibraryEnabled}
                  onValueChange={(v) => setSettings(prev => ({ ...prev, openlibraryEnabled: v }))}
                />
              </div>
            </div>
          </div>
        </AccordionItem>

        {/* Subtitles Section */}
        <AccordionItem
          key="subtitles"
          aria-label="Subtitles"
          title={
            <div className="flex items-center gap-2">
              <IconSubtask size={18} className="text-cyan-400" />
              <span className="font-semibold">Subtitles</span>
            </div>
          }
          subtitle="Automatic subtitle downloads"
        >
          <div className="space-y-4 pb-2">
            <div className="p-3 bg-content2 rounded-lg space-y-3">
              <div className="flex items-center justify-between">
                <div>
                  <p className="font-medium">OpenSubtitles</p>
                  <p className="text-sm text-default-500">
                    Large subtitle database. Requires free account.
                  </p>
                </div>
                <div className="flex items-center gap-3">
                  <Chip
                    color={settings.opensubtitlesEnabled && settings.opensubtitlesApiKey ? 'success' : settings.opensubtitlesEnabled ? 'warning' : 'default'}
                    variant="flat"
                    size="sm"
                  >
                    {settings.opensubtitlesEnabled ? (settings.opensubtitlesApiKey ? 'Configured' : 'Missing Credentials') : 'Disabled'}
                  </Chip>
                  <Switch
                    isSelected={settings.opensubtitlesEnabled}
                    onValueChange={(v) => setSettings(prev => ({ ...prev, opensubtitlesEnabled: v }))}
                  />
                </div>
              </div>
              {settings.opensubtitlesEnabled && (
                <div className="space-y-3">
                  <Input
                    label="API Key"
                    labelPlacement="inside"
                    variant="flat"
                    placeholder="Enter your OpenSubtitles API key"
                    value={settings.opensubtitlesApiKey}
                    onValueChange={(v) => setSettings(prev => ({ ...prev, opensubtitlesApiKey: v }))}
                    type="password"
                    startContent={<IconKey size={16} className="text-default-400" />}
                    classNames={{
                      label: 'text-sm font-medium text-primary!',
                    }}
                  />
                  <div className="grid grid-cols-2 gap-3">
                    <Input
                      label="Username"
                      labelPlacement="inside"
                      variant="flat"
                      placeholder="OpenSubtitles username"
                      value={settings.opensubtitlesUsername}
                      onValueChange={(v) => setSettings(prev => ({ ...prev, opensubtitlesUsername: v }))}
                      classNames={{
                        label: 'text-sm font-medium text-primary!',
                      }}
                    />
                    <Input
                      label="Password"
                      labelPlacement="inside"
                      variant="flat"
                      placeholder="OpenSubtitles password"
                      value={settings.opensubtitlesPassword}
                      onValueChange={(v) => setSettings(prev => ({ ...prev, opensubtitlesPassword: v }))}
                      type="password"
                      classNames={{
                        label: 'text-sm font-medium text-primary!',
                      }}
                    />
                  </div>
                  <p className="text-xs text-default-400">
                    Create a free account at{' '}
                    <a
                      href="https://www.opensubtitles.com"
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-primary hover:underline"
                    >
                      opensubtitles.com
                    </a>
                  </p>
                </div>
              )}
            </div>
          </div>
        </AccordionItem>

        {/* Test Parser Section */}
        <AccordionItem
          key="parser"
          aria-label="Test Parser"
          title={
            <div className="flex items-center gap-2">
              <IconTestPipe size={18} className="text-default-400" />
              <span className="font-semibold">Test Filename Parser</span>
            </div>
          }
          subtitle="Test how filenames are parsed and matched"
        >
          <div className="space-y-4 pb-2">
            <p className="text-sm text-default-500">
              Enter a torrent name or filename to test how it will be parsed and
              matched to media items.
            </p>

            <Input
              label="Test Filename"
              labelPlacement="inside"
              variant="flat"
              placeholder="e.g., Chicago Fire S14E08 1080p WEB h264-ETHEL"
              value={testInput}
              onChange={(e) => setTestInput(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleTest()}
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
                  onPress={handleTest}
                  isLoading={testing}
                >
                  Test
                </Button>
              }
            />

            <div className="text-xs text-default-400">
              <p className="font-medium mb-1">Example inputs:</p>
              <ul className="list-disc list-inside space-y-1">
                <li>Chicago Fire S14E08 1080p WEB h264-ETHEL</li>
                <li>Inception.2010.1080p.BluRay.x264-SPARKS</li>
                <li>Pink Floyd - The Dark Side of the Moon (1973) [FLAC]</li>
              </ul>
            </div>

            {/* Test Result */}
            {testResult && (
              <div className="space-y-4 mt-4">
                <Divider />
                
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
                    {testResult.parsed.releaseGroup && (
                      <div>
                        <span className="text-default-500">Release Group:</span>
                        <span className="ml-2">{testResult.parsed.releaseGroup}</span>
                      </div>
                    )}
                  </div>
                </div>

                <Divider />

                {/* Matches */}
                <div>
                  <h3 className="text-sm font-medium text-default-500 mb-2">
                    Matches ({testResult.matches.length})
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
                      No matches found. Try a different filename.
                    </p>
                  )}
                </div>

                <Divider />

                {/* Raw JSON */}
                <div>
                  <h3 className="text-sm font-medium text-default-500 mb-2">
                    Raw JSON Output
                  </h3>
                  <Textarea
                    isReadOnly
                    value={JSON.stringify(testResult, null, 2)}
                    minRows={6}
                    maxRows={12}
                    classNames={{
                      input: 'font-mono text-xs',
                    }}
                  />
                </div>
              </div>
            )}
          </div>
        </AccordionItem>
      </Accordion>
    </div>
  )
}
