<script lang="ts">
  import { ListChecks, ListX } from "@lucide/svelte";
  import type { Metadata } from "../api";
  import { formatSize } from "../format";
  import { t } from "../i18n.svelte";
  import FileTree from "./FileTree.svelte";
  import Modal from "./Modal.svelte";

  interface Props {
    metadata: Metadata;
    priorities?: number[];
    done: (priorities: number[] | null) => void;
  }
  let { metadata, priorities, done }: Props = $props();

  // The dialog edits its own copy: initial values only, on purpose
  // svelte-ignore state_referenced_locally
  let checked = $state(metadata.files.map((f) => !priorities || (priorities[f.index] ?? 4) > 0));
  const selectedSize = $derived(metadata.files.reduce((sum, f) => sum + (checked[f.index] ? f.size : 0), 0));
  const count = $derived(checked.filter(Boolean).length);
  // Normal priority (4) for checked files, skip (0) for the rest, like the other versions
  const submit = () => count > 0 && done(checked.map((c) => (c ? 4 : 0)));
</script>

<Modal title={t("select_files")} wide oncancel={() => done(null)}>
  <div class="head">
    <div class="name ellipsis" title={metadata.name}>{metadata.name}</div>
    <div class="muted small">{formatSize(selectedSize)} / {formatSize(metadata.total_size)} · {count} / {metadata.files.length}</div>
  </div>
  <FileTree files={metadata.files} bind:checked />
  {#snippet footer()}
    <div class="select-tools">
      <button class="btn icon" title={t("select_all")} aria-label={t("select_all")} onclick={() => (checked = checked.map(() => true))}>
        <ListChecks size={17} />
      </button>
      <button class="btn icon" title={t("select_none")} aria-label={t("select_none")} onclick={() => (checked = checked.map(() => false))}>
        <ListX size={17} />
      </button>
    </div>
    <button class="btn" onclick={() => done(null)}>{t("cancel_button")}</button>
    <button class="btn primary" disabled={count === 0} onclick={submit}>{t("add")}</button>
  {/snippet}
</Modal>

<style>
  .head {
    margin-bottom: 10px;
  }
  .name {
    font-weight: 600;
  }
  /* Select all / none sit on the left of the footer, the actions on the right */
  .select-tools {
    display: flex;
    gap: 4px;
    margin-right: auto;
  }
  .btn.icon {
    width: 32px;
    padding: 0;
  }
</style>
