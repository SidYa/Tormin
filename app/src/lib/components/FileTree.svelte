<script lang="ts" module>
  import type { FileEntry } from "../api";

  interface Node {
    name: string;
    size: number;
    /** File index, -1 for folders */
    index: number;
    /** Indices of every file below (or the file itself) */
    files: number[];
    children: Node[];
  }

  function buildTree(entries: FileEntry[]): Node[] {
    const root: Node = { name: "", size: 0, index: -1, files: [], children: [] };
    for (const entry of entries) {
      const parts = entry.path.split("/");
      let parent = root;
      parent.files.push(entry.index);
      for (let i = 0; i < parts.length - 1; i++) {
        let folder = parent.children.find((c) => c.index === -1 && c.name === parts[i]);
        if (!folder) {
          folder = { name: parts[i], size: 0, index: -1, files: [], children: [] };
          parent.children.push(folder);
        }
        folder.size += entry.size;
        folder.files.push(entry.index);
        parent = folder;
      }
      parent.children.push({ name: parts.at(-1)!, size: entry.size, index: entry.index, files: [entry.index], children: [] });
    }
    // Folders first, then by name, like a file manager
    const sort = (nodes: Node[]) => {
      nodes.sort((a, b) => Number(b.index === -1) - Number(a.index === -1) || a.name.localeCompare(b.name));
      nodes.forEach((n) => sort(n.children));
    };
    sort(root.children);
    return root.children;
  }
</script>

<script lang="ts">
  import { ChevronRight, File, Folder } from "@lucide/svelte";
  import { formatSize } from "../format";

  interface Props {
    files: FileEntry[];
    /** checked[i] = download file i */
    checked: boolean[];
  }
  let { files, checked = $bindable() }: Props = $props();

  const tree = $derived(buildTree(files));
  // Folders the user collapsed; everything starts expanded
  let collapsed = $state(new Set<Node>());

  function stateOf(node: Node): "on" | "off" | "mixed" {
    let on = 0;
    for (const i of node.files) if (checked[i]) on++;
    return on === 0 ? "off" : on === node.files.length ? "on" : "mixed";
  }

  function toggle(node: Node) {
    const value = stateOf(node) !== "on";
    const next = [...checked];
    for (const i of node.files) next[i] = value;
    checked = next;
  }

  function toggleOpen(node: Node) {
    const next = new Set(collapsed);
    if (next.has(node)) next.delete(node);
    else next.add(node);
    collapsed = next;
  }
</script>

{#snippet branch(nodes: Node[], depth: number)}
  {#each nodes as node (node)}
    {@const s = stateOf(node)}
    <div class="item" style:padding-left="{depth * 16 + 4}px">
      {#if node.index === -1}
        <button class="chevron" class:open={!collapsed.has(node)} aria-label="Expand" onclick={() => toggleOpen(node)}>
          <ChevronRight size={14} />
        </button>
      {:else}
        <span class="chevron"></span>
      {/if}
      <label class="check">
        <input
          type="checkbox"
          checked={s === "on"}
          indeterminate={s === "mixed"}
          onchange={() => toggle(node)}
        />
        <span class="icon">
          {#if node.index === -1}<Folder size={15} />{:else}<File size={15} />{/if}
        </span>
        <span class="name ellipsis" title={node.name}>{node.name}</span>
        <span class="size">{formatSize(node.size)}</span>
      </label>
    </div>
    {#if node.children.length && !collapsed.has(node)}
      {@render branch(node.children, depth + 1)}
    {/if}
  {/each}
{/snippet}

<div class="tree">
  {@render branch(tree, 0)}
</div>

<style>
  .tree {
    border: 1px solid var(--border);
    border-radius: 9px;
    padding: 4px;
    background: var(--surface);
    max-height: 320px;
    overflow: auto;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 2px;
    border-radius: 6px;
  }
  .item:hover {
    background: var(--surface-hover);
  }
  .chevron {
    display: grid;
    place-items: center;
    flex-shrink: 0;
    width: 20px;
    height: 20px;
    border: none;
    background: none;
    padding: 0;
    color: var(--text-3);
    transition: transform 0.15s;
  }
  .chevron.open {
    transform: rotate(90deg);
  }
  .check {
    display: flex;
    align-items: center;
    gap: 7px;
    flex: 1;
    min-width: 0;
    height: 30px;
    padding-right: 8px;
  }
  input {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    margin: 0;
    accent-color: var(--accent);
  }
  .icon {
    display: inline-flex;
    color: var(--text-3);
  }
  .name {
    flex: 1;
    font-size: 13px;
  }
  .size {
    font-size: 12px;
    color: var(--text-2);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
</style>
