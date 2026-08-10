<script lang="ts">
  import type { Snippet } from "svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import SelectionCheckbox from "$lib/components/SelectionCheckbox.svelte";
  import TrackGrid from "$lib/components/TrackGrid.svelte";
  import TrackRow from "$lib/components/TrackRow.svelte";
  import type { Track } from "$lib/types";
  import { isInRekordbox } from "$lib/rekordboxMembership";
  import {
    filterTracks,
    getVisibleTrackRange,
    sortTracks,
    TRACK_ROW_HEIGHT,
    type SortColumn,
    type SortDirection,
    type ViewMode,
  } from "$lib/trackListView";

  const SORT_LABELS: Record<SortColumn, string> = {
    title: "タイトル",
    artist: "アーティスト",
    album: "アルバム",
    bpm: "BPM",
    bitrateKbps: "ビットレート",
    key: "キー",
    genre: "ジャンル",
    rating: "レート",
    durationMs: "時間",
  };

  type MembershipFilter = "all" | "missing" | "present";

  interface Props {
    tracks: Track[];
    selectedId: number | null;
    readonly?: boolean;
    searchPlaceholder?: string;
    emptyTitle?: string;
    emptyHint?: string;
    bulkRemoveConfirmMessage?: string;
    removeTitle?: string;
    headerExtra?: Snippet;
    rekordboxPathIndex?: Map<string, string>;
    rekordboxWritable?: boolean;
    rekordboxBusy?: boolean;
    rekordboxLockedHint?: string | null;
    onselect: (track: Track) => void;
    onremove?: (track: Track) => void;
    onbulkremove?: (ids: number[]) => void | Promise<void>;
    onAddToRekordbox?: (tracks: Track[]) => void | Promise<void>;
    onRemoveFromRekordbox?: (tracks: Track[]) => void | Promise<void>;
  }

  let {
    tracks,
    selectedId,
    readonly = false,
    searchPlaceholder = "トラックを検索...",
    emptyTitle = "ライブラリにトラックがありません",
    emptyHint = "フォルダを追加して音楽をスキャンしてください",
    bulkRemoveConfirmMessage = "曲をライブラリから削除しますか？",
    removeTitle = "ライブラリから削除",
    headerExtra,
    rekordboxPathIndex,
    rekordboxWritable = false,
    rekordboxBusy = false,
    rekordboxLockedHint = null,
    onselect,
    onremove,
    onbulkremove,
    onAddToRekordbox,
    onRemoveFromRekordbox,
  }: Props = $props();

  /** Library bridge mode: actions live in the command bar, not on rows. */
  let commandBarMode = $derived(
    !!rekordboxPathIndex && (!!onAddToRekordbox || !!onRemoveFromRekordbox),
  );
  let showRowRemove = $derived(!commandBarMode && !readonly && !!onremove);

  let checkedIds = $state<Set<number>>(new Set());
  let membershipFilter = $state<MembershipFilter>("all");

  let queryInput = $state("");
  let query = $state("");
  let viewMode = $state<ViewMode>("list");
  let sortColumn = $state<SortColumn>("artist");
  let sortDirection = $state<SortDirection>("asc");
  let scrollTop = $state(0);
  let viewportHeight = $state(0);

  let tableWrap = $state<HTMLDivElement | null>(null);

  $effect(() => {
    const value = queryInput;
    const timer = setTimeout(() => {
      query = value;
    }, 150);

    return () => clearTimeout(timer);
  });

  $effect(() => {
    const element = tableWrap;
    if (!element) return;

    const observer = new ResizeObserver(([entry]) => {
      viewportHeight = entry.contentRect.height;
    });

    observer.observe(element);
    viewportHeight = element.clientHeight;

    return () => observer.disconnect();
  });

  let searched = $derived(filterTracks(tracks, query));
  let membershipFiltered = $derived.by(() => {
    if (!rekordboxPathIndex || membershipFilter === "all") return searched;
    if (membershipFilter === "missing") {
      return searched.filter((track) => !isInRekordbox(track.path, rekordboxPathIndex));
    }
    return searched.filter((track) => isInRekordbox(track.path, rekordboxPathIndex));
  });
  let sorted = $derived(sortTracks(membershipFiltered, sortColumn, sortDirection));

  let visibleRange = $derived(
    getVisibleTrackRange(scrollTop, viewportHeight, sorted.length),
  );

  let visibleTracks = $derived(sorted.slice(visibleRange.start, visibleRange.end));
  let totalBodyHeight = $derived(sorted.length * TRACK_ROW_HEIGHT);
  let bodyOffsetY = $derived(visibleRange.start * TRACK_ROW_HEIGHT);

  let checkedCount = $derived(checkedIds.size);
  let allVisibleSelected = $derived(
    sorted.length > 0 && sorted.every((track) => checkedIds.has(track.id)),
  );
  let someVisibleSelected = $derived(
    sorted.some((track) => checkedIds.has(track.id)) && !allVisibleSelected,
  );

  let targetTracks = $derived.by((): Track[] => {
    if (checkedIds.size > 0) {
      return tracks.filter((track) => checkedIds.has(track.id));
    }
    if (selectedId != null) {
      const focused = tracks.find((track) => track.id === selectedId);
      return focused ? [focused] : [];
    }
    return [];
  });

  let targetNotInRekordbox = $derived(
    rekordboxPathIndex
      ? targetTracks.filter((track) => !isInRekordbox(track.path, rekordboxPathIndex))
      : [],
  );
  let targetInRekordbox = $derived(
    rekordboxPathIndex
      ? targetTracks.filter((track) => isInRekordbox(track.path, rekordboxPathIndex))
      : [],
  );

  let targetSummary = $derived.by(() => {
    if (targetTracks.length === 0) return "";
    if (checkedIds.size > 0) {
      const parts = [`${targetTracks.length} 曲を選択中`];
      if (rekordboxPathIndex) {
        parts.push(`未登録 ${targetNotInRekordbox.length}`);
        parts.push(`登録済 ${targetInRekordbox.length}`);
      }
      return parts.join(" · ");
    }
    const title = targetTracks[0]?.title ?? "1 曲";
    if (!rekordboxPathIndex) return title;
    return targetInRekordbox.length > 0
      ? `${title} · Rekordbox 登録済`
      : `${title} · 未登録`;
  });

  let showCommandBar = $derived(commandBarMode && targetTracks.length > 0);

  $effect(() => {
    const validIds = new Set(tracks.map((track) => track.id));
    const next = new Set([...checkedIds].filter((id) => validIds.has(id)));
    if (next.size !== checkedIds.size) {
      checkedIds = next;
    }
  });

  function toggleCheck(track: Track) {
    const next = new Set(checkedIds);
    if (next.has(track.id)) {
      next.delete(track.id);
    } else {
      next.add(track.id);
    }
    checkedIds = next;
  }

  function toggleSelectAll() {
    if (allVisibleSelected) {
      const next = new Set(checkedIds);
      for (const track of sorted) {
        next.delete(track.id);
      }
      checkedIds = next;
      return;
    }

    const next = new Set(checkedIds);
    for (const track of sorted) {
      next.add(track.id);
    }
    checkedIds = next;
  }

  function clearSelection() {
    checkedIds = new Set();
  }

  async function handleLibraryRemove() {
    if (targetTracks.length === 0) return;
    const ids = targetTracks.map((track) => track.id);
    const confirmed = await ask(`${ids.length} ${bulkRemoveConfirmMessage}`, {
      title: "削除の確認",
      kind: "warning",
    });
    if (!confirmed) return;

    if (onbulkremove) {
      await onbulkremove(ids);
    } else if (onremove) {
      for (const track of targetTracks) {
        await onremove(track);
      }
    }
    checkedIds = new Set();
  }

  /** Legacy bulk remove for non-command-bar mode (Rekordbox tab). */
  async function handleBulkRemove() {
    if (!onbulkremove) return;
    const ids = [...checkedIds];
    if (ids.length === 0) return;
    const confirmed = await ask(`${ids.length} ${bulkRemoveConfirmMessage}`, {
      title: "削除の確認",
      kind: "warning",
    });
    if (!confirmed) return;

    await onbulkremove(ids);
    checkedIds = new Set();
  }

  async function handleAddToRekordbox() {
    if (!onAddToRekordbox || targetNotInRekordbox.length === 0) return;
    await onAddToRekordbox(targetNotInRekordbox);
  }

  async function handleRemoveFromRekordbox() {
    if (!onRemoveFromRekordbox || targetInRekordbox.length === 0) return;
    const confirmed = await ask(
      `${targetInRekordbox.length} 曲を Rekordbox から削除しますか？`,
      { title: "Rekordbox から削除", kind: "warning" },
    );
    if (!confirmed) return;
    await onRemoveFromRekordbox(targetInRekordbox);
  }

  function handleScroll(event: Event) {
    scrollTop = (event.currentTarget as HTMLDivElement).scrollTop;
  }

  function toggleSort(column: SortColumn) {
    if (sortColumn === column) {
      sortDirection = sortDirection === "asc" ? "desc" : "asc";
      return;
    }

    sortColumn = column;
    sortDirection = "asc";
  }

  function sortIndicator(column: SortColumn): string {
    if (sortColumn !== column) return "";
    return sortDirection === "asc" ? " ↑" : " ↓";
  }
