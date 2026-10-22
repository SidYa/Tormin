<script lang="ts">
  import type { Torrent } from "../api";
  import { formatEta, formatSize, formatSpeed } from "../format";
  import { t } from "../i18n.svelte";
  import { app } from "../store.svelte";
  import Progress from "./Progress.svelte";

  interface Props {
    torrent: Torrent;
    selected: boolean;
  }
  let { torrent, selected }: Props = $props();

  const compact = $derived(app.config?.compact_mode ?? false);

  const status = $derived.by(() => {
    const tr = torrent;
    const parts: string[] = [];
    if (tr.paused) parts.push(t("status_paused"));
    else if (tr.state === "metadata") parts.push(t("status_metadata"));
    else if (tr.state === "checking") parts.push(t("status_checking"));
    else if (tr.finished) parts.push(tr.upload_rate > 0 ? `▲ ${formatSpeed(tr.upload_rate)}` : t("status_done"));
    else {
      if (tr.download_rate > 0) parts.push(`▼ ${formatSpeed(tr.download_rate)}`);
      if (tr.upload_rate > 0) parts.push(`▲ ${formatSpeed(tr.upload_rate)}`);
    }
    if (tr.total > 0) {
      parts.push(tr.finished ? formatSize(tr.total) : `${formatSize(tr.done)} / ${formatSize(tr.total)}`);
      if (!tr.paused && !tr.finished && tr.download_rate > 0)
        parts.push(formatEta((tr.total - tr.done) / tr.download_rate));
    }
    if (!tr.paused && tr.peers > 0) parts.push(`${tr.peers} ${t("peers").toLowerCase()}`);
    return parts.join("  ·  ");
  });
</script>

<div class="row" class:selected class:compact class:paused={torrent.paused} title={compact ? status : undefined}>
  <div class="text">
    <div class="name ellipsis" title={torrent.name}>{torrent.name}</div>
    {#if !compact}
      <div class="status ellipsis">{status}</div>
    {/if}
  </div>
  <Progress
    progress={torrent.progress}
    finished={torrent.finished}
    paused={torrent.paused}
    variant={app.config?.progress_style ?? 3}
    monochrome={app.config?.monochrome ?? false}
    {compact}
  />
</div>

<style>
  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 0 6px;
    padding: 9px 10px 9px 12px;
    border-radius: 9px;
    transition: background 0.12s;
  }
  .row.compact {
    padding-top: 5px;
    padding-bottom: 5px;
  }
  .row:hover {
    background: var(--surface-hover);
  }
  .row.selected {
    background: var(--accent-soft);
  }
  .row.selected:hover {
    background: var(--accent-soft-strong);
  }
  /* Accent marker on the left of the selected row, like Fluent lists */
  .row.selected::before {
    content: "";
    position: absolute;
    left: 3px;
    top: 50%;
    width: 3px;
    height: 16px;
    margin-top: -8px;
    border-radius: 3px;
    background: var(--accent);
    animation: grow 0.18s var(--ease);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .name {
    font-weight: 500;
  }
  .compact .name {
    font-size: 13px;
  }
  .paused .name {
    color: var(--text-2);
  }
  .status {
    margin-top: 2px;
    font-size: 12px;
    color: var(--text-2);
    font-variant-numeric: tabular-nums;
  }
  @keyframes grow {
    from {
      transform: scaleY(0.2);
    }
  }
</style>
