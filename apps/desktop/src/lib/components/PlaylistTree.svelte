<script lang="ts">
import { ask } from "@tauri-apps/plugin-dialog";
import {
	playlistCreate,
	playlistDelete,
	playlistDeleteImpact,
	playlistMove,
	playlistRename,
} from "$lib/api";
import { canDropInto, playlistTreeRows } from "$lib/playlistTree";
import { hasTrackDrag, readTrackDrag } from "$lib/trackDrag";
import type { PlaylistKind, PlaylistNode, TagAxis } from "$lib/types";

interface Props {
	nodes: PlaylistNode[];
	axes: TagAxis[];
	selectedId: number | null;
	expandedIds: number[];
	busy?: boolean;
	onselect: (id: number | null) => void;
	ontoggleexpand: (id: number) => void;
	onchanged: () => void | Promise<void>;
	onerror?: (message: string) => void;
	/** 静的プレイリストに曲が落ちた。列なので、既に入っていても黙って足す。 */
	ondroptracks: (
		playlistId: number,
		trackIds: number[],
	) => void | Promise<void>;
}

let {
	nodes,
	axes,
	selectedId,
	expandedIds,
	busy = false,
	onselect,
	ontoggleexpand,
	onchanged,
	onerror,
	ondroptracks,
}: Props = $props();

const _KIND_ICON: Record<PlaylistKind, string> = {
	folder: "🗀",
	static: "≡",
	smart: "✦",
};

const KIND_LABEL: Record<PlaylistKind, string> = {
	folder: "フォルダ",
	static: "プレイリスト",
	smart: "スマートプレイリスト",
};

let _rows = $derived(playlistTreeRows(nodes, expandedIds));

function fail(error: unknown) {
	onerror?.(error instanceof Error ? error.message : String(error));
}

// ---- 名前の入力

let _promptOpen = $state(false);
let _promptTitle = $state("");
let promptValue = $state("");
let promptAction: ((name: string) => Promise<void>) | null = null;

function openPrompt(
	title: string,
	initial: string,
	action: (name: string) => Promise<void>,
) {
	_promptTitle = title;
	promptValue = initial;
	promptAction = action;
	_promptOpen = true;
}

async function _submitPrompt() {
	const name = promptValue.trim();
	if (!name || !promptAction) return;
	const action = promptAction;
	_promptOpen = false;
	promptAction = null;
	try {
		await action(name);
		await onchanged();
	} catch (error) {
		fail(error);
	}
}

/** 新しい行は、選んでいるフォルダの中に作る。フォルダ以外を選んでいたらその隣に作る。 */
function parentForNew(): number | null {
	const selected = nodes.find((node) => node.id === selectedId);
	if (!selected) return null;
	return selected.kind === "folder" ? selected.id : selected.parentId;
}

function _startCreate(kind: PlaylistKind) {
	openPrompt(`新しい${KIND_LABEL[kind]}`, "", async (name) => {
		await playlistCreate(name, parentForNew(), kind);
	});
}

function _startRename(node: PlaylistNode) {
	openPrompt("名前を変更", node.name, async (name) => {
		await playlistRename(node.id, name);
	});
}

async function _startDelete(node: PlaylistNode) {
	try {
		const impact = await playlistDeleteImpact(node.id);
		const lines = [`「${node.name}」を削除しますか？`];
		if (impact.nodes > 1)
			lines.push(`中の ${impact.nodes - 1} 個も一緒に消えます。`);
		if (impact.entries > 0) {
			lines.push(
				`要素 ${impact.entries} 件が消えます。曲そのものは消えません。`,
			);
		}
		if (impact.referencing.length > 0) {
			const names = impact.referencing.map((ref) => ref.name).join("、");
			lines.push(`${names} の規則がこれを参照しています。その条件は外れます。`);
		}
		const confirmed = await ask(lines.join("\n"), {
			title: "削除の確認",
			kind: "warning",
		});
		if (!confirmed) return;
		await playlistDelete(node.id);
		if (selectedId === node.id) onselect(null);
		await onchanged();
	} catch (error) {
		fail(error);
	}
}

// ---- 規則エディタ

let _ruleTarget = $state<PlaylistNode | null>(null);

// ---- ドラッグ

type DropSpot = "before" | "into" | "after";

let dragId = $state<number | null>(null);
let _dropId = $state<number | null>(null);
let dropSpot = $state<DropSpot>("into");
let _trackDropId = $state<number | null>(null);

function clearDrag() {
	dragId = null;
	_dropId = null;
	_trackDropId = null;
}

function _handleNodeDragStart(node: PlaylistNode, event: DragEvent) {
	dragId = node.id;
	if (event.dataTransfer) {
		event.dataTransfer.effectAllowed = "move";
		event.dataTransfer.setData("text/plain", String(node.id));
	}
}

