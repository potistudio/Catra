<script lang="ts">
  import type { Snippet } from "svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import SelectionCheckbox from "$lib/components/SelectionCheckbox.svelte";
  import TrackGrid from "$lib/components/TrackGrid.svelte";
  import TrackRow from "$lib/components/TrackRow.svelte";
  import { appSession, persistAppSession } from "$lib/appSession.svelte";
  import type { PlaylistEntry, Track } from "$lib/types";
  import { isInRekordbox } from "$lib/rekordboxMembership";
  import { writeTrackDrag } from "$lib/trackDrag";
  import {
    filterRows,
    getVisibleTrackRange,
    rowsFromTracks,
    sortRows,
    TRACK_ROW_HEIGHT,
    type SortColumn,
    type SortDirection,
    type TrackListRow,
    type ViewMode,
  } from "$lib/trackListView";

  const SORT_LABELS: Record<SortColumn, string> = {
    position: "列の順番",
    title: "タイトル",
    artist: "アーティスト",
    album: "アルバム",
    bpm: "BPM",
    bitrateKbps: "ビットレート",
    key: "キー",
    genre: "ジャンル",
    rating: "レート",
    durationMs: "時間",
    addedAt: "追加時刻",
  };

  type MembershipFilter = "all" | "missing" | "present";
  type ListSessionScope = "library" | "rekordbox" | "trash";

  interface Props {
    tracks?: Track[];
    /**
     * 列（静的プレイリスト）や集合（スマート／フォルダ）の中身。
     * 渡すと `tracks` の代わりにこちらが一覧になる。
     * `entryId` が入っているなら列なので、同じ曲が2回並ぶ。
     */
    entries?: PlaylistEntry[] | null;
    selectedId: number | null;
    readonly?: boolean;
    searchPlaceholder?: string;
    emptyTitle?: string;
    emptyHint?: string;
    bulkRemoveConfirmMessage?: string;
    removeTitle?: string;
    bulkRemoveLabel?: string;
    permanentConfirmMessage?: string;
    permanentTitle?: string;
    emptyTrashLabel?: string;
    headerExtra?: Snippet;
    rekordboxPathIndex?: Map<string, string>;
    rekordboxContentIds?: Set<string>;
    rekordboxWritable?: boolean;
    rekordboxBusy?: boolean;
    rekordboxLockedHint?: string | null;
    sessionScope?: ListSessionScope;
    onselect: (track: Track) => void;
    onremove?: (track: Track) => void;
    onbulkremove?: (ids: number[]) => void | Promise<void>;
    onbulkpermanent?: (ids: number[]) => void | Promise<void>;
    onemptytrash?: () => void | Promise<void>;
    onAddToRekordbox?: (tracks: Track[]) => void | Promise<void>;
    onRemoveFromRekordbox?: (tracks: Track[]) => void | Promise<void>;
    onconvert?: (tracks: Track[]) => void;
    convertBusy?: boolean;
    /** 列の並べ替え。要素 ID の完全な配列を新しい順で受け取る。 */
    onreorder?: (entryIds: number[]) => void | Promise<void>;
    /** この要素だけプレイリストから外す。同じ曲の別の要素は残る。 */
    onremoveentries?: (entryIds: number[]) => void | Promise<void>;
    removeEntriesLabel?: string;
    /** 選択中の曲にタグを付ける入口。 */
    ontagtracks?: (tracks: Track[]) => void;
  }

  let {
    tracks = [],
    entries = null,
    selectedId,
    readonly = false,
    searchPlaceholder = "トラックを検索...",
    emptyTitle = "ライブラリにトラックがありません",
    emptyHint = "フォルダを追加するか、ファイルをドロップしてください",
    bulkRemoveConfirmMessage = "曲をライブラリから外してゴミ箱へ移しますか？",
    removeTitle = "ライブラリから外す",
    bulkRemoveLabel = "ライブラリから外す",
    permanentConfirmMessage = "曲を完全に削除しますか？この操作は取り消せません。",
    permanentTitle = "完全に削除",
    emptyTrashLabel = "ゴミ箱を空にする",
    headerExtra,
    rekordboxPathIndex,
    rekordboxContentIds,
    rekordboxWritable = false,
    rekordboxBusy = false,
    rekordboxLockedHint = null,
    sessionScope,
    onselect,
    onremove,
    onbulkremove,
    onbulkpermanent,
    onemptytrash,
    onAddToRekordbox,
    onRemoveFromRekordbox,
    onconvert,
    convertBusy = false,
    onreorder,
    onremoveentries,
    removeEntriesLabel = "プレイリストから外す",
    ontagtracks,
  }: Props = $props();

  /** Library bridge mode: actions live in the command bar, not on rows. */
  let commandBarMode = $derived(
    !!rekordboxPathIndex && (!!onAddToRekordbox || !!onRemoveFromRekordbox),
  );
  let showRowRemove = $derived(!commandBarMode && !readonly && !!onremove);

  const listSession =
    sessionScope === "rekordbox"
      ? appSession.rekordbox.list
      : sessionScope === "trash"
        ? appSession.trash
        : sessionScope === "library"
          ? appSession.library.list
          : null;

  /** 選択の単位は行。列の中では要素 ID、それ以外はトラック ID。 */
  let checkedKeys = $state<Set<number>>(new Set());
  let membershipFilter = $state<MembershipFilter>(listSession?.membershipFilter ?? "all");

  let queryInput = $state(listSession?.query ?? "");
  let query = $state(listSession?.query ?? "");
  let viewMode = $state<ViewMode>(listSession?.viewMode ?? "list");
  let sortColumn = $state<SortColumn>(listSession?.sortColumn ?? "artist");
  let sortDirection = $state<SortDirection>(listSession?.sortDirection ?? "asc");
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
    if (!listSession) return;
    listSession.query = queryInput;
    listSession.viewMode = viewMode;
    listSession.sortColumn = sortColumn;
    listSession.sortDirection = sortDirection;
    listSession.membershipFilter = membershipFilter;
    persistAppSession();
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

  let allRows = $derived.by<TrackListRow[]>(() => {
    if (!entries) return rowsFromTracks(tracks);
    return entries.map((entry) => ({
      key: entry.entryId ?? entry.track.id,
      entryId: entry.entryId,
      position: entry.position,
      track: entry.track,
    }));
  });

  /** 列かどうか。要素 ID を持つ一覧だけが列で、並べ替えができる。 */
  let isSequence = $derived(!!entries && entries.length > 0 && entries[0].entryId != null);
  let sourceCount = $derived(entries ? entries.length : tracks.length);

  let searched = $derived(filterRows(allRows, query));
  let membershipFiltered = $derived.by(() => {
    if (!rekordboxPathIndex || membershipFilter === "all") return searched;
    if (membershipFilter === "missing") {
      return searched.filter(({ track }) => !isInRekordbox(track, rekordboxPathIndex, rekordboxContentIds));
    }
    return searched.filter(({ track }) => isInRekordbox(track, rekordboxPathIndex, rekordboxContentIds));
  });
  let sorted = $derived(sortRows(membershipFiltered, sortColumn, sortDirection));

  /**
   * 並べ替えは列の全体を書き換える操作なので、一覧が列そのものを映しているときだけ許す。
   * 絞り込みや別の列での並びが挟まっていると、見えている順番が列の順番と違う。
   */
  let reorderable = $derived(
    isSequence &&
      !!onreorder &&
      !readonly &&
      viewMode === "list" &&
      sortColumn === "position" &&
      sortDirection === "asc" &&
      sorted.length === allRows.length,
  );

  let visibleRange = $derived(
    getVisibleTrackRange(scrollTop, viewportHeight, sorted.length),
  );

  let visibleRows = $derived(sorted.slice(visibleRange.start, visibleRange.end));
  let totalBodyHeight = $derived(sorted.length * TRACK_ROW_HEIGHT);
  let bodyOffsetY = $derived(visibleRange.start * TRACK_ROW_HEIGHT);

  let checkedCount = $derived(checkedKeys.size);
  let allVisibleSelected = $derived(
    sorted.length > 0 && sorted.every((row) => checkedKeys.has(row.key)),
  );
  let someVisibleSelected = $derived(
    sorted.some((row) => checkedKeys.has(row.key)) && !allVisibleSelected,
  );

  let targetRows = $derived(allRows.filter((row) => checkedKeys.has(row.key)));

  /**
   * Action targets come only from checkboxes; card/row click is preview focus.
   * 曲への操作（変換、Rekordbox、ゴミ箱、タグ）は曲に1回だけ効くので、
   * 同じ曲の要素を2つ選んでいても1曲に畳む。
   */
  let targetTracks = $derived.by(() => {
    const seen = new Set<number>();
    const picked: Track[] = [];
    for (const { track } of targetRows) {
      if (seen.has(track.id)) continue;
      seen.add(track.id);
      picked.push(track);
    }
    return picked;
  });

  /** 列から外す操作だけは要素単位。同じ曲の片方だけを外せる。 */
  let targetEntryIds = $derived(
    targetRows
      .map((row) => row.entryId)
      .filter((id): id is number => id != null),
  );

  let targetNotInRekordbox = $derived(
    rekordboxPathIndex
      ? targetTracks.filter((track) => !isInRekordbox(track, rekordboxPathIndex, rekordboxContentIds))
      : [],
  );
  let targetInRekordbox = $derived(
    rekordboxPathIndex
      ? targetTracks.filter((track) => isInRekordbox(track, rekordboxPathIndex, rekordboxContentIds))
      : [],
  );

  let targetSummary = $derived.by(() => {
    if (targetTracks.length === 0) return "";
    const parts = [`${targetTracks.length} 曲を選択中`];
    if (rekordboxPathIndex) {
      parts.push(`未登録 ${targetNotInRekordbox.length}`);
      parts.push(`登録済 ${targetInRekordbox.length}`);
    }
    return parts.join(" · ");
  });

  let showCommandBar = $derived(
    (commandBarMode || !!onremoveentries || !!ontagtracks) && checkedCount > 0,
  );

  /** Remove keys that are not in the current filtered list from checkedKeys. */
  $effect(() => {
    const visibleKeys = new Set(sorted.map((row) => row.key));
    const next = new Set([...checkedKeys].filter((key) => visibleKeys.has(key)));
    if (next.size !== checkedKeys.size) {
      checkedKeys = next;
    }
  });

  function toggleCheck(row: TrackListRow) {
    const next = new Set(checkedKeys);
    if (next.has(row.key)) {
      next.delete(row.key);
    } else {
      next.add(row.key);
    }
    checkedKeys = next;
  }

  function toggleSelectAll() {
    if (allVisibleSelected || someVisibleSelected) {
      checkedKeys = new Set();
      return;
    }

    const next = new Set(checkedKeys);
    for (const row of sorted) {
      next.add(row.key);
    }
    checkedKeys = next;
  }

  /** どの行を今プレビューしているか。同じ曲が2回あっても、押した方だけ光る。 */
  let focusedKey = $state<number | null>(null);

  function isFocused(row: TrackListRow): boolean {
    if (selectedId !== row.track.id) return false;
    if (focusedKey == null) return true;
    if (!allRows.some((candidate) => candidate.key === focusedKey)) return true;
    return focusedKey === row.key;
  }

  function selectRow(row: TrackListRow) {
    focusedKey = row.key;
    onselect(row.track);
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
    checkedKeys = new Set();
  }

  /** Legacy bulk remove for non-command-bar mode (Rekordbox tab). */
  async function handleBulkRemove() {
    if (!onbulkremove) return;
    const ids = targetTracks.map((track) => track.id);
    if (ids.length === 0) return;
    const confirmed = await ask(`${ids.length} ${bulkRemoveConfirmMessage}`, {
      title: "削除の確認",
      kind: "warning",
    });
    if (!confirmed) return;

    await onbulkremove(ids);
    checkedKeys = new Set();
  }

  async function handlePermanentDelete() {
    if (!onbulkpermanent) return;
    const ids = targetTracks.map((track) => track.id);
    if (ids.length === 0) return;
    const confirmed = await ask(`${ids.length} ${permanentConfirmMessage}`, {
      title: "完全削除の確認",
      kind: "warning",
    });
    if (!confirmed) return;
    await onbulkpermanent(ids);
    checkedKeys = new Set();
  }

  async function handleEmptyTrash() {
    if (!onemptytrash) return;
    const confirmed = await ask("ゴミ箱の中身をすべて完全に削除しますか？この操作は取り消せません。", {
      title: "ゴミ箱を空にする",
      kind: "warning",
    });
    if (!confirmed) return;
    await onemptytrash();
    checkedKeys = new Set();
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

  function handleConvert() {
    if (!onconvert || targetTracks.length === 0 || convertBusy) return;
    onconvert(targetTracks);
  }

  function handleTag() {
    if (!ontagtracks || targetTracks.length === 0) return;
    ontagtracks(targetTracks);
  }

  /** 確認は出さない。列から要素を抜くのは、曲を消すことではない。 */
  async function handleRemoveEntries() {
    if (!onremoveentries || targetEntryIds.length === 0) return;
    await onremoveentries(targetEntryIds);
    checkedKeys = new Set();
  }

  let dragKey = $state<number | null>(null);
  let dropKey = $state<number | null>(null);
  let dropAfter = $state(false);

  /**
   * 掴んだ行がチェック済みなら選択の全部を、そうでなければその1行だけを運ぶ。
   * 列の中では同じ曲が2回入ることもあるので、重複は畳まずそのまま渡す。
   */
  function draggedTrackIds(row: TrackListRow): number[] {
    if (!checkedKeys.has(row.key)) return [row.track.id];
    return sorted.filter((item) => checkedKeys.has(item.key)).map((item) => item.track.id);
  }

  function handleDragStart(row: TrackListRow, event: DragEvent) {
    dragKey = reorderable ? row.key : null;
    if (!event.dataTransfer) return;
    event.dataTransfer.effectAllowed = reorderable ? "copyMove" : "copy";
    event.dataTransfer.setData("text/plain", String(row.key));
    writeTrackDrag(event, draggedTrackIds(row));
  }

  function handleDragOver(row: TrackListRow, event: DragEvent) {
    if (!reorderable || dragKey == null) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    dropAfter = event.clientY - rect.top > rect.height / 2;
    dropKey = row.key;
  }

  function clearDrag() {
    dragKey = null;
    dropKey = null;
    dropAfter = false;
  }

  async function handleDrop(row: TrackListRow, event: DragEvent) {
    if (!reorderable || !onreorder || dragKey == null) return;
    event.preventDefault();

    const moving = dragKey;
    const after = dropAfter;
    clearDrag();
    if (moving === row.key) return;

    // 列の全体を作り直して渡す。動かすのは要素なので、同じ曲の片方だけが動く。
    const order = sorted.map((item) => item.key).filter((key) => key !== moving);
    const anchor = order.indexOf(row.key);
    if (anchor < 0) return;
    order.splice(after ? anchor + 1 : anchor, 0, moving);

    const byKey = new Map(sorted.map((item) => [item.key, item]));
    const entryIds = order
      .map((key) => byKey.get(key)?.entryId ?? null)
      .filter((id): id is number => id != null);
    if (entryIds.length !== order.length) return;

    await onreorder(entryIds);
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

  let sortOptions = $derived(
    Object.entries(SORT_LABELS).filter(([value]) => value !== "position" || isSequence),
  );

  /** 列を開いたら列の順番で見せる。列でない一覧に「列の順番」は残さない。 */
  let wasSequence = $state(false);
  $effect(() => {
    if (isSequence === wasSequence) return;
    wasSequence = isSequence;
    if (isSequence) {
      sortColumn = "position";
      sortDirection = "asc";
    } else if (sortColumn === "position") {
      sortColumn = "artist";
      sortDirection = "asc";
    }
  });
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
          {#each sortOptions as [value, label] (value)}
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
    {#if isSequence && sortColumn !== "position"}
      <button
        type="button"
        class="bulk-btn"
        title="表示だけを並べ替えている。列そのものの順番は変わっていない。"
        onclick={() => {
          sortColumn = "position";
          sortDirection = "asc";
        }}
      >
        プレイリスト順に戻す
      </button>
    {/if}
    {#if commandBarMode && !rekordboxWritable && rekordboxLockedHint}
      <span class="rb-hint">{rekordboxLockedHint}</span>
    {/if}
    {#if !commandBarMode && !readonly && checkedCount > 0}
      <span class="selection-count">{checkedCount} 曲を選択中</span>
      <button type="button" class="bulk-btn danger" onclick={handleBulkRemove}>
        {bulkRemoveLabel}
      </button>
      {#if onbulkpermanent}
        <button type="button" class="bulk-btn danger" onclick={handlePermanentDelete}>
          {permanentTitle}
        </button>
      {/if}
    {/if}
    {#if headerExtra}
      <div class="header-extra">
        {@render headerExtra()}
      </div>
    {/if}
    {#if onemptytrash}
      <button type="button" class="bulk-btn danger" onclick={handleEmptyTrash}>
        {emptyTrashLabel}
      </button>
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

  {#if sorted.length === 0}
    <div class="empty">
      {#if sourceCount === 0}
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
      rows={sorted}
      selectedId={selectedId}
      {checkedKeys}
      {readonly}
      {rekordboxPathIndex}
      {rekordboxContentIds}
      {showRowRemove}
      onselect={selectRow}
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
      <div class="table-inner" class:no-actions={!showRowRemove} class:with-position={isSequence}>
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
          {#if isSequence}
            <button
              type="button"
              class="column-header position-header"
              class:active={sortColumn === "position"}
              role="columnheader"
              title="列の順番"
              aria-sort={sortColumn === "position" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
              onclick={() => toggleSort("position")}
            >
              #{sortIndicator("position")}
            </button>
          {/if}
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
          <button
            type="button"
            class="column-header"
            class:active={sortColumn === "addedAt"}
            role="columnheader"
            aria-sort={sortColumn === "addedAt" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
            onclick={() => toggleSort("addedAt")}
          >
            追加時刻{sortIndicator("addedAt")}
          </button>
          {#if showRowRemove}
            <span role="columnheader"></span>
          {/if}
        </div>

        <div class="virtual-body" style:height="{totalBodyHeight}px">
          <div class="virtual-window" style:transform="translateY({bodyOffsetY}px)">
            {#each visibleRows as row (row.key)}
              <div
                class="row-slot"
                class:dragging={dragKey === row.key}
                class:drop-before={dropKey === row.key && !dropAfter}
                class:drop-after={dropKey === row.key && dropAfter}
                draggable={true}
                role="presentation"
                ondragstart={(event) => handleDragStart(row, event)}
                ondragover={(event) => handleDragOver(row, event)}
                ondrop={(event) => handleDrop(row, event)}
                ondragend={clearDrag}
              >
                <TrackRow
                  track={row.track}
                  position={isSequence ? row.position : null}
                  selected={isFocused(row)}
                  checked={checkedKeys.has(row.key)}
                  {readonly}
                  {removeTitle}
                  inRekordbox={rekordboxPathIndex
                    ? isInRekordbox(row.track, rekordboxPathIndex, rekordboxContentIds)
                    : false}
                  showActions={showRowRemove}
                  onselect={() => selectRow(row)}
                  onremove={showRowRemove ? onremove : undefined}
                  ontogglecheck={readonly ? undefined : () => toggleCheck(row)}
                />
              </div>
            {/each}
          </div>
        </div>
      </div>
    </div>
  {/if}

  {#if showCommandBar}
    <div class="command-bar" aria-label="トラック操作">
      <span class="command-summary">{targetSummary}</span>
      {#if ontagtracks}
        <button type="button" class="command-btn" onclick={handleTag}>
          タグ ({targetTracks.length})
        </button>
      {/if}
      {#if onconvert}
        <button
          type="button"
          class="command-btn"
          disabled={convertBusy || rekordboxBusy}
          onclick={handleConvert}
        >
          変換
          {#if targetTracks.length > 0}
            ({targetTracks.length})
          {/if}
        </button>
      {/if}
      <div class="command-rekordbox">
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
      {#if onremoveentries}
        <button
          type="button"
          class="command-btn"
          disabled={targetEntryIds.length === 0}
          onclick={handleRemoveEntries}
        >
          {removeEntriesLabel}
          {#if targetEntryIds.length > 0}
            ({targetEntryIds.length})
          {/if}
        </button>
      {/if}
      {#if onbulkremove || onremove}
        <button
          type="button"
          class="command-btn danger"
          disabled={rekordboxBusy}
          onclick={handleLibraryRemove}
        >
          {bulkRemoveLabel}
        </button>
      {/if}
      {#if onbulkpermanent}
        <button
          type="button"
          class="command-btn danger"
          disabled={rekordboxBusy}
          onclick={handlePermanentDelete}
        >
          {permanentTitle}
        </button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .track-list {
    position: relative;
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
    background: var(--surface-selected);
    color: var(--accent);
  }

  .filter-btn.active:hover {
    background: var(--surface-selected-hover);
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

  .header-extra :global(button) {
    padding: 0.35rem 0.7rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.78rem;
    cursor: pointer;
  }

  .selection-count {
    font-size: 0.8rem;
    color: var(--accent);
    white-space: nowrap;
  }

  .command-bar {
    position: absolute;
    left: 0.75rem;
    right: 0.75rem;
    bottom: 0.75rem;
    z-index: 5;
    display: flex;
    align-items: center;
    gap: 0.65rem;
    padding: 0.65rem 1rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-raised);
    flex-wrap: wrap;
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
    border-color: var(--surface-active);
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
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }

  .command-btn.danger {
    border-color: var(--danger);
    color: var(--danger);
    background: transparent;
  }

  .command-btn.danger:hover:not(:disabled) {
    background: var(--danger-subtle-hover);
    color: var(--danger-hover);
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
    border-color: var(--surface-active);
  }

  .bulk-btn.danger {
    border-color: var(--danger);
    color: var(--danger);
  }

  .bulk-btn.danger:hover:not(:disabled) {
    background: var(--danger-subtle-hover);
    color: var(--danger-hover);
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
    border-color: var(--surface-active);
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
    background: var(--surface-selected);
    color: var(--accent);
  }

  .view-btn.active:hover {
    background: var(--surface-selected-hover);
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
    min-width: 81rem;
  }

  .table-inner.no-actions {
    min-width: 79rem;
  }

  .table-header {
    display: grid;
    grid-template-columns:
      2.5rem 3rem minmax(10rem, 1.4fr) minmax(8rem, 1.1fr) minmax(8rem, 1.1fr)
      3.5rem 5.5rem 3.5rem minmax(6rem, 1fr) 4.5rem 3.5rem 8.5rem 2rem;
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
      3.5rem 5.5rem 3.5rem minmax(6rem, 1fr) 4.5rem 3.5rem 8.5rem;
  }

  /* 列のときだけ先頭に順番の列が増える。 */
  .table-inner.with-position {
    min-width: 84rem;
  }

  .table-inner.with-position.no-actions {
    min-width: 82rem;
  }

  .table-inner.with-position .table-header {
    grid-template-columns:
      2.5rem 3rem 3rem minmax(10rem, 1.4fr) minmax(8rem, 1.1fr) minmax(8rem, 1.1fr)
      3.5rem 5.5rem 3.5rem minmax(6rem, 1fr) 4.5rem 3.5rem 8.5rem 2rem;
  }

  .table-inner.with-position.no-actions .table-header {
    grid-template-columns:
      2.5rem 3rem 3rem minmax(10rem, 1.4fr) minmax(8rem, 1.1fr) minmax(8rem, 1.1fr)
      3.5rem 5.5rem 3.5rem minmax(6rem, 1fr) 4.5rem 3.5rem 8.5rem;
  }

  .position-header {
    text-align: right;
    padding-right: 0.2rem;
  }

  .row-slot {
    position: relative;
  }

  .row-slot.dragging {
    opacity: 0.45;
  }

  .row-slot.drop-before::before,
  .row-slot.drop-after::after {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--accent);
    z-index: 2;
    pointer-events: none;
  }

  .row-slot.drop-before::before {
    top: -1px;
  }

  .row-slot.drop-after::after {
    bottom: -1px;
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
