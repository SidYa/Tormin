// Typed wrappers over the Rust commands (src-tauri/src/lib.rs).
import { invoke } from "@tauri-apps/api/core";

export type TorrentState = "unknown" | "checking" | "metadata" | "downloading" | "finished" | "seeding";

export interface Torrent {
  id: number;
  name: string;
  state: TorrentState;
  progress: number;
  paused: boolean;
  finished: boolean;
  has_metadata: boolean;
  download_rate: number;
  upload_rate: number;
  total: number;
  done: number;
  peers: number;
  seeds: number;
}

export interface Snapshot {
  torrents: Torrent[];
  download_rate: number;
  upload_rate: number;
}

export interface FileEntry {
  index: number;
  path: string;
  size: number;
}

export interface Metadata {
  name: string;
  total_size: number;
  files: FileEntry[];
}

export interface Details {
  id: number;
  name: string;
  save_path: string;
  download_limit_kb: number;
  metadata: Metadata | null;
  priorities: number[];
}

export interface Config {
  download_dir: string;
  language: string;
  theme: "system" | "light" | "dark";
  accent: string;
  compact_mode: boolean;
  compact_window_size: boolean;
  progress_style: 1 | 2 | 3;
  monochrome: boolean;
  shutdown_complete: boolean;
  start_with_windows: boolean;
  start_minimized: boolean;
}

export interface InitialState {
  config: Config;
  snapshot: Snapshot | null;
  version: string;
  libtorrent: string;
  platform: "windows" | "linux" | "macos";
  error: string | null;
}

export interface OpenRequest {
  files: string[];
  magnets: string[];
}

export const api = {
  getState: () => invoke<InitialState>("get_state"),
  takePendingOpen: () => invoke<OpenRequest[]>("take_pending_open"),
  readTorrentFile: (path: string) => invoke<Metadata>("read_torrent_file", { path }),
  addTorrentFile: (path: string, priorities: number[] | null = null) =>
    invoke<number>("add_torrent_file", { path, priorities }),
  addMagnet: (uri: string) => invoke<number>("add_magnet", { uri }),
  pause: (ids: number[]) => invoke<void>("pause", { ids }),
  resume: (ids: number[]) => invoke<void>("resume", { ids }),
  remove: (ids: number[], deleteFiles: boolean) => invoke<void>("remove", { ids, deleteFiles }),
  details: (id: number) => invoke<Details>("torrent_details", { id }),
  applyProperties: (id: number, downloadLimitKb: number, priorities: number[] | null) =>
    invoke<void>("apply_properties", { id, downloadLimitKb, priorities }),
  completeFileSelection: (id: number, priorities: number[] | null) =>
    invoke<void>("complete_file_selection", { id, priorities }),
  openFolder: (id: number) => invoke<void>("open_folder", { id }),
  setConfig: (config: Config) => invoke<void>("set_config", { config }),
  associateFiles: () => invoke<void>("associate_files"),
  quit: () => invoke<void>("quit"),
};
