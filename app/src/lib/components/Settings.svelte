<!-- Preferences: every change is applied and saved immediately, as in the other versions. -->
<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { FolderOpen, Link } from "@lucide/svelte";
  import { api, type Config } from "../api";
  import { languages, setLanguage, t } from "../i18n.svelte";
  import { app, notify } from "../store.svelte";
  import Modal from "./Modal.svelte";
  import Select from "./Select.svelte";

  interface Props {
    onclose: () => void;
  }
  let { onclose }: Props = $props();

  const config = $derived(app.config!);
  const LANGUAGE_NAMES: Record<string, string> = { en: "English", uk: "Українська" };
  const ACCENTS = ["purple", "green", "orange", "blue"];
  const custom = $derived(config.accent.startsWith("#"));

  async function update<K extends keyof Config>(key: K, value: Config[K]) {
    const next = { ...config, [key]: value };
    app.config = next;
    if (key === "language") setLanguage(value as string);
    try {
      await api.setConfig(next);
    } catch (e) {
      notify(String(e), "error");
    }
  }

  async function chooseFolder() {
    const dir = await open({ directory: true, defaultPath: config.download_dir });
    if (typeof dir === "string") await update("download_dir", dir);
  }

  async function associate() {
    try {
      await api.associateFiles();
      notify(t("associate_success"));
    } catch (e) {
      notify(String(e), "error");
    }
  }
</script>

