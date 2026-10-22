import { SvelteSet } from "svelte/reactivity";
import type { Config, Details, Metadata, Snapshot, Torrent } from "./api";

export const app = $state({
  torrents: [] as Torrent[],
  downloadRate: 0,
  uploadRate: 0,
  config: null as Config | null,
  platform: "windows" as "windows" | "linux" | "macos",
  version: "",
  libtorrent: "",
  error: null as string | null,
  /** Toast-like message at the bottom of the window */
  notice: null as { text: string; kind: "error" | "info" } | null,
});

export const selection = new SvelteSet<number>();

export function applySnapshot(s: Snapshot) {
  app.torrents = s.torrents;
  app.downloadRate = s.download_rate;
  app.uploadRate = s.upload_rate;
  // Forget selected torrents that are gone
  for (const id of selection) if (!s.torrents.some((t) => t.id === id)) selection.delete(id);
}

export const selectedTorrents = () => app.torrents.filter((t) => selection.has(t.id));

let noticeTimer: ReturnType<typeof setTimeout> | undefined;
export function notify(text: string, kind: "error" | "info" = "info") {
  app.notice = { text, kind };
  clearTimeout(noticeTimer);
  noticeTimer = setTimeout(() => (app.notice = null), kind === "error" ? 7000 : 3500);
}

// ---------------------------------------------------------------- dialogs

export type DeleteChoice = "cancel" | "list" | "files";

/** Dialog kind -> [props, result] */
interface DialogTypes {
  confirm: [{ title: string; body: string; action: string }, boolean];
  delete: [{ torrents: Torrent[] }, DeleteChoice];
  magnet: [{ initial: string }, string | null];
  files: [{ metadata: Metadata; priorities?: number[] }, number[] | null];
  properties: [{ details: Details }, { limitKb: number; priorities: number[] | null } | null];
  message: [{ title: string; body: string }, void];
  settings: [{}, void];
  about: [{}, void];
}

export type DialogKind = keyof DialogTypes;

export interface OpenDialog<K extends DialogKind = DialogKind> {
  id: number;
  kind: K;
  props: DialogTypes[K][0];
  resolve: (result: DialogTypes[K][1]) => void;
}

let nextDialogId = 1;
/** Dialogs wait in a queue; only the first one is shown. */
export const dialogs = $state<OpenDialog[]>([]);

export function openDialog<K extends DialogKind>(kind: K, props: DialogTypes[K][0]): Promise<DialogTypes[K][1]> {
  return new Promise((resolve) => {
    const id = nextDialogId++;
    dialogs.push({
      id,
      kind,
      props,
      resolve: (result: DialogTypes[K][1]) => {
        const index = dialogs.findIndex((d) => d.id === id);
        if (index >= 0) dialogs.splice(index, 1);
        resolve(result);
      },
    } as unknown as OpenDialog);
  });
}
