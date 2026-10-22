<script lang="ts">
  import { flip } from "svelte/animate";
  import { fly } from "svelte/transition";
  import { Download, FolderOpen, Pause, Play, SlidersHorizontal, Trash2 } from "@lucide/svelte";
  import * as actions from "../actions";
  import { t } from "../i18n.svelte";
  import { app, selection } from "../store.svelte";
  import Menu, { type MenuItem } from "./Menu.svelte";
  import TorrentRow from "./TorrentRow.svelte";

  let anchor: number | null = null;
  let menu = $state<{ x: number; y: number } | null>(null);

  function select(id: number, e: MouseEvent | KeyboardEvent) {
    const ids = app.torrents.map((t) => t.id);
    if (e.shiftKey && anchor !== null && ids.includes(anchor)) {
      const [a, b] = [ids.indexOf(anchor), ids.indexOf(id)].sort((x, y) => x - y);
      if (!e.ctrlKey) selection.clear();
      ids.slice(a, b + 1).forEach((i) => selection.add(i));
      return;
    }
    if (e.ctrlKey || e.metaKey) {
      if (selection.has(id)) selection.delete(id);
      else selection.add(id);
    } else {
      selection.clear();
      selection.add(id);
    }
    anchor = id;
  }

  function onContextMenu(id: number, e: MouseEvent) {
    e.preventDefault();
    // Right click on an unselected row selects only that row (as in the Linux version)
    if (!selection.has(id)) {
      selection.clear();
      selection.add(id);
      anchor = id;
    }
    menu = { x: e.clientX, y: e.clientY };
  }

  const menuItems = (): MenuItem[] => [
    { label: t("resume"), icon: Play, action: actions.resumeSelected },
    { label: t("pause"), icon: Pause, action: actions.pauseSelected },
    { label: t("open_folder"), icon: FolderOpen, action: actions.openFolders },
    { label: t("torrent_props"), icon: SlidersHorizontal, disabled: selection.size !== 1, action: () => actions.showProperties() },
    { separator: true },
    { label: t("delete"), icon: Trash2, shortcut: "Del", danger: true, action: actions.deleteSelected },
  ];

  function onkeydown(e: KeyboardEvent) {
    const ids = app.torrents.map((t) => t.id);
    if (!ids.length) return;
    const current = anchor !== null ? ids.indexOf(anchor) : -1;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      const next = Math.max(0, Math.min(ids.length - 1, current + (e.key === "ArrowDown" ? 1 : -1)));
      if (e.shiftKey && anchor !== null) {
        selection.add(ids[next]);
        anchor = ids[next];
      } else select(ids[next], e);
      document.getElementById(`row-${ids[next]}`)?.scrollIntoView({ block: "nearest" });
    } else if (e.key === "Enter") {
      actions.showProperties();
    } else return;
    e.preventDefault();
  }
</script>

{#if app.torrents.length === 0}
  <div class="empty" in:fly={{ y: 8, duration: 250 }}>
    <div class="empty-icon"><Download size={40} strokeWidth={1.5} /></div>
    <div class="empty-title">{t("no_torrents")}</div>
    <div class="empty-hint">{t("empty_hint")}</div>
  </div>
{:else}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div class="list" role="listbox" aria-multiselectable="true" tabindex="0" {onkeydown}>
    {#each app.torrents as torrent (torrent.id)}
      <div
        id="row-{torrent.id}"
        role="option"
        tabindex="-1"
        aria-selected={selection.has(torrent.id)}
        animate:flip={{ duration: 200 }}
        in:fly={{ y: 10, duration: 220 }}
        onclick={(e) => select(torrent.id, e)}
        ondblclick={() => actions.openFolders()}
        oncontextmenu={(e) => onContextMenu(torrent.id, e)}
        onkeydown={() => {}}
      >
        <TorrentRow {torrent} selected={selection.has(torrent.id)} />
      </div>
    {/each}
    <!-- Clicking the empty area below the rows clears the selection -->
    <div class="filler" role="presentation" onclick={() => selection.clear()}></div>
  </div>
{/if}

{#if menu}
  <Menu items={menuItems()} x={menu.x} y={menu.y} onclose={() => (menu = null)} />
{/if}

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    height: 100%;
    overflow-y: auto;
    padding: 2px 0 8px;
    outline: none;
  }
  .filler {
    flex: 1;
    min-height: 24px;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 100%;
    padding: 24px;
    text-align: center;
  }
  .empty-icon {
    display: grid;
    place-items: center;
    width: 84px;
    height: 84px;
    margin-bottom: 8px;
    border-radius: 24px;
    background: var(--accent-soft);
    color: var(--accent);
  }
  .empty-title {
    font-size: 17px;
    font-weight: 600;
  }
  .empty-hint {
    max-width: 240px;
    font-size: 13px;
    color: var(--text-2);
  }
</style>
