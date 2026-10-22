<!-- Custom title bar: every action lives here; empty space drags the window. -->
<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import {
    Ellipsis,
    FilePlus,
    FolderOpen,
    Info,
    Magnet,
    Minus,
    Pause,
    Play,
    Plus,
    Power,
    Settings,
    SlidersHorizontal,
    Trash2,
    X,
  } from "@lucide/svelte";
  import * as actions from "../actions";
  import { api } from "../api";
  import { t } from "../i18n.svelte";
  import { app, openDialog, selection } from "../store.svelte";
  import Menu, { type MenuItem } from "./Menu.svelte";

  const win = getCurrentWindow();
  let menu = $state<{ items: MenuItem[]; x: number; y: number; align: "left" | "right" } | null>(null);

  const hasTorrents = $derived(app.torrents.length > 0);
  const hasSelection = $derived(selection.size > 0);

  function openMenu(e: MouseEvent, items: MenuItem[], align: "left" | "right") {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    menu = { items, x: align === "left" ? rect.left : rect.right, y: rect.bottom + 4, align };
  }

  const addItems = (): MenuItem[] => [
    { label: t("add_torrent_btn"), icon: FilePlus, shortcut: "Ctrl+O", action: actions.pickTorrentFiles },
    { label: t("add_magnet_menu"), icon: Magnet, shortcut: "Ctrl+M", action: actions.askMagnet },
  ];

  const moreItems = (): MenuItem[] => [
    { label: t("torrent_props"), icon: SlidersHorizontal, disabled: selection.size !== 1, action: () => actions.showProperties() },
    { label: t("open_folder"), icon: FolderOpen, disabled: !hasSelection, action: actions.openFolders },
    { separator: true },
    { label: t("preferences"), icon: Settings, shortcut: "Ctrl+,", action: () => openDialog("settings", {}) },
    { label: t("about"), icon: Info, action: () => openDialog("about", {}) },
    { separator: true },
    { label: t("quit"), icon: Power, shortcut: "Ctrl+Q", action: () => api.quit() },
  ];
</script>

<header class="bar" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <img src="/favicon.png" alt="" width="20" height="20" data-tauri-drag-region />
    <span class="title" data-tauri-drag-region>Tormin</span>
  </div>

  <div class="actions">
    <button class="tool" title={t("add_torrent_tooltip")} onclick={(e) => openMenu(e, addItems(), "left")}>
      <Plus size={18} strokeWidth={2.2} />
    </button>
    <button class="tool" title={t("resume")} disabled={!hasTorrents} onclick={actions.resumeSelected}>
      <Play size={16} />
    </button>
    <button class="tool" title={t("pause")} disabled={!hasTorrents} onclick={actions.pauseSelected}>
      <Pause size={16} />
    </button>
    <button class="tool" title={t("delete")} disabled={!hasSelection} onclick={actions.deleteSelected}>
      <Trash2 size={16} />
    </button>
    <button class="tool" title={t("preferences")} onclick={(e) => openMenu(e, moreItems(), "right")}>
      <Ellipsis size={18} />
    </button>
  </div>

  <div class="window">
    <button class="win" title={t("minimize")} aria-label={t("minimize")} onclick={() => win.minimize()}><Minus size={16} /></button>
    <button class="win close" title={t("close")} aria-label={t("close")} onclick={() => win.close()}><X size={16} /></button>
  </div>
</header>

{#if menu}
  <Menu items={menu.items} x={menu.x} y={menu.y} align={menu.align} onclose={() => (menu = null)} />
{/if}

<style>
  .bar {
    display: flex;
    align-items: center;
    height: 46px;
    padding-left: 12px;
    flex-shrink: 0;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
    height: 100%;
  }
  .brand img {
    flex-shrink: 0;
  }
  .title {
    overflow: hidden;
    white-space: nowrap;
    font-weight: 600;
    font-size: 14px;
    letter-spacing: 0.2px;
  }
  /* Very narrow window: the logo alone is enough */
  @media (max-width: 350px) {
    .title {
      display: none;
    }
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 2px;
    padding-right: 6px;
  }
  .tool {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border: none;
    border-radius: 8px;
    background: none;
    color: var(--text-2);
    transition: background 0.15s, color 0.15s, transform 0.1s;
  }
  .tool:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .tool:active {
    transform: scale(0.92);
  }
  .tool:disabled {
    opacity: 0.35;
    pointer-events: none;
  }
  .window {
    display: flex;
    align-self: stretch;
  }
  .win {
    display: grid;
    place-items: center;
    width: 40px;
    border: none;
    background: none;
    color: var(--text-2);
    transition: background 0.12s, color 0.12s;
  }
  .win:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .win.close:hover {
    background: #e81123;
    color: #fff;
  }
</style>
