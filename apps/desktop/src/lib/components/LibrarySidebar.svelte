<script lang="ts">
import { onMount, type Snippet } from "svelte";
import { appSession, persistAppSession } from "$lib/appSession.svelte";
import {
	clampSidebarWidth,
	SIDEBAR_DEFAULT_WIDTH,
	SIDEBAR_MAX_WIDTH,
	SIDEBAR_MIN_WIDTH,
	type SidebarSection,
	sidebarMaxWidth,
} from "$lib/sidebarLayout";

interface Props {
	playlists: Snippet;
	tags: Snippet;
	facets: Snippet;
	activeTagCount: number;
	facetActive: boolean;
}

let { playlists, tags, facets, activeTagCount, facetActive }: Props = $props();

const contentId = $props.id();
let sidebar: HTMLElement;
let maxWidth = $state(SIDEBAR_MAX_WIDTH);
let drag = $state<{
	pointerId: number;
	startX: number;
	startWidth: number;
} | null>(null);
let layout = $derived(appSession.library.sidebar);
let width = $derived(clampSidebarWidth(layout.width, maxWidth));

onMount(() => {
	const container = sidebar.parentElement;
	if (!container) return;
	const observer = new ResizeObserver(([entry]) => {
		if (entry.contentRect.width > 0) {
			maxWidth = sidebarMaxWidth(entry.contentRect.width);
		}
	});
	observer.observe(container);
	return () => observer.disconnect();
});

function setWidth(value: number) {
	layout.width = clampSidebarWidth(value, maxWidth);
	persistAppSession();
}

function setOpen(section: SidebarSection, open: boolean) {
	if (layout[section] === open) return;
	layout[section] = open;
	persistAppSession();
}

function startResize(event: PointerEvent) {
	if (event.button !== 0 || !event.isPrimary) return;
	const handle = event.currentTarget as HTMLElement;
	handle.setPointerCapture(event.pointerId);
	handle.focus();
	drag = {
		pointerId: event.pointerId,
		startX: event.clientX,
		startWidth: width,
	};
	event.preventDefault();
}

function resize(event: PointerEvent) {
	if (!drag || event.pointerId !== drag.pointerId) return;
	setWidth(drag.startWidth + event.clientX - drag.startX);
}

function stopResize(event: PointerEvent) {
	if (event.pointerId !== drag?.pointerId) return;
	drag = null;
	const handle = event.currentTarget as HTMLElement;
	if (handle.hasPointerCapture(event.pointerId)) {
		handle.releasePointerCapture(event.pointerId);
	}
}

function resizeWithKeyboard(event: KeyboardEvent) {
	switch (event.key) {
		case "ArrowLeft":
			setWidth(width - 16);
			break;
		case "ArrowRight":
			setWidth(width + 16);
			break;
		case "Home":
			setWidth(SIDEBAR_MIN_WIDTH);
			break;
		case "End":
			setWidth(maxWidth);
			break;
		default:
			return;
	}
	event.preventDefault();
}
</script>

<aside
  class="sidebar"
  class:resizing={drag !== null}
  style:width={`${width}px`}
  bind:this={sidebar}
  aria-label="ライブラリの分類"
>
  <div class="sections" id={contentId}>
    <details
      class="section"
      open={layout.playlistsOpen}
      ontoggle={(event) => setOpen("playlistsOpen", event.currentTarget.open)}
    >
      <summary>
        <span class="chevron" aria-hidden="true">▸</span>
        <span>プレイリスト</span>
      </summary>
      {@render playlists()}
    </details>
    <details
      class="section"
      open={layout.tagsOpen}
      ontoggle={(event) => setOpen("tagsOpen", event.currentTarget.open)}
    >
      <summary>
        <span class="chevron" aria-hidden="true">▸</span>
        <span>タグ</span>
        {#if activeTagCount > 0}
          <span class="filter-badge">{activeTagCount} 個で絞り込み中</span>
        {/if}
      </summary>
      {@render tags()}
    </details>
    <details
      class="section"
      open={layout.facetsOpen}
      ontoggle={(event) => setOpen("facetsOpen", event.currentTarget.open)}
    >
      <summary>
        <span class="chevron" aria-hidden="true">▸</span>
        <span>ブラウズ</span>
        {#if facetActive}
          <span class="filter-badge">絞り込み中</span>
        {/if}
      </summary>
      {@render facets()}
    </details>
  </div>
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions (フォーカス可能な separator はキーボード操作による幅調整に使用する。) -->
  <div
    class="resize-handle"
    role="separator"
    tabindex="0"
    aria-label="サイドバーの幅"
    aria-orientation="vertical"
    aria-controls={contentId}
    aria-valuemin={SIDEBAR_MIN_WIDTH}
    aria-valuemax={maxWidth}
    aria-valuenow={width}
    aria-valuetext={`${width} ピクセル`}
    title="ドラッグまたは左右キーで幅を調整、ダブルクリックで初期幅に戻す"
    onpointerdown={startResize}
    onpointermove={resize}
    onpointerup={stopResize}
    onpointercancel={stopResize}
    onlostpointercapture={stopResize}
    onkeydown={resizeWithKeyboard}
    ondblclick={() => setWidth(SIDEBAR_DEFAULT_WIDTH)}
  ></div>
</aside>

<style>
  .sidebar {
    position: relative;
    flex: none;
    min-height: 0;
    border-right: 1px solid var(--border);
    background: var(--surface-sidebar);
  }

  .sections {
    height: 100%;
    overflow: auto;
    padding: 0.25rem 0 0.75rem;
    scrollbar-gutter: stable;
  }

  .section + .section {
    border-top: 1px solid var(--border-subtle);
    margin-top: 0.25rem;
    padding-top: 0.25rem;
  }

  summary {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.65rem 0.75rem;
    list-style: none;
    color: var(--text-muted);
    font-size: 0.78rem;
    font-weight: 600;
    cursor: pointer;
  }

  summary::-webkit-details-marker {
    display: none;
  }

  summary:hover {
    background: var(--surface-hover);
    color: var(--text);
  }

  summary:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .chevron {
    transition: transform var(--motion-fast) var(--motion-ease);
  }

  details[open] > summary .chevron {
    transform: rotate(90deg);
  }

  .filter-badge {
    margin-left: auto;
    font-size: 0.65rem;
    font-weight: 400;
    color: var(--accent);
  }

  .resize-handle {
    position: absolute;
    inset: 0 -4px 0 auto;
    z-index: 2;
    width: 9px;
    cursor: col-resize;
    touch-action: none;
  }

  .resize-handle::after {
    position: absolute;
    content: "";
    inset: 0 auto 0 4px;
    width: 2px;
    background: transparent;
  }

  .resize-handle:hover::after,
  .resize-handle:focus-visible::after,
  .resizing .resize-handle::after {
    background: var(--accent);
  }
</style>
