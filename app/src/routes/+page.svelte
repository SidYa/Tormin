<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { fade, fly } from "svelte/transition";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { CircleAlert, FileDown } from "@lucide/svelte";
  import { fitAccent } from "$lib/accent";
  import * as actions from "$lib/actions";
  import { api, type Metadata, type Snapshot } from "$lib/api";
  import { readText } from "$lib/clipboard";
  import { isMagnet } from "$lib/format";
  import { setLanguage, t } from "$lib/i18n.svelte";
  import { app, applySnapshot, dialogs, notify, openDialog, selection } from "$lib/store.svelte";
  import Dialogs from "$lib/components/Dialogs.svelte";
  import StatusBar from "$lib/components/StatusBar.svelte";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import TorrentList from "$lib/components/TorrentList.svelte";

  let dragging = $state(false);
  let ready = $state(false);

  // Theme: explicit choice or follow the OS
  const systemDark = window.matchMedia("(prefers-color-scheme: dark)");
  let prefersDark = $state(systemDark.matches);
  systemDark.addEventListener("change", (e) => (prefersDark = e.matches));
  const dark = $derived((app.config?.theme ?? "system") === "system" ? prefersDark : app.config?.theme === "dark");
  $effect(() => {
    document.documentElement.dataset.theme = dark ? "dark" : "light";
  });
  // Accent: a preset name or a custom #rrggbb; monochrome replaces it with gray
  $effect(() => {
    const root = document.documentElement;
    const accent = app.config?.monochrome ? "mono" : (app.config?.accent ?? "purple");
    const custom = accent.startsWith("#");
    if (accent === "purple") delete root.dataset.accent;
    else root.dataset.accent = custom ? "custom" : accent;
    if (custom) {
      const fit = fitAccent(accent, dark);
      root.style.setProperty("--accent", fit.accent);
      root.style.setProperty("--accent-text", fit.text);
    } else {
      root.style.removeProperty("--accent");
      root.style.removeProperty("--accent-text");
    }
  });

  async function takePendingOpen() {
    for (const req of await api.takePendingOpen()) await actions.handleOpenRequest(req);
  }

  onMount(() => {
    const unlisten: Promise<() => void>[] = [];

    (async () => {
      const state = await api.getState();
      app.config = state.config;
      app.platform = state.platform;
      app.version = state.version;
      app.libtorrent = state.libtorrent;
      app.error = state.error;
      document.documentElement.classList.add(`platform-${state.platform}`);
      setLanguage(state.config.language);
      if (state.snapshot) applySnapshot(state.snapshot);
      ready = true;
      await takePendingOpen();
    })();

    unlisten.push(
      listen<Snapshot>("torrents", (e) => applySnapshot(e.payload)),
      listen("open-pending", () => takePendingOpen()),
      listen<{ id: number; metadata: Metadata }>("select-files", (e) =>
        actions.selectMagnetFiles(e.payload.id, e.payload.metadata),
      ),
      listen<string>("error", (e) => notify(e.payload, "error")),
      getCurrentWebview().onDragDropEvent(async (e) => {
        if (e.payload.type === "enter" || e.payload.type === "over") dragging = true;
        else if (e.payload.type === "leave") dragging = false;
        else if (e.payload.type === "drop") {
          dragging = false;
          for (const path of e.payload.paths)
            if (path.toLowerCase().endsWith(".torrent")) await actions.addTorrentFile(path);
        }
      }),
    );

    return () => unlisten.forEach((p) => p.then((f) => f()));
  });

  function onkeydown(e: KeyboardEvent) {
    // Shortcuts only when no dialog is open and the user is not typing
    if (dialogs.length || e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
    const ctrl = e.ctrlKey || e.metaKey;
    const key = e.key.toLowerCase();
    if (ctrl && key === "o") actions.pickTorrentFiles();
    else if (ctrl && key === "m") actions.askMagnet();
    else if (ctrl && key === "q") api.quit();
    else if (ctrl && key === ",") openDialog("settings", {});
    else if (ctrl && key === "a") app.torrents.forEach((t) => selection.add(t.id));
    else if (e.key === "Delete") actions.deleteSelected();
    else if (e.key === "Escape") selection.clear();
    else if (e.key === "F5" || (ctrl && key === "r")) {
      /* no page reloads in an app */
    } else return;
    e.preventDefault();
  }

  // Ctrl+V anywhere adds a magnet link from the clipboard
  async function onpaste(e: ClipboardEvent) {
    if (dialogs.length || e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
    const text = (e.clipboardData?.getData("text") || (await readText())).trim();
    if (isMagnet(text)) {
      e.preventDefault();
      await actions.addMagnet(text);
    }
  }
</script>

<svelte:window {onkeydown} {onpaste} oncontextmenu={(e) => e.preventDefault()} />

<div class="app">
  <TitleBar />
  <main>
    {#if app.error}
      <div class="fatal" in:fade>
        <CircleAlert size={28} />
        <div>libtorrent</div>
        <div class="muted small">{app.error}</div>
      </div>
    {:else if ready}
      <TorrentList />
    {/if}
  </main>
  <StatusBar />
</div>

{#if dragging}
  <div class="drop" transition:fade={{ duration: 120 }}>
    <div class="drop-card" in:fly={{ y: 8, duration: 180 }}>
      <FileDown size={34} strokeWidth={1.6} />
      <div>{t("drop_here")}</div>
    </div>
  </div>
{/if}

{#if app.notice}
  {#key app.notice}
    <div class="notice" class:error={app.notice.kind === "error"} transition:fly={{ y: 16, duration: 200 }}>
      {app.notice.text}
    </div>
  {/key}
{/if}

<Dialogs />

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }
  main {
    flex: 1;
    min-height: 0;
  }
  .fatal {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 100%;
    padding: 24px;
    text-align: center;
    color: var(--red);
  }
  .drop {
    position: fixed;
    inset: 46px 8px 8px;
    z-index: 30;
    display: grid;
    place-items: center;
    border: 2px dashed var(--accent);
    border-radius: var(--radius);
    background: var(--accent-soft);
    backdrop-filter: blur(2px);
    pointer-events: none;
  }
  .drop-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    color: var(--accent);
    font-weight: 600;
  }
  .notice {
    position: fixed;
    left: 12px;
    right: 12px;
    bottom: 40px;
    z-index: 60;
    padding: 10px 14px;
    border-radius: 10px;
    background: var(--bg-elevated);
    box-shadow: var(--shadow);
    font-size: 13px;
    word-break: break-word;
  }
  .notice.error {
    border-left: 3px solid var(--red);
  }
</style>