function spotFor(node: PlaylistNode, event: DragEvent): DropSpot {
	const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
	const ratio = (event.clientY - rect.top) / rect.height;
	if (node.kind !== "folder") return ratio < 0.5 ? "before" : "after";
	if (ratio < 0.25) return "before";
	if (ratio > 0.75) return "after";
	return "into";
}

function _handleRowDragOver(node: PlaylistNode, event: DragEvent) {
	if (hasTrackDrag(event)) {
		// 曲を受けられるのは列だけ。フォルダとスマートは外延を自分で決められない。
		if (node.kind !== "static") return;
		event.preventDefault();
		if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
		_trackDropId = node.id;
		return;
	}

	if (dragId == null) return;
	const spot = spotFor(node, event);
	if (spot === "into" && !canDropInto(nodes, dragId, node.id)) return;
	if (spot !== "into" && !canDropInto(nodes, dragId, node.parentId)) return;
	event.preventDefault();
	if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
	_dropId = node.id;
	dropSpot = spot;
}

async function _handleRowDrop(node: PlaylistNode, event: DragEvent) {
	if (hasTrackDrag(event)) {
		if (node.kind !== "static") return;
		event.preventDefault();
		const trackIds = readTrackDrag(event);
		clearDrag();
		if (trackIds.length === 0) return;
		await ondroptracks(node.id, trackIds);
		return;
	}

	if (dragId == null) return;
	event.preventDefault();
	const moving = dragId;
	const spot = dropSpot;
	clearDrag();
	if (moving === node.id) return;

	try {
		if (spot === "into") {
			const inside = nodes.filter((item) => item.parentId === node.id).length;
			await playlistMove(moving, node.id, inside);
		} else {
			const siblings = nodes
				.filter((item) => item.parentId === node.parentId && item.id !== moving)
				.sort((a, b) => a.position - b.position);
			const anchor = siblings.findIndex((item) => item.id === node.id);
			if (anchor < 0) return;
			await playlistMove(
				moving,
				node.parentId,
				spot === "before" ? anchor : anchor + 1,
			);
		}
		await onchanged();
	} catch (error) {
		fail(error);
	}
}

/** 根に落とす。木のいちばん外側へ出す道。 */
async function _handleRootDrop(event: DragEvent) {
	if (dragId == null || hasTrackDrag(event)) return;
	event.preventDefault();
	const moving = dragId;
	clearDrag();
	try {
		const roots = nodes.filter((item) => item.parentId === null).length;
		await playlistMove(moving, null, roots);
		await onchanged();
	} catch (error) {
		fail(error);
	}
}

let backdropDismissArmed = false;

function _onBackdropPointerDown(event: PointerEvent) {
	backdropDismissArmed = event.target === event.currentTarget;
}

function _onBackdropPointerUp(event: PointerEvent, close: () => void) {
	if (backdropDismissArmed && event.target === event.currentTarget) close();
	backdropDismissArmed = false;
}
</script>

<div
  class="tree"
  role="presentation"
  ondragover={(event) => {
    if (dragId != null && !hasTrackDrag(event)) event.preventDefault();
  }}
  ondrop={handleRootDrop}
