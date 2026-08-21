<script lang="ts">
  import { ask } from "@tauri-apps/plugin-dialog";
  import {
    playlistCreate,
    playlistSetRule,
    tagAssign,
    tagAxisConflicts,
    tagCreate,
    tagCreateAxis,
    tagDelete,
    tagDeleteAxis,
    tagOfTracks,
    tagRename,
    tagUnassign,
    tagUpdateAxis,
  } from "$lib/api";
  import { hasTrackDrag, readTrackDrag } from "$lib/trackDrag";
  import type { Rule, Tag, TagAxis, Track } from "$lib/types";

  interface Props {
    axes: TagAxis[];
    /** いまチェックが入っている曲。ここへの付け外しは一括で効く。 */
    selectedTracks: Track[];
    activeTagIds: number[];
    busy?: boolean;
    onchanged: () => void | Promise<void>;
    onfilterchange: (tagIds: number[]) => void;
    onerror?: (message: string) => void;
  }

  let {
    axes,
    selectedTracks,
    activeTagIds,
    busy = false,
    onchanged,
    onfilterchange,
    onerror,
  }: Props = $props();

  function fail(error: unknown) {
    onerror?.(error instanceof Error ? error.message : String(error));
  }

  // ---- 選択中の曲が持っているタグ

  let assigned = $state<Map<number, number>>(new Map());

  $effect(() => {
    const ids = selectedTracks.map((track) => track.id);
    if (ids.length === 0) {
      assigned = new Map();
      return;
    }

    let alive = true;
    void tagOfTracks(ids)
      .then((rows) => {
        if (!alive) return;
        const counts = new Map<number, number>();
        for (const row of rows) {
          for (const tagId of row.tagIds) {
            counts.set(tagId, (counts.get(tagId) ?? 0) + 1);
          }
        }
        assigned = counts;
      })
      .catch(fail);

    return () => {
      alive = false;
    };
  });

  type TagState = "none" | "some" | "all";

  function stateOf(tag: Tag): TagState {
    const count = assigned.get(tag.id) ?? 0;
    if (count === 0) return "none";
    return count === selectedTracks.length ? "all" : "some";
  }

  async function toggleAssign(axis: TagAxis, tag: Tag) {
    if (selectedTracks.length === 0) return;
    const ids = selectedTracks.map((track) => track.id);
    try {
      if (stateOf(tag) === "all") {
        await tagUnassign(ids, [tag.id]);
      } else {
        // 単一選択の軸では、同じ軸の別のタグはバックエンド側で外れる。
        await tagAssign(ids, [tag.id]);
      }
      const rows = await tagOfTracks(ids);
      const counts = new Map<number, number>();
      for (const row of rows) {
        for (const tagId of row.tagIds) counts.set(tagId, (counts.get(tagId) ?? 0) + 1);
      }
      assigned = counts;
      await onchanged();
    } catch (error) {
      fail(error);
    }
  }

  // ---- 絞り込み

  let activeSet = $derived(new Set(activeTagIds));

  function toggleFilter(tag: Tag) {
    const next = new Set(activeTagIds);
    if (next.has(tag.id)) next.delete(tag.id);
    else next.add(tag.id);
    onfilterchange([...next]);
  }

  /** 絞り込みを集合層へ上げる。軸をまたいだ選択は AND で効く。 */
  async function promoteFilter() {
    if (activeTagIds.length === 0) return;
    const names = activeTagIds
      .map((id) => axes.flatMap((axis) => axis.tags).find((tag) => tag.id === id)?.name)
      .filter((name): name is string => !!name);
    try {
      const created = await playlistCreate(names.join(" / ") || "タグ", null, "smart");
      const rule: Rule =
        activeTagIds.length === 1
          ? { tag: activeTagIds[0] }
          : { all: activeTagIds.map((id) => ({ tag: id })) };
      await playlistSetRule(created.id, rule, null);
      await onchanged();
    } catch (error) {
      fail(error);
    }
  }

  // ---- 軸とタグの手入れ

  let promptOpen = $state(false);
  let promptTitle = $state("");
  let promptValue = $state("");
  let promptAction: ((name: string) => Promise<void>) | null = null;

  function openPrompt(title: string, initial: string, action: (name: string) => Promise<void>) {
    promptTitle = title;
    promptValue = initial;
    promptAction = action;
    promptOpen = true;
  }

  async function submitPrompt() {
    const name = promptValue.trim();
    if (!name || !promptAction) return;
    const action = promptAction;
    promptOpen = false;
    promptAction = null;
    try {
      await action(name);
      await onchanged();
    } catch (error) {
      fail(error);
    }
  }

  function startCreateAxis() {
    openPrompt("新しい軸", "", async (name) => {
      await tagCreateAxis(name, "multi");
    });
  }

  function startRenameAxis(axis: TagAxis) {
    openPrompt("軸の名前を変更", axis.name, async (name) => {
      await tagUpdateAxis(axis.id, name, null);
    });
  }

  function startCreateTag(axis: TagAxis) {
    openPrompt(`${axis.name} に新しいタグ`, "", async (name) => {
      await tagCreate(axis.id, name);
    });
  }

  function startRenameTag(tag: Tag) {
    openPrompt("タグの名前を変更", tag.name, async (name) => {
      await tagRename(tag.id, name);
    });
  }

  /** multi から single に落とすときだけ、はみ出す曲の数を先に見せる。 */
  async function changeSelection(axis: TagAxis, selection: "single" | "multi") {
    if (selection === axis.selection) return;
    try {
      if (selection === "single") {
        const conflicts = await tagAxisConflicts(axis.id);
        if (conflicts > 0) {
          await ask(
            `${conflicts} 曲がこの軸のタグを2つ以上持っている。単一選択にする前に減らす必要がある。`,
            { title: "単一選択にできない", kind: "warning" },
          );
          return;
        }
      }
      await tagUpdateAxis(axis.id, null, selection);
      await onchanged();
    } catch (error) {
      fail(error);
    }
  }

  async function deleteAxis(axis: TagAxis) {
    const confirmed = await ask(
      `軸「${axis.name}」と、その中の ${axis.tags.length} 個のタグを消しますか？曲は消えません。`,
      { title: "軸の削除", kind: "warning" },
    );
    if (!confirmed) return;
    try {
      await tagDeleteAxis(axis.id);
      onfilterchange(activeTagIds.filter((id) => !axis.tags.some((tag) => tag.id === id)));
      await onchanged();
    } catch (error) {
      fail(error);
    }
  }

  async function deleteTag(tag: Tag) {
    const confirmed = await ask(`タグ「${tag.name}」を消しますか？曲は消えません。`, {
      title: "タグの削除",
      kind: "warning",
    });
    if (!confirmed) return;
    try {
      await tagDelete(tag.id);
      onfilterchange(activeTagIds.filter((id) => id !== tag.id));
      await onchanged();
    } catch (error) {
      fail(error);
    }
  }

  // ---- 曲をタグに落とす

  let dropTagId = $state<number | null>(null);

  function handleTagDragOver(tag: Tag, event: DragEvent) {
    if (!hasTrackDrag(event)) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
    dropTagId = tag.id;
  }

  async function handleTagDrop(tag: Tag, event: DragEvent) {
    if (!hasTrackDrag(event)) return;
    event.preventDefault();
    const trackIds = [...new Set(readTrackDrag(event))];
    dropTagId = null;
    if (trackIds.length === 0) return;
    try {
      await tagAssign(trackIds, [tag.id]);
      await onchanged();
    } catch (error) {
      fail(error);
    }
  }

  let backdropDismissArmed = false;

  function onBackdropPointerDown(event: PointerEvent) {
    backdropDismissArmed = event.target === event.currentTarget;
  }

  function onBackdropPointerUp(event: PointerEvent) {
    if (backdropDismissArmed && event.target === event.currentTarget) promptOpen = false;
    backdropDismissArmed = false;
  }