</script>

<div class="track-list">
  <div class="track-list-header">
    <input
      class="search"
      type="search"
      placeholder={searchPlaceholder}
      bind:value={queryInput}
    />
    {#if commandBarMode}
      <div class="membership-filter" role="group" aria-label="Rekordbox 所属フィルタ">
        <button
          type="button"
          class="filter-btn"
          class:active={membershipFilter === "all"}
          aria-pressed={membershipFilter === "all"}
          onclick={() => (membershipFilter = "all")}
        >
          すべて
        </button>
        <button
          type="button"
          class="filter-btn"
          class:active={membershipFilter === "missing"}
          aria-pressed={membershipFilter === "missing"}
          onclick={() => (membershipFilter = "missing")}
        >
          未登録
        </button>
        <button
          type="button"
          class="filter-btn"
          class:active={membershipFilter === "present"}
          aria-pressed={membershipFilter === "present"}
          onclick={() => (membershipFilter = "present")}
        >
          登録済
        </button>
      </div>
    {/if}
    {#if viewMode === "grid"}
      {#if !readonly}
        <div class="select-all-grid">
          <SelectionCheckbox
            checked={allVisibleSelected}
            indeterminate={someVisibleSelected}
            label="表示中のトラックをすべて選択"
            onToggle={toggleSelectAll}
          />
          <button type="button" class="select-all-label" onclick={toggleSelectAll}>
            全選択
          </button>
        </div>
      {/if}
      <div class="grid-sort">
        <label class="sort-label" for="grid-sort-column">並び替え</label>
        <select
          id="grid-sort-column"
          class="sort-select"
          bind:value={sortColumn}
        >
          {#each Object.entries(SORT_LABELS) as [value, label] (value)}
            <option value={value}>{label}</option>
          {/each}
        </select>
        <button
          type="button"
          class="sort-direction"
          aria-label={sortDirection === "asc" ? "昇順" : "降順"}
          onclick={() => (sortDirection = sortDirection === "asc" ? "desc" : "asc")}
        >
          {sortDirection === "asc" ? "↑" : "↓"}
        </button>
      </div>
    {/if}
    <span class="count">{sorted.length} tracks</span>
    {#if !commandBarMode && !readonly && checkedCount > 0}
      <span class="selection-count">{checkedCount} 曲を選択中</span>
      <button type="button" class="bulk-btn danger" onclick={handleBulkRemove}>
        削除
      </button>
      <button type="button" class="bulk-btn" onclick={clearSelection}>
        選択解除
      </button>
    {/if}
    {#if headerExtra}
      {@render headerExtra()}
    {/if}
    <div class="view-toggle" role="group" aria-label="表示切替">
      <button
        type="button"
        class="view-btn"
        class:active={viewMode === "list"}
        aria-label="リスト表示"
        aria-pressed={viewMode === "list"}
        onclick={() => (viewMode = "list")}
      >
        <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
          <rect x="1" y="2" width="14" height="2" rx="0.5" fill="currentColor" />
          <rect x="1" y="7" width="14" height="2" rx="0.5" fill="currentColor" />
          <rect x="1" y="12" width="14" height="2" rx="0.5" fill="currentColor" />
        </svg>
      </button>
      <button
        type="button"
        class="view-btn"
        class:active={viewMode === "grid"}
        aria-label="グリッド表示"
        aria-pressed={viewMode === "grid"}
        onclick={() => (viewMode = "grid")}
      >
        <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
          <rect x="1" y="1" width="6" height="6" rx="1" fill="currentColor" />
          <rect x="9" y="1" width="6" height="6" rx="1" fill="currentColor" />
          <rect x="1" y="9" width="6" height="6" rx="1" fill="currentColor" />
          <rect x="9" y="9" width="6" height="6" rx="1" fill="currentColor" />
        </svg>
      </button>
    </div>
  </div>

  {#if showCommandBar}
    <div class="command-bar" aria-label="トラック操作">
      <span class="command-summary">{targetSummary}</span>
      {#if checkedCount > 0}
        <button type="button" class="bulk-btn" onclick={clearSelection}>
          選択解除
        </button>
      {/if}
      <div class="command-rekordbox">
        {#if !rekordboxWritable && rekordboxLockedHint}
          <span class="rb-hint">{rekordboxLockedHint}</span>
        {/if}
        {#if onAddToRekordbox}
          <button
            type="button"
            class="command-btn primary"
            disabled={!rekordboxWritable ||
              rekordboxBusy ||
              targetNotInRekordbox.length === 0}
            onclick={handleAddToRekordbox}
          >
            Rekordboxに追加
            {#if targetNotInRekordbox.length > 0}
              ({targetNotInRekordbox.length})
            {/if}
          </button>
        {/if}
        {#if onRemoveFromRekordbox}
          <button
            type="button"
            class="command-btn"
            disabled={!rekordboxWritable ||
              rekordboxBusy ||
              targetInRekordbox.length === 0}
            onclick={handleRemoveFromRekordbox}
          >
            Rekordboxから削除
            {#if targetInRekordbox.length > 0}
              ({targetInRekordbox.length})
            {/if}
          </button>
        {/if}
      </div>
      <div class="command-spacer"></div>
      {#if onbulkremove || onremove}
        <button
          type="button"
          class="command-btn danger"
          disabled={rekordboxBusy}
          onclick={handleLibraryRemove}
        >
          ライブラリから削除
        </button>
      {/if}
    </div>
  {:else if commandBarMode && !rekordboxWritable && rekordboxLockedHint}
    <div class="command-bar idle">
      <span class="rb-hint">{rekordboxLockedHint}</span>
    </div>
  {/if}

  {#if sorted.length === 0}
    <div class="empty">
      {#if tracks.length === 0}
        <p>{emptyTitle}</p>
        <p class="hint">{emptyHint}</p>
      {:else if membershipFilter !== "all"}
        <p>このフィルタに一致するトラックがありません</p>
      {:else}
        <p>検索に一致するトラックがありません</p>
      {/if}
    </div>
  {:else if viewMode === "grid"}
    <TrackGrid
      tracks={sorted}
      selectedId={selectedId}
      {checkedIds}
      {readonly}
      {rekordboxPathIndex}
      {showRowRemove}
      {onselect}
      onremove={showRowRemove ? onremove : undefined}
      ontogglecheck={readonly ? undefined : toggleCheck}
    />
  {:else}
    <div
      class="table-wrap"
      role="grid"
      aria-label="Track library"
      bind:this={tableWrap}
      onscroll={handleScroll}
    >
      <div class="table-inner" class:no-actions={!showRowRemove}>
        <div class="table-header" role="row">
          <span class="checkbox-cell sticky-col" role="columnheader">
            {#if !readonly}
              <SelectionCheckbox
                checked={allVisibleSelected}
                indeterminate={someVisibleSelected}
                label="表示中のトラックをすべて選択"
                onToggle={toggleSelectAll}
              />
            {/if}
          </span>
          <span role="columnheader">ジャケット</span>
          <button
            type="button"
            class="column-header"
            class:active={sortColumn === "title"}
            role="columnheader"
            aria-sort={sortColumn === "title" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
            onclick={() => toggleSort("title")}
          >
            タイトル{sortIndicator("title")}
          </button>
          <button
            type="button"
            class="column-header"
            class:active={sortColumn === "artist"}
            role="columnheader"
            aria-sort={sortColumn === "artist" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
            onclick={() => toggleSort("artist")}
          >
            アーティスト{sortIndicator("artist")}
          </button>
          <button
            type="button"
            class="column-header"
            class:active={sortColumn === "album"}
            role="columnheader"
            aria-sort={sortColumn === "album" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
            onclick={() => toggleSort("album")}
          >
            アルバム{sortIndicator("album")}
          </button>
          <button
            type="button"
            class="column-header"
            class:active={sortColumn === "bpm"}
            role="columnheader"
            aria-sort={sortColumn === "bpm" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
            onclick={() => toggleSort("bpm")}
          >
            BPM{sortIndicator("bpm")}
          </button>
          <button
            type="button"
            class="column-header"
            class:active={sortColumn === "bitrateKbps"}
            role="columnheader"
            aria-sort={sortColumn === "bitrateKbps" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
            onclick={() => toggleSort("bitrateKbps")}
          >
            ビットレート{sortIndicator("bitrateKbps")}
          </button>
          <button
            type="button"
            class="column-header"
            class:active={sortColumn === "key"}
            role="columnheader"
            aria-sort={sortColumn === "key" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
            onclick={() => toggleSort("key")}
          >
            キー{sortIndicator("key")}
          </button>
          <button
            type="button"
            class="column-header"
            class:active={sortColumn === "genre"}
            role="columnheader"
            aria-sort={sortColumn === "genre" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
            onclick={() => toggleSort("genre")}
          >
            ジャンル{sortIndicator("genre")}
          </button>
          <button
            type="button"
            class="column-header"
            class:active={sortColumn === "rating"}
            role="columnheader"
            aria-sort={sortColumn === "rating" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
            onclick={() => toggleSort("rating")}
          >
            レート{sortIndicator("rating")}
          </button>
          <button
            type="button"
            class="column-header"
            class:active={sortColumn === "durationMs"}
            role="columnheader"
            aria-sort={sortColumn === "durationMs" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
            onclick={() => toggleSort("durationMs")}
          >
            時間{sortIndicator("durationMs")}
          </button>
          {#if showRowRemove}
            <span role="columnheader"></span>
          {/if}
        </div>

        <div class="virtual-body" style:height="{totalBodyHeight}px">
          <div class="virtual-window" style:transform="translateY({bodyOffsetY}px)">
            {#each visibleTracks as track (track.id)}
              <TrackRow
                {track}
                selected={selectedId === track.id}
                checked={checkedIds.has(track.id)}
                {readonly}
                {removeTitle}
                inRekordbox={rekordboxPathIndex
                  ? isInRekordbox(track.path, rekordboxPathIndex)
                  : false}
                showActions={showRowRemove}
                {onselect}
                onremove={showRowRemove ? onremove : undefined}
                ontogglecheck={readonly ? undefined : toggleCheck}
              />
            {/each}
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .track-list {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }

  .track-list-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1.25rem;
    border-bottom: 1px solid var(--border);
    flex-wrap: wrap;
  }

  .search {
    flex: 1;
    min-width: 10rem;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.9rem;
  }

  .search:focus {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }

  .membership-filter {
    display: flex;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
    flex-shrink: 0;
  }

  .filter-btn {
    padding: 0.35rem 0.7rem;
    border: none;
    background: var(--surface);
    color: var(--text-muted);
    font-size: 0.78rem;
    font-weight: 500;
    cursor: pointer;
  }

  .filter-btn:not(:last-child) {
    border-right: 1px solid var(--border);
  }

  .filter-btn:hover {
    background: var(--surface-hover);
    color: var(--text);
  }

  .filter-btn.active {
    background: var(--accent-subtle);
    color: var(--accent);
  }

  .filter-btn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .count {
    font-size: 0.8rem;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .selection-count {
    font-size: 0.8rem;
    color: var(--accent);
    white-space: nowrap;
  }

  .command-bar {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    padding: 0.65rem 1.25rem;
    border-bottom: 1px solid var(--border);
    background: var(--surface-raised);
    flex-wrap: wrap;
  }

  .command-bar.idle {
    justify-content: flex-start;
  }

  .command-summary {
    font-size: 0.82rem;
    color: var(--text);
    font-weight: 500;
    min-width: 0;
  }

  .command-rekordbox {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .command-spacer {
    flex: 1;
    min-width: 0.5rem;
  }

  .rb-hint {
    font-size: 0.75rem;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .command-btn {
    padding: 0.45rem 0.9rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }

  .command-btn:hover:not(:disabled) {
    background: var(--surface-hover);
  }

  .command-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .command-btn.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }

  .command-btn.primary:hover:not(:disabled) {
    filter: brightness(1.08);
  }

  .command-btn.danger {
    border-color: var(--danger);
    color: var(--danger);
    background: transparent;
  }

  .command-btn.danger:hover:not(:disabled) {
    background: var(--danger-subtle);
  }

  .command-btn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .bulk-btn {
    padding: 0.35rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.8rem;
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
  }

  .bulk-btn:hover:not(:disabled) {
    background: var(--surface-hover);
  }

  .bulk-btn.danger {
    border-color: var(--danger);
    color: var(--danger);
  }

  .bulk-btn.danger:hover:not(:disabled) {
    background: var(--danger-subtle);
  }

  .bulk-btn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .select-all-grid {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    white-space: nowrap;
  }

  .select-all-label {
    padding: 0;
    border: none;
    background: transparent;
    font-size: 0.8rem;
    color: var(--text-muted);
    cursor: pointer;
    user-select: none;
  }

  .select-all-label:hover {
    color: var(--text);
  }

  .select-all-label:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    border-radius: 2px;
  }

  .grid-sort {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  .sort-label {
    font-size: 0.75rem;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .sort-select {
    padding: 0.35rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.8rem;
    cursor: pointer;
  }

  .sort-select:focus {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }

  .sort-direction {
    width: 28px;
    height: 28px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.85rem;
    cursor: pointer;
  }

  .sort-direction:hover {
    background: var(--surface-hover);
  }

  .sort-direction:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .view-toggle {
    display: flex;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }

  .view-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    border: none;
    background: var(--surface);
    color: var(--text-muted);
    cursor: pointer;
  }

  .view-btn:not(:last-child) {
    border-right: 1px solid var(--border);
  }

  .view-btn:hover {
    background: var(--surface-hover);
    color: var(--text);
  }

  .view-btn.active {
    background: var(--accent-subtle);
    color: var(--accent);
  }

  .view-btn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    color: var(--text-muted);
    gap: 0.25rem;
  }

  .empty p {
    margin: 0;
  }

  .hint {
    font-size: 0.85rem;
    opacity: 0.7;
  }

  .table-wrap {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }

  .table-inner {
    min-width: 72rem;
  }

  .table-inner.no-actions {
    min-width: 70rem;
  }

  .table-header {
    display: grid;
    grid-template-columns:
      2.5rem 3rem minmax(10rem, 1.4fr) minmax(8rem, 1.1fr) minmax(8rem, 1.1fr)
      3.5rem 5.5rem 3.5rem minmax(6rem, 1fr) 4.5rem 3.5rem 2rem;
    gap: 0.6rem;
    align-items: center;
    padding: 0 1rem;
    position: sticky;
    top: 0;
    z-index: 1;
    padding-top: 0.5rem;
    padding-bottom: 0.5rem;
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }

  .table-inner.no-actions .table-header {
    grid-template-columns:
      2.5rem 3rem minmax(10rem, 1.4fr) minmax(8rem, 1.1fr) minmax(8rem, 1.1fr)
      3.5rem 5.5rem 3.5rem minmax(6rem, 1fr) 4.5rem 3.5rem;
  }

  .column-header {
    padding: 0;
    border: none;
    background: transparent;
    font: inherit;
    text-transform: inherit;
    letter-spacing: inherit;
    color: inherit;
    text-align: left;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .column-header:hover {
    color: var(--text);
  }

  .column-header.active {
    color: var(--accent);
  }

  .column-header:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    border-radius: 2px;
  }

  .checkbox-cell {
    display: flex;
    justify-content: center;
    align-items: center;
  }

  .sticky-col {
    position: sticky;
    left: 0;
    z-index: 2;
    background: var(--surface);
  }

  .virtual-body {
    position: relative;
  }

  .virtual-window {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    will-change: transform;
  }
</style>