>
  <button
    type="button"
    class="all-btn"
    class:active={selectedId === null}
    onclick={() => onselect(null)}
  >
    <span class="kind-icon" aria-hidden="true">◎</span>
    <span class="node-name">全て</span>
  </button>

  <div class="tree-actions">
    <button type="button" class="side-btn" disabled={busy} onclick={() => startCreate("static")}>
      ＋ リスト
    </button>
    <button type="button" class="side-btn" disabled={busy} onclick={() => startCreate("smart")}>
      ＋ スマート
    </button>
    <button type="button" class="side-btn" disabled={busy} onclick={() => startCreate("folder")}>
      ＋ フォルダ
    </button>
  </div>

  <div class="tree-scroll">
    {#if rows.length === 0}
      <p class="tree-empty">プレイリストがありません</p>
    {:else}
      <ul class="tree-list">
        {#each rows as row (row.node.id)}
          <li>
            <div
              class="node-row"
              class:active={selectedId === row.node.id}
              class:drop-before={dropId === row.node.id && dropSpot === "before"}
              class:drop-after={dropId === row.node.id && dropSpot === "after"}
              class:drop-into={(dropId === row.node.id && dropSpot === "into") ||
                trackDropId === row.node.id}
              style={`padding-left: ${0.35 + row.depth * 0.75}rem`}
              draggable={!busy}
              role="presentation"
              ondragstart={(event) => handleNodeDragStart(row.node, event)}
              ondragover={(event) => handleRowDragOver(row.node, event)}
              ondrop={(event) => handleRowDrop(row.node, event)}
              ondragend={clearDrag}
            >
              {#if row.hasChildren}
                <button
                  type="button"
                  class="twisty"
                  aria-label={row.expanded ? "折りたたむ" : "開く"}
                  onclick={() => ontoggleexpand(row.node.id)}
                >
                  {row.expanded ? "▾" : "▸"}
                </button>
              {:else}
                <span class="twisty-space"></span>
              {/if}
              <button
                type="button"
                class="node-item"
                title={KIND_LABEL[row.node.kind]}
                onclick={() => onselect(row.node.id)}
              >
                <span class="kind-icon" aria-hidden="true">{KIND_ICON[row.node.kind]}</span>
                <span class="node-name">{row.node.name}</span>
                <span class="node-count">{row.node.trackCount}</span>
              </button>
              <div class="node-ops">
                {#if row.node.kind === "smart"}
                  <button
                    type="button"
                    class="icon-btn"
                    title="規則を編集"
                    disabled={busy}
                    onclick={() => (ruleTarget = row.node)}
                  >
                    ⚙
                  </button>
                {/if}
                <button
                  type="button"
                  class="icon-btn"
                  title="名前を変更"
                  disabled={busy}
                  onclick={() => startRename(row.node)}
                >
                  ✎
                </button>
                <button
                  type="button"
                  class="icon-btn"
                  title="削除"
                  disabled={busy}
                  onclick={() => startDelete(row.node)}
                >
                  ✕
                </button>
              </div>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

{#if promptOpen}
  <div
    class="modal-backdrop"
    role="presentation"
    onpointerdown={onBackdropPointerDown}
    onpointerup={(event) => onBackdropPointerUp(event, () => (promptOpen = false))}
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

{#if ruleTarget}
  <RuleEditor
    node={ruleTarget}
    {nodes}
    {axes}
    onclose={() => (ruleTarget = null)}
    onsaved={() => {
      ruleTarget = null;
      void onchanged();
    }}
    onerror={(message) => onerror?.(message)}
  />
{/if}

<style>
  .tree {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
  }

  .all-btn {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    width: calc(100% - 1rem);
    margin: 0.5rem 0.5rem 0;
    padding: 0.3rem 0.4rem;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text);
    font-size: 0.8rem;
    text-align: left;
    cursor: pointer;
  }

  .all-btn:hover {
    background: var(--surface-hover);
  }

  .all-btn.active {
    background: var(--surface-selected);
  }

  .tree-actions {
    display: flex;
    gap: 0.3rem;
    padding: 0.5rem 0.5rem 0.35rem;
    flex-wrap: wrap;
  }

  .side-btn {
    padding: 0.3rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.75rem;
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

  .tree-scroll {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }

  .tree-empty {
    margin: 0.75rem 0.6rem;
    font-size: 0.78rem;
    color: var(--text-muted);
  }

  .tree-list {
    list-style: none;
    margin: 0;
    padding: 0 0 0.5rem;
  }

  .node-row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 0.15rem;
    padding-right: 0.3rem;
    border-radius: 4px;
  }

  .node-row:hover {
    background: var(--surface-hover);
  }

  .node-row.active {
    background: var(--surface-selected);
  }

  .node-row.drop-into {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .node-row.drop-before::before,
  .node-row.drop-after::after {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--accent);
    pointer-events: none;
  }

  .node-row.drop-before::before {
    top: -1px;
  }

  .node-row.drop-after::after {
    bottom: -1px;
  }

  .twisty,
  .twisty-space {
    width: 1rem;
    flex-shrink: 0;
  }

  .twisty {
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 0.7rem;
    cursor: pointer;
    padding: 0;
  }

  .node-item {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    flex: 1;
    min-width: 0;
    padding: 0.3rem 0.2rem;
    border: none;
    background: transparent;
    color: var(--text);
    font-size: 0.8rem;
    text-align: left;
    cursor: pointer;
  }

  .kind-icon {
    flex-shrink: 0;
    color: var(--text-muted);
    font-size: 0.75rem;
  }

  .node-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .node-count {
    flex-shrink: 0;
    font-size: 0.7rem;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .node-ops {
    display: flex;
    gap: 0.1rem;
    opacity: 0;
  }

  .node-row:hover .node-ops,
  .node-row:focus-within .node-ops {
    opacity: 1;
  }

  .icon-btn {
    width: 20px;
    height: 20px;
    padding: 0;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-muted);
    font-size: 0.7rem;
    cursor: pointer;
  }

  .icon-btn:hover:not(:disabled) {
    background: var(--surface-active);
    color: var(--text);
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
