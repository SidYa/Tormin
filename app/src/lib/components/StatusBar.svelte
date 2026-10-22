<script lang="ts">
  import { ArrowDown, ArrowUp } from "@lucide/svelte";
  import { formatSpeed } from "../format";
  import { t } from "../i18n.svelte";
  import { app } from "../store.svelte";
</script>

<footer class="status">
  <span class="rate" class:active={app.downloadRate > 0}>
    <ArrowDown size={13} strokeWidth={2.4} />
    {formatSpeed(app.downloadRate)}
  </span>
  <span class="rate up" class:active={app.uploadRate > 0}>
    <ArrowUp size={13} strokeWidth={2.4} />
    {formatSpeed(app.uploadRate)}
  </span>
  <span class="count">{t("torrents_count", app.torrents.length)}</span>
</footer>

<style>
  .status {
    display: flex;
    align-items: center;
    gap: 14px;
    height: 30px;
    padding: 0 14px;
    flex-shrink: 0;
    border-top: 1px solid var(--border);
    font-size: 12px;
    color: var(--text-3);
    font-variant-numeric: tabular-nums;
  }
  .rate {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    transition: color 0.2s;
  }
  .rate.active {
    color: var(--accent);
  }
  .rate.up.active {
    color: var(--green);
  }
  .count {
    margin-left: auto;
  }
</style>
