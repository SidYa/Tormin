<!-- Shows the first dialog of the queue in store.svelte.ts. -->
<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { Heart } from "@lucide/svelte";
  import { t } from "../i18n.svelte";
  import { app, dialogs, type OpenDialog } from "../store.svelte";
  import FilesDialog from "./FilesDialog.svelte";
  import MagnetDialog from "./MagnetDialog.svelte";
  import Modal from "./Modal.svelte";
  import PropertiesDialog from "./PropertiesDialog.svelte";
  import Settings from "./Settings.svelte";

  const DONATE_URL = "https://www.paypal.com/donate/?business=sergiisidko%40gmail.com&no_recurring=0&currency_code=USD";

  const current = $derived(dialogs[0] as OpenDialog | undefined);
  // Narrowing helper: the dialog's kind decides its props and result type
  const as = <K extends OpenDialog["kind"]>(d: OpenDialog, _kind: K) => d as unknown as OpenDialog<K>;
</script>

{#if current}
  {#key current.id}
    {#if current.kind === "confirm"}
      {@const d = as(current, "confirm")}
      <Modal title={d.props.title} oncancel={() => d.resolve(false)} onconfirm={() => d.resolve(true)}>
        <p class="text">{d.props.body}</p>
        {#snippet footer()}
          <button class="btn" onclick={() => d.resolve(false)}>{t("cancel")}</button>
          <button class="btn primary" onclick={() => d.resolve(true)}>{d.props.action}</button>
        {/snippet}
      </Modal>
    {:else if current.kind === "delete"}
      {@const d = as(current, "delete")}
      {@const count = d.props.torrents.length}
      <Modal title={count === 1 ? t("delete_h") : t("delete_h_plural", count)} oncancel={() => d.resolve("cancel")}>
        <p class="text">{count === 1 ? t("delete_b", d.props.torrents[0].name) : t("delete_b_plural", count)}</p>
        <p class="text hint">{t("delete_hint")}</p>
        {#snippet footer()}
          <!-- Oops = cancel, No = list only, Yes = list + downloaded files + .torrent file -->
          <button class="btn" onclick={() => d.resolve("cancel")}>{t("oops")}</button>
          <button class="btn" onclick={() => d.resolve("list")}>{t("no_list_only")}</button>
          <button class="btn danger" onclick={() => d.resolve("files")}>{t("yes_delete")}</button>
        {/snippet}
      </Modal>
    {:else if current.kind === "magnet"}
      {@const d = as(current, "magnet")}
      <MagnetDialog initial={d.props.initial} done={d.resolve} />
    {:else if current.kind === "files"}
      {@const d = as(current, "files")}
      <FilesDialog metadata={d.props.metadata} priorities={d.props.priorities} done={d.resolve} />
    {:else if current.kind === "properties"}
      {@const d = as(current, "properties")}
      <PropertiesDialog details={d.props.details} done={d.resolve} />
    {:else if current.kind === "message"}
      {@const d = as(current, "message")}
      <Modal title={d.props.title} oncancel={() => d.resolve()} onconfirm={() => d.resolve()}>
        <p class="text selectable">{d.props.body}</p>
        {#snippet footer()}
          <button class="btn primary" onclick={() => d.resolve()}>{t("close")}</button>
        {/snippet}
      </Modal>
    {:else if current.kind === "settings"}
      {@const d = as(current, "settings")}
      <Settings onclose={() => d.resolve()} />
    {:else if current.kind === "about"}
      {@const d = as(current, "about")}
      <Modal title={t("about")} oncancel={() => d.resolve()} onconfirm={() => d.resolve()}>
        <div class="about">
          <img src="/favicon.png" alt="" width="64" height="64" />
          <div class="about-name">Tormin</div>
          <div class="muted">{app.version}</div>
          <div class="muted small">Minimalist BitTorrent client</div>
          <button class="paypal" onclick={() => openUrl(DONATE_URL)}><Heart size={15} />{t("donate_paypal")}</button>
        </div>
        {#snippet footer()}
          <button class="btn primary" onclick={() => d.resolve()}>{t("close")}</button>
        {/snippet}
      </Modal>
    {/if}
  {/key}
{/if}

<style>
  .text {
    margin: 0;
    color: var(--text-2);
    word-break: break-word;
  }
  .hint {
    margin-top: 8px;
    font-size: 12px;
    color: var(--text-3);
  }
  .selectable {
    user-select: text;
    -webkit-user-select: text;
  }
  .about {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 8px 0;
    text-align: center;
  }
  .paypal {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 34px;
    margin-top: 12px;
    padding: 0 18px;
    border: none;
    border-radius: 17px;
    background: var(--paypal);
    color: var(--paypal-text);
    font-weight: 600;
    transition: background 0.15s, transform 0.1s;
  }
  .paypal:hover {
    background: var(--paypal-hover);
  }
  .paypal:active {
    transform: scale(0.97);
  }
  .about-name {
    margin-top: 6px;
    font-size: 20px;
    font-weight: 700;
  }
</style>