</script>

<div class="tag-panel">
  <div class="panel-head">
    <span class="panel-title">タグ</span>
    <button type="button" class="side-btn" disabled={busy} onclick={startCreateAxis}>＋ 軸</button>
  </div>

  {#if selectedTracks.length > 0}
    <p class="lead">{selectedTracks.length} 曲に付け外しできる</p>
  {:else}
    <p class="lead">クリックで絞り込み。曲を選ぶと付け外しになる</p>
  {/if}

  {#if axes.length === 0}
    <p class="lead">軸がありません</p>
  {:else}
    <div class="axes">
      {#each axes as axis (axis.id)}
        <div class="axis">
          <div class="axis-head">
            <span class="axis-name">{axis.name}</span>
            <select
              class="axis-selection"
              value={axis.selection}
              disabled={busy}
              title="単一選択の軸は、1曲につき1つしか付かない"
              onchange={(event) =>
                void changeSelection(axis, event.currentTarget.value as "single" | "multi")}
            >
              <option value="multi">複数可</option>
              <option value="single">単一</option>
            </select>
            <div class="axis-ops">
              <button
                type="button"
                class="icon-btn"
                title="タグを足す"
                disabled={busy}
                onclick={() => startCreateTag(axis)}
              >
                ＋
              </button>
              <button
                type="button"
                class="icon-btn"
                title="軸の名前を変更"
                disabled={busy}
                onclick={() => startRenameAxis(axis)}
              >
                ✎
              </button>
              <button
                type="button"
                class="icon-btn"
                title="軸を削除"
                disabled={busy}
                onclick={() => void deleteAxis(axis)}
              >
                ✕
              </button>
            </div>
          </div>

          {#if axis.tags.length === 0}
            <p class="axis-empty">タグがありません</p>
          {:else}
            <div class="tags">
              {#each axis.tags as tag (tag.id)}
                <div
                  class="tag-chip"
                  class:active={activeSet.has(tag.id)}
                  class:assigned={selectedTracks.length > 0 && stateOf(tag) === "all"}
                  class:partial={selectedTracks.length > 0 && stateOf(tag) === "some"}
                  class:drop={dropTagId === tag.id}
                  role="presentation"
                  ondragover={(event) => handleTagDragOver(tag, event)}
                  ondragleave={() => (dropTagId = null)}
                  ondrop={(event) => void handleTagDrop(tag, event)}
                >
                  <button
                    type="button"
                    class="tag-btn"
                    onclick={() =>
                      selectedTracks.length > 0 ? void toggleAssign(axis, tag) : toggleFilter(tag)}
                  >
                    {#if selectedTracks.length > 0}
                      <span class="mark">
                        {stateOf(tag) === "all" ? "●" : stateOf(tag) === "some" ? "◐" : "○"}
                      </span>
                    {/if}
                    <span class="tag-name">{tag.name}</span>
                    <span class="tag-count">{tag.trackCount}</span>
                  </button>
                  <button
                    type="button"
                    class="icon-btn tiny"
                    title="タグの名前を変更"
                    onclick={() => startRenameTag(tag)}
                  >
                    ✎
                  </button>
                  <button
                    type="button"
                    class="icon-btn tiny"
                    title="タグを削除"
                    onclick={() => void deleteTag(tag)}
                  >
                    ✕
                  </button>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  {#if activeTagIds.length > 0}
    <div class="filter-actions">
      <span class="lead">{activeTagIds.length} 個のタグで絞り込み中（すべて満たす）</span>
      <button type="button" class="side-btn" onclick={() => onfilterchange([])}>解除</button>
      <button type="button" class="side-btn primary" onclick={() => void promoteFilter()}>
        スマートプレイリストにする
      </button>
    </div>
  {/if}
</div>

{#if promptOpen}
  <div
    class="modal-backdrop"
    role="presentation"
    onpointerdown={onBackdropPointerDown}
    onpointerup={onBackdropPointerUp}
  >
    <div class="modal" role="dialog" aria-modal="true" aria-label={promptTitle}>
      <h3>{promptTitle}</h3>
      <label>
        名前
        <input
          bind:value={promptValue}
          onkeydown={(event) => {
            if (event.key === "Enter") void submitPrompt();
            if (event.key === "Escape") promptOpen = false;
          }}
        />
      </label>
      <div class="modal-actions">
        <button type="button" class="side-btn" onclick={() => (promptOpen = false)}>
          キャンセル
        </button>
        <button type="button" class="side-btn primary" onclick={() => void submitPrompt()}>
          決定
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .tag-panel {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    padding: 0.5rem;
    border-top: 1px solid var(--border);
    overflow: auto;
    max-height: 45%;
  }

  .panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.4rem;
  }

  .panel-title {
    font-size: 0.75rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .lead {
    margin: 0;
    font-size: 0.7rem;
    color: var(--text-muted);
  }

  .axes {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .axis-head {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .axis-name {
    flex: 1;
    min-width: 0;
    font-size: 0.78rem;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .axis-selection {
    padding: 0.1rem 0.2rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--surface);
    color: var(--text-muted);
    font-size: 0.65rem;
  }

  .axis-ops {
    display: flex;
    gap: 0.05rem;
  }

  .axis-empty {
    margin: 0.1rem 0 0;
    font-size: 0.68rem;
    color: var(--text-muted);
  }

  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin-top: 0.2rem;
  }

  .tag-chip {
    display: flex;
    align-items: center;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    padding-right: 0.15rem;
  }

  .tag-chip.active {
    border-color: var(--accent);
    background: var(--surface-selected);
  }

  .tag-chip.assigned {
    border-color: var(--accent);
  }

  .tag-chip.partial {
    border-style: dashed;
    border-color: var(--accent);
  }

  .tag-chip.drop {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .tag-btn {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.2rem 0.45rem;
    border: none;
    background: transparent;
    color: var(--text);
    font-size: 0.72rem;
    cursor: pointer;
    border-radius: 999px;
  }

  .mark {
    font-size: 0.6rem;
    color: var(--accent);
  }

  .tag-count {
    font-size: 0.62rem;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .icon-btn {
    width: 18px;
    height: 18px;
    padding: 0;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-muted);
    font-size: 0.65rem;
    cursor: pointer;
    opacity: 0;
  }

  .tag-chip:hover .icon-btn,
  .axis-head:hover .icon-btn,
  .tag-chip:focus-within .icon-btn,
  .axis-head:focus-within .icon-btn {
    opacity: 1;
  }

  .icon-btn.tiny {
    font-size: 0.58rem;
  }

  .icon-btn:hover:not(:disabled) {
    background: var(--surface-active);
    color: var(--text);
  }

  .filter-actions {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    flex-wrap: wrap;
  }

  .side-btn {
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.72rem;
    cursor: pointer;
    white-space: nowrap;
  }

  .side-btn:hover:not(:disabled) {
    background: var(--surface-hover);
  }

  .side-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .side-btn.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }

  .modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 20;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.45);
  }

  .modal {
    width: min(24rem, 90vw);
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 1rem;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface-raised);
  }

  .modal h3 {
    margin: 0;
    font-size: 0.95rem;
  }

  .modal label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.78rem;
    color: var(--text-muted);
  }

  .modal input {
    padding: 0.4rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.85rem;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.4rem;
  }
</style>
