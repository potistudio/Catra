<script lang="ts">
import TrackCard from "$lib/components/TrackCard.svelte";
import type { Track } from "$lib/types";
import { isInRekordbox } from "$lib/rekordboxMembership";
import {
	getGridColumnCount,
	getVisibleGridRange,
	TRACK_GRID_GAP,
	TRACK_GRID_ROW_HEIGHT,
	type TrackListRow,
} from "$lib/trackListView";

interface Props {
	rows: TrackListRow[];
	selectedId: number | null;
	checkedKeys?: Set<number>;
	readonly?: boolean;
	rekordboxPathIndex?: Map<string, string>;
	rekordboxContentIds?: Set<string>;
	showRowRemove?: boolean;
	onselect: (row: TrackListRow) => void;
	onremove?: (track: Track) => void;
	onplay?: (track: Track) => void;
}

let {
	rows,
	selectedId,
	checkedKeys = new Set(),
	readonly = false,
	rekordboxPathIndex,
	rekordboxContentIds,
	showRowRemove = true,
	onselect,
	onremove,
	onplay,
}: Props = $props();

let scrollTop = $state(0);
let viewportHeight = $state(0);
let containerWidth = $state(0);

let gridWrap = $state<HTMLDivElement | null>(null);

$effect(() => {
	const element = gridWrap;
	if (!element) return;

	const observer = new ResizeObserver(([entry]) => {
		viewportHeight = entry.contentRect.height;
		containerWidth = entry.contentRect.width;
	});

	observer.observe(element);
	viewportHeight = element.clientHeight;
	containerWidth = element.clientWidth;

	return () => observer.disconnect();
});

let columnCount = $derived(getGridColumnCount(containerWidth));
let visibleRange = $derived(
	getVisibleGridRange(scrollTop, viewportHeight, columnCount, rows.length),
);
let visibleRows = $derived(rows.slice(visibleRange.start, visibleRange.end));
let rowCount = $derived(Math.ceil(rows.length / columnCount));
let totalBodyHeight = $derived(rowCount * TRACK_GRID_ROW_HEIGHT);
let bodyOffsetY = $derived(visibleRange.startRow * TRACK_GRID_ROW_HEIGHT);

function handleScroll(event: Event) {
	scrollTop = (event.currentTarget as HTMLDivElement).scrollTop;
}
</script>

<div
  class="grid-wrap"
  bind:this={gridWrap}
  onscroll={handleScroll}
  role="list"
  aria-label="Track library grid"
>
  <div class="virtual-body" style:height="{totalBodyHeight}px">
    <div
      class="virtual-window"
      style:transform="translateY({bodyOffsetY}px)"
      style:grid-template-columns="repeat({columnCount}, minmax(0, 1fr))"
      style:gap="{TRACK_GRID_GAP}px"
    >
      {#each visibleRows as row (row.key)}
        <TrackCard
          track={row.track}
          selected={selectedId === row.track.id}
          checked={checkedKeys.has(row.key)}
          {readonly}
          inRekordbox={rekordboxPathIndex
            ? isInRekordbox(row.track, rekordboxPathIndex, rekordboxContentIds)
            : false}
          {showRowRemove}
          onselect={() => onselect(row)}
          {onremove}
          {onplay}
        />
      {/each}
    </div>
  </div>
</div>

<style>
  .grid-wrap {
    flex: 1;
    overflow: auto;
    min-height: 0;
    padding: 0.75rem 1rem;
  }

  .virtual-body {
    position: relative;
  }

  .virtual-window {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    display: grid;
    will-change: transform;
  }
</style>
