<script lang="ts">
  import { Magnet } from "@lucide/svelte";
  import { isMagnet } from "../format";
  import { t } from "../i18n.svelte";
  import Modal from "./Modal.svelte";

  interface Props {
    initial: string;
    done: (uri: string | null) => void;
  }
  let { initial, done }: Props = $props();

  // svelte-ignore state_referenced_locally
  let uri = $state(initial);
  const valid = $derived(isMagnet(uri));
  const submit = () => valid && done(uri.trim());
</script>

<Modal title={t("add_magnet_menu").replace(/(\.\.\.|…)$/, "")} oncancel={() => done(null)} onconfirm={submit}>
  <div class="field">
    <span class="icon"><Magnet size={16} /></span>
    <textarea class="input" rows="3" placeholder="magnet:?xt=urn:btih:…" spellcheck="false" bind:value={uri}></textarea>
  </div>
  {#snippet footer()}
    <button class="btn" onclick={() => done(null)}>{t("cancel")}</button>
    <button class="btn primary" disabled={!valid} onclick={submit}>{t("add")}</button>
  {/snippet}
</Modal>

<style>
  .field {
    position: relative;
  }
  .icon {
    position: absolute;
    left: 10px;
    top: 9px;
    color: var(--text-3);
  }
  textarea {
    height: auto;
    padding: 8px 10px 8px 34px;
    resize: none;
    font-size: 12.5px;
    word-break: break-all;
  }
</style>
