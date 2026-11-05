<script lang="ts" module>
  /** Lane colours, in the order lanes appear (validated for the dark panel; more lanes reuse them, the lines keep them apart). */
  export const LANE_COLORS = ["#3987e5", "#d95926", "#199e70", "#c98500", "#d55181", "#008300", "#9085e9", "#e66767"];
  export const LANE = 12;
  export const ROW = 46;
  const PAD = 9;
  export const graphWidth = (lanes: number) => PAD * 2 + (Math.max(lanes, 1) - 1) * LANE;
</script>

<script lang="ts">
  // One history row's slice of the commit graph (graph.ts lays it out).
  import type { GraphRow, Segment } from "../lib/graph";

  let { row, width, merge, head }: { row: GraphRow; width: number; merge: boolean; head: boolean } = $props();

  const x = (lane: number) => PAD + lane * LANE;
  const mid = ROW / 2;
  const color = (i: number) => LANE_COLORS[i % LANE_COLORS.length];

  /** A straight line within a lane, or an S-curve when it changes lanes. */
  function path(s: Segment, y1: number, y2: number) {
    const [x1, x2] = [x(s.from), x(s.to)];
    if (x1 === x2) return `M${x1} ${y1}V${y2}`;
    const ym = (y1 + y2) / 2;
    return `M${x1} ${y1}C${x1} ${ym} ${x2} ${ym} ${x2} ${y2}`;
  }
</script>

<svg class="graph" {width} height={ROW} viewBox="0 0 {width} {ROW}" aria-hidden="true">
  {#each row.top as s, i (i)}
    <path d={path(s, 0, mid)} stroke={color(s.color)} />
  {/each}
  {#each row.bottom as s, i (i)}
    <path d={path(s, mid, ROW)} stroke={color(s.color)} />
  {/each}
  {#if head}
    <circle class="head" cx={x(row.col)} cy={mid} r="7" stroke={color(row.color)} />
  {/if}
  <!-- A ring of the panel colour keeps the dot clear of the lines; a merge is hollow. -->
  <circle cx={x(row.col)} cy={mid} r="4.5" class="dot" class:merge fill={merge ? "var(--panel)" : color(row.color)} stroke={merge ? color(row.color) : "var(--panel)"} />
</svg>

<style>
  .graph {
    flex: none;
    overflow: hidden;
  }
  path {
    fill: none;
    stroke-width: 2;
    stroke-linecap: round;
  }
  .dot {
    stroke-width: 2;
  }
  .head {
    fill: none;
    stroke-width: 1.5;
    opacity: 0.55;
  }
</style>
