<script lang="ts">
  import type { Details } from "../api";
  import { t } from "../i18n.svelte";
  import FileTree from "./FileTree.svelte";
  import Modal from "./Modal.svelte";

  interface Props {
    details: Details;
    done: (result: { limitKb: number; priorities: number[] | null } | null) => void;
  }
  let { details, done }: Props = $props();

  // The dialog edits its own copy: initial values only, on purpose
  // svelte-ignore state_referenced_locally
  const files = details.metadata && details.metadata.files.length > 1 ? details.metadata.files : null;
  // svelte-ignore state_referenced_locally
  let limit = $state(details.download_limit_kb || "");
  // svelte-ignore state_referenced_locally
  let checked = $state(files ? files.map((f) => (details.priorities[f.index] ?? 4) > 0) : []);

  function apply() {
    done({
      limitKb: Math.max(0, Math.floor(Number(limit) || 0)),
      priorities: files ? checked.map((c) => (c ? 4 : 0)) : null,
    });
  }
</script>

<Modal title={t("torrent_props")} wide oncancel={() => done(null)} onconfirm={apply}>
  <div class="name ellipsis" title={details.name}>{details.name}</div>
  <div class="muted small ellipsis" title={details.save_path}>{details.save_path}</div>

  <label class="field">
    <span>{t("download_speed_limit")}</span>
    <input class="input" type="number" min="0" step="100" placeholder={t("unlimited")} bind:value={limit} />
  </label>

  {#if files}
    <div class="section">{t("files")}</div>
    <FileTree {files} bind:checked />
  {/if}

  {#snippet footer()}
    <button class="btn" onclick={() => done(null)}>{t("cancel")}</button>
    <button class="btn primary" onclick={apply}>{t("apply")}</button>
  {/snippet}
</Modal>

<style>
  .name {
    font-weight: 600;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 16px 0 4px;
    font-size: 13px;
  }
  .section {
    margin: 16px 0 6px;
    font-weight: 600;
    font-size: 13px;
  }
</style>
