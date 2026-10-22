// User-level flows shared by the toolbar, context menu, shortcuts and drag & drop.
import { open } from "@tauri-apps/plugin-dialog";
import { readText } from "./clipboard";
import { api, type Metadata, type OpenRequest } from "./api";
import { isMagnet, magnetName } from "./format";
import { t } from "./i18n.svelte";
import { app, notify, openDialog, selectedTorrents, selection } from "./store.svelte";

async function attempt<T>(work: () => Promise<T>): Promise<T | undefined> {
  try {
    return await work();
  } catch (e) {
    notify(String(e), "error");
    return undefined;
  }
}

/** Adds a .torrent file: multi-file torrents get the file picker, others an optional confirmation. */
export async function addTorrentFile(path: string, confirm = false) {
  const metadata = await attempt(() => api.readTorrentFile(path));
  if (!metadata) return;
  if (metadata.files.length > 1) {
    const priorities = await openDialog("files", { metadata });
    if (priorities) await attempt(() => api.addTorrentFile(path, priorities));
  } else if (
    !confirm ||
    (await openDialog("confirm", {
      title: t("add_torrent_single", metadata.name),
      body: t("add_torrent_confirm"),
      action: t("add_torrent_action"),
    }))
  ) {
    await attempt(() => api.addTorrentFile(path));
  }
}

export async function addMagnet(uri: string, confirm = false) {
  if (
    confirm &&
    !(await openDialog("confirm", {
      title: t("add_magnet", magnetName(uri)),
      body: t("add_magnet_confirm"),
      action: t("add_magnet_action"),
    }))
  )
    return;
  await attempt(() => api.addMagnet(uri));
}

export async function pickTorrentFiles() {
  const picked = await open({ multiple: true, filters: [{ name: "Torrent", extensions: ["torrent"] }] });
  for (const path of [picked ?? []].flat()) await addTorrentFile(path);
}

export async function askMagnet() {
  const clip = (await readText()).trim();
  const uri = await openDialog("magnet", { initial: isMagnet(clip) ? clip : "" });
  if (uri) await addMagnet(uri);
}

/** Files and links passed on the command line or from a second launch. */
export async function handleOpenRequest(req: OpenRequest) {
  for (const file of req.files) await addTorrentFile(file, true);
  for (const magnet of req.magnets) await addMagnet(magnet, true);
}

const targets = () => {
  const selected = selectedTorrents();
  // Like the Linux version: without a selection the action applies to every torrent
  return (selected.length ? selected : app.torrents).map((t) => t.id);
};

export const resumeSelected = () => attempt(() => api.resume(targets()));
export const pauseSelected = () => attempt(() => api.pause(targets()));

export async function deleteSelected() {
  const torrents = selectedTorrents();
  if (!torrents.length) return;
  const choice = await openDialog("delete", { torrents });
  if (choice === "cancel") return;
  await attempt(() => api.remove(torrents.map((t) => t.id), choice === "files"));
  selection.clear();
}

export async function showProperties(id?: number) {
  const target = id ?? (selection.size === 1 ? [...selection][0] : undefined);
  if (target === undefined) return;
  const details = await attempt(() => api.details(target));
  if (!details) return;
  const result = await openDialog("properties", { details });
  if (result) await attempt(() => api.applyProperties(target, result.limitKb, result.priorities));
}

export async function openFolders() {
  for (const t of selectedTorrents().slice(0, 5)) await attempt(() => api.openFolder(t.id));
}

/** Magnet metadata arrived: let the user choose files (cancel removes the magnet). */
export async function selectMagnetFiles(id: number, metadata: Metadata) {
  const priorities = await openDialog("files", { metadata });
  await attempt(() => api.completeFileSelection(id, priorities));
}
