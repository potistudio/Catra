<script lang="ts">
import { browseFacet, playlistCreate, playlistSetRule } from "$lib/api";
import type { FacetField, FacetValue, Rule, RuleField } from "$lib/types";

interface Props {
	field: FacetField;
	value: string | null;
	/** 「未設定」も1つの束なので、選ばれていないことと区別する。 */
	active: boolean;
	onselect: (field: FacetField, value: string | null) => void;
	onclear: () => void;
	onchanged: () => void | Promise<void>;
	onerror?: (message: string) => void;
}

let { field, value, active, onselect, onclear, onchanged, onerror }: Props =
	$props();

const FIELDS: Array<{ value: FacetField; label: string }> = [
	{ value: "album", label: "アルバム" },
	{ value: "artist", label: "アーティスト" },
	{ value: "genre", label: "ジャンル" },
	{ value: "key", label: "キー" },
	{ value: "source", label: "入手元" },
];

let values = $state<FacetValue[]>([]);
let loading = $state(false);

$effect(() => {
	const target = field;
	loading = true;
	let alive = true;
	void browseFacet(target)
		.then((rows) => {
			if (alive) values = rows;
		})
		.catch((error) =>
			onerror?.(error instanceof Error ? error.message : String(error)),
		)
		.finally(() => {
			if (alive) loading = false;
		});

	return () => {
		alive = false;
	};
});

function label(item: FacetValue): string {
	return item.value ?? "未設定";
}

/** 眺めるための束を、持ち物としての集合に上げる。 */
async function promote(item: FacetValue) {
	const rule: Rule =
		item.value == null
			? { field: field as RuleField, op: "isNull" }
			: { field: field as RuleField, op: "eq", value: item.value };
	try {
		const created = await playlistCreate(label(item), null, "smart");
		await playlistSetRule(created.id, rule, null);
		await onchanged();
	} catch (error) {
		onerror?.(error instanceof Error ? error.message : String(error));
	}
}
</script>

<div class="facets">
  <div class="facet-head">
    <span class="panel-title">ブラウズ</span>
    <select
      class="facet-select"
      value={field}
      onchange={(event) => onselect(event.currentTarget.value as FacetField, null)}
    >
      {#each FIELDS as option (option.value)}
        <option value={option.value}>{option.label}</option>
      {/each}
    </select>
    {#if active}
      <button type="button" class="side-btn" onclick={onclear}>解除</button>
    {/if}
  </div>

  <p class="lead">値は配信元まかせで揺れる。整えたい分類はタグに移す</p>

  <div class="facet-list">
    {#if loading && values.length === 0}
      <p class="lead">読み込み中...</p>
    {:else if values.length === 0}
      <p class="lead">値がありません</p>
    {:else}
      {#each values as item (item.value ?? "")}
        <div class="facet-row" class:active={active && value === item.value}>
          <button type="button" class="facet-btn" onclick={() => onselect(field, item.value)}>
            <span class="facet-name" class:unset={item.value == null}>{label(item)}</span>
            <span class="facet-count">{item.count}</span>
          </button>
          <button
            type="button"
            class="icon-btn"
            title="スマートプレイリストにする"
            onclick={() => void promote(item)}
          >
            ✦
          </button>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .facets {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    padding: 0.5rem;
    border-top: 1px solid var(--border);
    min-height: 0;
    overflow: hidden;
  }

  .facet-head {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .panel-title {
    flex: 1;
    font-size: 0.75rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .facet-select {
    padding: 0.2rem 0.3rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.72rem;
  }

  .lead {
    margin: 0;
    font-size: 0.68rem;
    color: var(--text-muted);
  }

  .facet-list {
    overflow: auto;
    min-height: 0;
    max-height: 14rem;
  }

  .facet-row {
    display: flex;
    align-items: center;
    border-radius: 4px;
  }

  .facet-row:hover {
    background: var(--surface-hover);
  }

  .facet-row.active {
    background: var(--surface-selected);
  }

  .facet-btn {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    flex: 1;
    min-width: 0;
    padding: 0.22rem 0.3rem;
    border: none;
    background: transparent;
    color: var(--text);
    font-size: 0.74rem;
    text-align: left;
    cursor: pointer;
  }

  .facet-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .facet-name.unset {
    color: var(--text-muted);
    font-style: italic;
  }

  .facet-count {
    font-size: 0.65rem;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .icon-btn {
    width: 20px;
    height: 20px;
    padding: 0;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-muted);
    font-size: 0.68rem;
    cursor: pointer;
    opacity: 0;
  }

  .facet-row:hover .icon-btn,
  .facet-row:focus-within .icon-btn {
    opacity: 1;
  }

  .icon-btn:hover {
    background: var(--surface-active);
    color: var(--accent);
  }

  .side-btn {
    padding: 0.2rem 0.4rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.7rem;
    cursor: pointer;
  }

  .side-btn:hover {
    background: var(--surface-hover);
  }
</style>
