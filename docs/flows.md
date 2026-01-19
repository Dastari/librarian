# Backend Flows

This document describes the key backend flows in Librarian using Mermaid diagrams.

## Table of Contents

- [Library Scanning](#library-scanning)
- [Adding a New Library](#adding-a-new-library)
- [Torrent Lifecycle](#torrent-lifecycle)
- [File Organization](#file-organization)

---

## Library Scanning

When a library scan is triggered (manually, on schedule, or after library creation), the following flow occurs:

### High-Level Scan Flow

```mermaid
flowchart TD
    A[Scan Triggered] --> B{Already Scanning?}
    B -->|Yes| C[Skip - Return Early]
    B -->|No| D[Set scanning = true]
    D --> E[Walk Directory Tree]
    E --> F[Collect Media Files]
    F --> G{Library Type?}
    
    G -->|TV| H[Process TV Library]
    G -->|Movies| I[Process Movies Library]
    G -->|Music| J[Process Music Library]
    G -->|Audiobooks| K[Process Audiobooks Library]
    
    H --> L{auto_add_discovered?}
    L -->|Yes| M[Process with Auto-Add]
    L -->|No| N[Simple File Processing]
    
    M --> O[Group Files by Show Name]
    O --> P[Match/Create Shows via Metadata Service]
    P --> Q[Link Episodes to Files]
    
    N --> R[Create Unlinked Media Files]
    
    I --> S[Parse Movie Filenames]
    J --> T[Parse Audio Metadata]
    K --> U[Parse Audiobook Info]
    
    Q --> V[Queue for FFmpeg Analysis]
    R --> V
    S --> V
    T --> V
    U --> V
    
    V --> W{organize_files enabled?}
    W -->|Yes| X[Run Organizer Service]
    W -->|No| Y[Update last_scanned_at]
    
    X --> Y
    Y --> Z[Set scanning = false]
    Z --> AA[Broadcast Progress Complete]
```

### TV Library Scan Detail (with Auto-Add)

```mermaid
flowchart TD
    subgraph Scanner["Scanner Service"]
        A[Collect Video Files] --> B[Parse Filenames]
        B --> C[Group by Show Name]
        C --> D[Process Show Groups in Parallel]
    end
    
    subgraph ShowProcessing["Per-Show Processing"]
        D --> E[Acquire Semaphore]
        E --> F{Show Exists in DB?}
        F -->|Yes| G[Get Existing Show ID]
        F -->|No| H[Search Metadata Service]
        H --> I{Match Found?}
        I -->|Yes| J[Create Show + Episodes]
        I -->|No| K[Log Warning]
        
        G --> L[Process Files for Show]
        J --> L
        K --> M[Add as Unlinked Files]
    end
    
    subgraph FileProcessing["Per-File Processing"]
        L --> N{File Exists in DB?}
        N -->|Yes| O{Episode Linked?}
        N -->|No| P[Create Media File Record]
        
        O -->|Yes| Q[Skip - Already Processed]
        O -->|No| R[Find Matching Episode]
        R --> S[Link File to Episode]
        S --> T[Mark Episode Downloaded]
        
        P --> U[Link to Episode if Match]
        U --> V[Queue for FFmpeg Analysis]
        T --> V
    end
    
    V --> W[Update Show Stats]
    M --> W
```

### Metadata Lookup Flow

```mermaid
sequenceDiagram
    participant Scanner as ScannerService
    participant Meta as MetadataService
    participant TVMaze as TVMaze API
    participant DB as Database
    
    Scanner->>Meta: search_shows("Show Name")
    Meta->>TVMaze: GET /search/shows?q=...
    TVMaze-->>Meta: Search Results
    Meta-->>Scanner: Vec<SearchResult>
    
    alt Match Found
        Scanner->>Meta: add_tv_show_from_provider(...)
        Meta->>TVMaze: GET /shows/{id}?embed=episodes
        TVMaze-->>Meta: Show Details + Episodes
        Meta->>DB: Create TV Show
        DB-->>Meta: TvShowRecord
        Meta->>DB: Create Episodes (bulk)
        DB-->>Meta: Vec<EpisodeRecord>
        Meta-->>Scanner: TvShowRecord
    else No Match
        Scanner->>Scanner: Log warning, continue
    end
```

---

## Adding a New Library

When a user creates a new library via GraphQL mutation:

```mermaid
flowchart TD
    subgraph GraphQL["GraphQL Mutation"]
        A[createLibrary mutation] --> B[Validate User Auth]
        B --> C[Validate Input]
        C --> D[Create Library Record]
    end
    
    subgraph Database["Database Operations"]
        D --> E[Insert into libraries table]
        E --> F[Return LibraryRecord]
    end
    
    subgraph PostCreation["Post-Creation"]
        F --> G{auto_scan enabled?}
        G -->|Yes| H[Trigger Initial Scan]
        G -->|No| I[Return Success]
        
        H --> J[ScannerService.scan_library]
        J --> K[Full Scan Flow]
        K --> I
    end
```

### Library Settings Applied

```mermaid
flowchart LR
    subgraph LibrarySettings["Library Settings"]
        A[library_type]
        B[organize_files]
        C[rename_style]
        D[naming_pattern]
        E[post_download_action]
        F[auto_add_discovered]
        G[auto_download]
        H[auto_hunt]
        I[quality settings]
    end
    
    subgraph Effects["Applied To"]
        A --> J[File Extensions to Scan]
        A --> K[Parser Selection]
        
        B --> L[Organization Enabled]
        C --> M[Filename Format]
        D --> N[Path Pattern]
        
        E --> O[copy/move/hardlink]
        
        F --> P[Auto-create Shows]
        G --> Q[Auto-grab Torrents]
        H --> R[Hunt on Episode Air]
        
        I --> S[Torrent Filtering]
    end
```

---

## Torrent Lifecycle

### Adding a Torrent

```mermaid
sequenceDiagram
    participant User
    participant GQL as GraphQL
    participant TS as TorrentService
    participant LRQ as librqbit
    participant DB as Database
    participant Events as Broadcast Channel
    
    User->>GQL: addTorrent(magnet, libraryId?)
    GQL->>GQL: Validate Auth
    GQL->>TS: add_magnet(magnet, user_id)
    
    TS->>LRQ: session.add_torrent(magnet)
    LRQ-->>TS: AddTorrentResponse::Added(id, handle)
    
    TS->>TS: Extract info_hash from handle
    TS->>DB: torrents.create(CreateTorrent)
    DB-->>TS: TorrentRecord
    
    TS->>Events: TorrentEvent::Added
    TS-->>GQL: TorrentInfo
    GQL-->>User: AddTorrentResult
```

### Torrent Download Progress

```mermaid
flowchart TD
    subgraph ProgressMonitor["Progress Monitor (1s interval)"]
        A[Tick] --> B[List All Torrents]
        B --> C[For Each Torrent]
        C --> D[Get Stats from librqbit]
        D --> E[Calculate Progress]
        E --> F{Progress >= 100%?}
        
        F -->|Yes| G{Already in completed set?}
        G -->|No| H[Add to completed set]
        H --> I[Broadcast Completed Event]
        G -->|Yes| J[Skip]
        
        F -->|No| K[Broadcast Progress Event]
        
        I --> L[Continue to next torrent]
        J --> L
        K --> L
    end
    
    subgraph DBSync["DB Sync (10s interval)"]
        M[Tick] --> N[List All Torrents]
        N --> O[For Each Torrent]
        O --> P[Get Stats]
        P --> Q[Update DB Record]
        Q --> R{Completed?}
        R -->|Yes| S[Mark completed in DB]
        R -->|No| T[Continue]
    end
```

### Torrent Completion & Processing

```mermaid
flowchart TD
    subgraph DownloadMonitor["Download Monitor Job (1 min interval)"]
        A[Job Triggered] --> B[TorrentProcessor.process_pending_torrents]
        B --> C[Query: state=seeding AND post_process_status IN pending,NULL]
        C --> D{Torrents Found?}
        D -->|No| E[Exit]
        D -->|Yes| F[For Each Torrent]
    end
    
    subgraph Processing["TorrentProcessor.process_torrent"]
        F --> G[Get TorrentRecord from DB]
        G --> H[Mark status = processing]
        H --> I[Get Files from TorrentService]
        I --> J{Determine Processing Type}
        
        J -->|episode_id set| K[process_linked_episode]
        J -->|movie_id set| L[process_linked_movie]
        J -->|album_id set| M[process_linked_music]
        J -->|audiobook_id set| N[process_linked_audiobook]
        J -->|library_id only| O[process_with_library]
        J -->|nothing linked| P[process_without_library]
    end
    
    subgraph Result["Post-Processing Result"]
        K --> Q{Success?}
        L --> Q
        M --> Q
        N --> Q
        O --> Q
        P --> Q
        
        Q -->|matched + organized| R[status = completed]
        Q -->|matched only| S[status = matched]
        Q -->|no match| T[status = unmatched]
        Q -->|error| U[status = error]
    end
```

### Processing Linked Episode

```mermaid
flowchart TD
    A[process_linked_episode] --> B[Get Episode from DB]
    B --> C[Get Show from DB]
    C --> D[Get Library from DB]
    D --> E[For Each Video File]
    
    E --> F{is_video_file?}
    F -->|No| G[Skip]
    F -->|Yes| H[process_video_file]
    
    H --> I[Parse Filename]
    I --> J[Create MediaFile Record]
    J --> K[Queue for FFmpeg Analysis]
    K --> L[Update Episode status = downloaded]
    L --> M[Update Show Stats]
    
    M --> N{organize_files enabled?}
    N -->|Yes| O[Get Organize Settings]
    O --> P[OrganizerService.organize_file]
    P --> Q{Success?}
    Q -->|Yes| R[File Moved/Copied/Hardlinked]
    Q -->|No| S[Log Error]
    
    N -->|No| T[Skip Organization]
    
    R --> U[Return Result]
    S --> U
    T --> U
```

### Auto-Matching Unlinked Torrents

```mermaid
flowchart TD
    A[process_with_library] --> B[Get Library]
    B --> C[For Each File]
    C --> D{Video or Audio?}
    
    D -->|Video + TV Library| E[try_match_tv_file]
    D -->|Video + Movie Library| F[Create Unlinked File]
    D -->|Audio| G[Create Unlinked Audio File]
    
    E --> H[Parse Filename]
    H --> I[Extract Show Name + S##E##]
    I --> J[find_by_name_in_library]
    J --> K{Show Found?}
    
    K -->|Yes| L[get_by_show_season_episode]
    K -->|No| M[Create Unlinked File]
    
    L --> N{Episode Found?}
    N -->|Yes| O[Link Torrent to Episode]
    N -->|No| M
    
    O --> P[process_video_file]
    P --> Q[Organize if Enabled]
    Q --> R[Return matched=true]
    
    M --> S[Return matched=false]
```

---

## File Organization

### Organization Decision Flow

```mermaid
flowchart TD
    A[File Ready for Organization] --> B{Library organize_files?}
    B -->|No| C[Skip Organization]
    B -->|Yes| D{Show has override?}
    
    D -->|Yes| E{Override enables?}
    D -->|No| F[Use Library Setting]
    
    E -->|Yes| G[Organize]
    E -->|No| C
    F --> G
    
    G --> H[Get Effective Settings]
    H --> I[organize_files]
    H --> J[rename_style]
    H --> K[post_download_action]
    H --> L[naming_pattern]
```

### Organize File Operation

```mermaid
flowchart TD
    A[organize_file] --> B[Generate Target Path]
    B --> C{naming_pattern set?}
    
    C -->|Yes| D[Apply Pattern Variables]
    C -->|No| E[Use rename_style]
    
    D --> F[Build Path]
    E --> F
    
    F --> G{Source == Target?}
    G -->|Yes| H[Skip - Already Organized]
    G -->|No| I{Target Exists?}
    
    I -->|Yes| J{Same Size?}
    J -->|Yes| K[Mark as Organized]
    J -->|No| L[Mark as Conflicted]
    
    I -->|No| M[Create Parent Dirs]
    M --> N{post_download_action?}
    
    N -->|move| O[Rename or Copy+Delete]
    N -->|copy| P[Copy File]
    N -->|hardlink| Q[Create Hard Link]
    
    O --> R[Update DB Path]
    P --> R
    Q --> R
    
    R --> S[Mark as Organized]
```

### Naming Pattern Variables

```mermaid
flowchart LR
    subgraph Variables["Available Variables"]
        A["{show}"]
        B["{season}"]
        C["{season:02}"]
        D["{episode}"]
        E["{episode:02}"]
        F["{title}"]
        G["{year}"]
        H["{ext}"]
    end
    
    subgraph Examples["Example Outputs"]
        A --> I["Breaking Bad"]
        B --> J["1"]
        C --> K["01"]
        D --> L["5"]
        E --> M["05"]
        F --> N["Gray Matter"]
        G --> O["2008"]
        H --> P["mkv"]
    end
    
    subgraph Pattern["Default Pattern"]
        Q["{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}"]
    end
    
    subgraph Result["Result"]
        R["Breaking Bad/Season 01/Breaking Bad - S01E05 - Gray Matter.mkv"]
    end
    
    Q --> R
```

---

## Scheduled Jobs

```mermaid
flowchart TD
    subgraph Jobs["Background Jobs"]
        A[Download Monitor] -->|1 min| B[Process Completed Torrents]
        C[RSS Poller] -->|configurable| D[Check RSS Feeds]
        E[Scanner Job] -->|per library| F[Auto-scan Libraries]
        G[Transcode GC] -->|periodic| H[Clean Old Transcodes]
        I[Artwork Job] -->|on demand| J[Fetch Missing Artwork]
    end
    
    subgraph DownloadMonitor["Download Monitor Details"]
        B --> K[TorrentProcessor.process_pending_torrents]
        K --> L[Match Files to Items]
        L --> M[Create Media Records]
        M --> N[Organize Files]
        N --> O[Update Status]
    end
    
    subgraph RSSPoller["RSS Poller Details"]
        D --> P[Fetch Feed Items]
        P --> Q[Parse Torrent Info]
        Q --> R{Matches Quality Profile?}
        R -->|Yes| S{Auto-download Enabled?}
        S -->|Yes| T[Add Torrent]
        R -->|No| U[Skip]
        S -->|No| U
    end
```

---

## Event Flow (Subscriptions)

```mermaid
flowchart LR
    subgraph Publishers["Event Publishers"]
        A[TorrentService]
        B[ScannerService]
        C[Jobs]
    end
    
    subgraph Channels["Broadcast Channels"]
        D[torrent_events]
        E[scan_progress]
        F[log_events]
    end
    
    subgraph Subscribers["GraphQL Subscriptions"]
        G[torrentProgress]
        H[scanStatus]
        I[logStream]
    end
    
    A -->|TorrentEvent| D
    B -->|ScanProgress| E
    C -->|LogEvent| F
    
    D --> G
    E --> H
    F --> I
```
