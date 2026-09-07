export interface DownloadTorrent {
  id: string;
  infoHash: string;
  name: string;
  state: string;
  progress: number;
  totalBytes: number;
  downloadedBytes: number;
  uploadedBytes: number;
  savePath: string;
  addedAt: string;
}
