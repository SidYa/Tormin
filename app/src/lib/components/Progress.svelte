<!-- Progress indicator with the three styles of the Linux version:
     1 text, 2 ring + number, 3 "clock" (pie). Plus paused and finished states. -->
<script lang="ts">
  interface Props {
    progress: number;
    finished: boolean;
    paused: boolean;
    variant: 1 | 2 | 3;
    monochrome: boolean;
    compact: boolean;
  }
  let { progress, finished, paused, variant, monochrome, compact }: Props = $props();

  const size = $derived(compact ? 20 : 38);
  const r = $derived(size / 2 - (compact ? 2 : 3));
  const c = $derived(size / 2);
  const stroke = $derived(compact ? 2 : 3);
  const circumference = $derived(2 * Math.PI * r);
  const value = $derived(finished ? 1 : Math.min(Math.max(progress, 0), 1));
  const pct = $derived(Math.floor(value * 100));
  const color = $derived(monochrome ? "var(--text)" : variant === 2 ? "var(--blue)" : "var(--accent)");
  const done = $derived(monochrome ? "var(--text)" : "var(--green)");

  // Pie slice for the clock style
  const pie = $derived.by(() => {
    if (value <= 0 || value >= 0.999) return "";
    const angle = value * 2 * Math.PI - Math.PI / 2;
    const x = c + r * Math.cos(angle);
    const y = c + r * Math.sin(angle);
    return `M ${c} ${c} L ${c} ${c - r} A ${r} ${r} 0 ${value > 0.5 ? 1 : 0} 1 ${x} ${y} Z`;
  });

  const check = $derived(
    `M ${c - r * 0.36} ${c + r * 0.02} L ${c - r * 0.08} ${c + r * 0.3} L ${c + r * 0.4} ${c - r * 0.3}`,
  );
</script>

{#if variant === 1 && !paused}
  <span class="text" class:compact class:finished style:color={finished ? done : "var(--text-2)"}>{pct}%</span>
{:else}
  <svg width={size} height={size} viewBox="0 0 {size} {size}" aria-label="{pct}%">
    {#if paused}
      <g stroke="var(--text-2)" stroke-width={stroke} stroke-linecap="round">
        <line x1={c - r * 0.28} y1={c - r * 0.4} x2={c - r * 0.28} y2={c + r * 0.4} />
        <line x1={c + r * 0.28} y1={c - r * 0.4} x2={c + r * 0.28} y2={c + r * 0.4} />
      </g>
      <circle cx={c} cy={c} r={r} fill="none" stroke="var(--text-3)" stroke-width="1" opacity="0.6" />
    {:else if variant === 2}
      <circle cx={c} cy={c} r={r} fill="none" stroke="var(--border-strong)" stroke-width={stroke} />
      <circle
        class="arc"
        cx={c}
        cy={c}
        r={r}
        fill="none"
        stroke={finished ? done : color}
        stroke-width={stroke}
        stroke-linecap="round"
        stroke-dasharray={circumference}
        stroke-dashoffset={circumference * (1 - value)}
        transform="rotate(-90 {c} {c})"
      />
      {#if finished}
        <path d={check} fill="none" stroke={done} stroke-width={stroke} stroke-linecap="round" stroke-linejoin="round" />
      {:else if !compact}
        <text x={c} y={c} dy="0.35em" text-anchor="middle" class="num">{pct}</text>
      {/if}
    {:else}
      {#if finished}
        <circle cx={c} cy={c} r={r} fill={done} />
        <path
          d={check}
          fill="none"
          stroke={monochrome ? "var(--bg-elevated)" : "#fff"}
          stroke-width={stroke}
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      {:else}
        <circle cx={c} cy={c} r={r} fill={color} opacity="0.2" />
        {#if value >= 0.999}
          <circle cx={c} cy={c} r={r} fill={color} />
        {:else if pie}
          <path d={pie} fill={color} />
        {/if}
      {/if}
    {/if}
  </svg>
{/if}

<style>
  svg {
    display: block;
    flex-shrink: 0;
  }
  .arc {
    transition: stroke-dashoffset 0.6s var(--ease);
  }
  .num {
    font-size: 10.5px;
    font-weight: 600;
    fill: var(--text);
  }
  .text {
    min-width: 38px;
    text-align: right;
    font-size: 13px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .text.compact {
    font-size: 12px;
  }
</style>
