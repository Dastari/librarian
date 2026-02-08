export interface DownloadTorrent {
  Id: string;
  InfoHash: string;
  Name: string;
  State: string;
  Progress: number;
  TotalBytes: number;
  DownloadedBytes: number;
  UploadedBytes: number;
  SavePath: string;
  AddedAt: string;
}