{#snippet toggle(key: keyof Config, title: string, description?: string)}
  <label class="row">
    <div class="label">
      <div>{title}</div>
      {#if description}<div class="desc">{description}</div>{/if}
    </div>
    <input
      type="checkbox"
      class="switch"
      checked={config[key] as boolean}
      onchange={(e) => update(key, e.currentTarget.checked as never)}
    />
  </label>
{/snippet}

<Modal title={t("preferences")} wide oncancel={onclose}>
  <section>
    <h3>{t("downloads")}</h3>
    <div class="card">
      <div class="row">
        <div class="label">
          <div>{t("default_save_path")}</div>
          <div class="desc ellipsis" title={config.download_dir}>{config.download_dir}</div>
        </div>
        <button class="btn" onclick={chooseFolder}><FolderOpen size={15} />{t("choose")}</button>
      </div>
      <div class="row">
        <div class="label">
          <div>{t("file_association")}</div>
          <div class="desc">{t("associate_desc")}</div>
        </div>
        <button class="btn" onclick={associate}><Link size={15} />{t("associate_btn")}</button>
      </div>
      {@render toggle("shutdown_complete", t("shutdown_complete"), t("shutdown_complete_desc"))}
    </div>
  </section>

  <section>
    <h3>{t("appearance")}</h3>
    <div class="card">
      <div class="row">
        <div class="label">
          <div>{t("theme")}</div>
          <div class="desc">{t("theme_desc")}</div>
        </div>
        <div class="segmented">
          {#each [["system", t("theme_system")], ["light", t("theme_light")], ["dark", t("theme_dark")]] as [value, label] (value)}
            <button class:active={config.theme === value} onclick={() => update("theme", value as Config["theme"])}>{label}</button>
          {/each}
        </div>
      </div>
      <div class="row">
        <div class="label">
          <div>{t("accent_color")}</div>
          <div class="desc">{t("accent_desc")}</div>
        </div>
        <div class="swatches" class:off={config.monochrome} title={config.monochrome ? t("accent_mono") : undefined}>
          {#each ACCENTS as value (value)}
            <button
              class="swatch"
              class:active={config.accent === value}
              style:--swatch="var(--accent-{value})"
              title={t(`accent_${value}`)}
              aria-label={t(`accent_${value}`)}
              onclick={() => update("accent", value)}
            ></button>
          {/each}
          <label
            class="swatch custom"
            class:active={custom}
            style:--swatch={custom ? config.accent : undefined}
            title={t("accent_custom")}
          >
            <input
              type="color"
              value={custom ? config.accent : "#8f7dff"}
              onchange={(e) => update("accent", e.currentTarget.value)}
            />
          </label>
        </div>
      </div>
      {@render toggle("monochrome", t("monochrome"), t("monochrome_desc"))}
      <div class="row">
        <div class="label">
          <div>{t("progress_style")}</div>
          <div class="desc">{t("progress_desc")}</div>
        </div>
        <Select
          value={config.progress_style}
          options={[
            { value: 1, label: t("progress_text") },
            { value: 2, label: t("progress_circle_text") },
            { value: 3, label: t("progress_circle_clock") },
          ]}
          onchange={(v) => update("progress_style", v as Config["progress_style"])}
        />
      </div>
      {@render toggle("compact_mode", t("compact_mode"), t("compact_desc"))}
      {@render toggle("compact_window_size", t("compact_window"), t("compact_window_desc"))}
      <div class="row">
        <div class="label"><div>{t("language")}</div></div>
        <Select
          value={config.language}
          options={languages.map((code) => ({ value: code, label: LANGUAGE_NAMES[code] ?? code.toUpperCase() }))}
          onchange={(v) => update("language", v)}
        />
      </div>
    </div>
  </section>

  <section>
    <h3>{app.platform === "windows" ? "Windows" : "System"}</h3>
    <div class="card">
      {@render toggle("start_with_windows", t("start_with_windows"))}
      {@render toggle("start_minimized", t("start_minimized"))}
    </div>
  </section>

  {#snippet footer()}
    <button class="btn primary" onclick={onclose}>{t("close")}</button>
  {/snippet}
</Modal>

<style>
  section + section {
    margin-top: 18px;
  }
  h3 {
    margin: 0 0 8px 2px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-2);
  }
  .card {
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    overflow: hidden;
  }
  /* Controls move below their label when the window is too narrow for both */
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 12px;
    min-height: 52px;
    padding: 9px 12px;
  }
  .row + .row {
    border-top: 1px solid var(--border);
  }
  .label {
    flex: 1 1 170px;
    min-width: 0;
  }
  .desc {
    margin-top: 1px;
    font-size: 12px;
    color: var(--text-2);
  }
  @media (max-width: 460px) {
    .desc {
      font-size: 11.5px;
    }
    .segmented button {
      padding: 0 8px;
    }
  }
  .segmented {
    display: flex;
    flex-shrink: 0;
    padding: 2px;
    border-radius: 8px;
    background: var(--surface-active);
  }
  .segmented button {
    height: 28px;
    padding: 0 10px;
    border: none;
    border-radius: 6px;
    background: none;
    color: var(--text-2);
    font-size: 12.5px;
    transition: background 0.15s, color 0.15s;
  }
  .segmented button.active {
    background: var(--bg-elevated);
    color: var(--text);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
  }

  .swatches {
    display: flex;
    flex-shrink: 0;
    gap: 8px;
    padding: 3px;
  }
  /* Monochrome overrides the accent: the colors stay selectable for later */
  .swatches.off {
    opacity: 0.4;
  }
  /* Filled circle with a ring; the selected one gets a second ring in its own color */
  .swatch {
    position: relative;
    width: 22px;
    height: 22px;
    padding: 0;
    border: 2px solid var(--border-strong);
    border-radius: 50%;
    background: var(--swatch);
    background-clip: padding-box;
    transition: transform 0.12s, box-shadow 0.15s;
  }
  .swatch:hover {
    transform: scale(1.12);
  }
  .swatch.active {
    border-color: var(--bg-elevated);
    box-shadow: 0 0 0 2px var(--swatch);
  }
  .swatch.custom:not(.active) {
    background: conic-gradient(#ff5c5c, #f0c419, #22b573, #2fa8e8, #8f7dff, #ff5c9d, #ff5c5c);
  }
  .swatch.custom input {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    opacity: 0;
    cursor: pointer;
  }

  /* Fluent-style toggle switch */
  .switch {
    appearance: none;
    position: relative;
    flex-shrink: 0;
    width: 40px;
    height: 22px;
    margin: 0;
    border-radius: 11px;
    border: 1px solid var(--text-2);
    background: transparent;
    transition: background 0.18s, border-color 0.18s;
  }
  .switch::after {
    content: "";
    position: absolute;
    top: 4px;
    left: 4px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--text-2);
    transition: transform 0.18s var(--ease), background 0.18s, width 0.18s;
  }
  .switch:checked {
    background: var(--accent);
    border-color: var(--accent);
  }
  .switch:checked::after {
    transform: translateX(18px);
    background: var(--accent-text);
  }
  .switch:active::after {
    width: 15px;
  }
</style>
