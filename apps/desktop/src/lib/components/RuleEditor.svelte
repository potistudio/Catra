<script lang="ts">
import { playlistSetRule } from "$lib/api";
import type {
	PlaylistNode,
	Rule,
	RuleField,
	RuleOp,
	SortRule,
	TagAxis,
} from "$lib/types";

interface Props {
	node: PlaylistNode;
	nodes: PlaylistNode[];
	axes: TagAxis[];
	onclose: () => void;
	onsaved: () => void;
	onerror?: (message: string) => void;
}

let { node, nodes, axes, onclose, onsaved, onerror }: Props = $props();

type FieldType = "text" | "number" | "bool";

const FIELDS: Array<{ value: RuleField; label: string; type: FieldType }> = [
	{ value: "title", label: "タイトル", type: "text" },
	{ value: "artist", label: "アーティスト", type: "text" },
	{ value: "album", label: "アルバム", type: "text" },
	{ value: "genre", label: "ジャンル", type: "text" },
	{ value: "key", label: "キー", type: "text" },
	{ value: "source", label: "入手元", type: "text" },
	{ value: "path", label: "パス", type: "text" },
	{ value: "bpm", label: "BPM", type: "number" },
	{ value: "rating", label: "レート", type: "number" },
	{ value: "bitrateKbps", label: "ビットレート", type: "number" },
	{ value: "durationMs", label: "長さ (ミリ秒)", type: "number" },
	{ value: "addedAt", label: "追加時刻", type: "number" },
	{ value: "converted", label: "変換済み", type: "bool" },
];

const _OP_LABELS: Record<RuleOp, string> = {
	eq: "が次と等しい",
	ne: "が次と違う",
	contains: "が次を含む",
	startsWith: "が次で始まる",
	endsWith: "が次で終わる",
	between: "が次の範囲",
	gt: "が次より大きい",
	gte: "が次以上",
	lt: "が次より小さい",
	lte: "が次以下",
	isNull: "が未設定",
	isNotNull: "が設定済み",
};

const TEXT_OPS: RuleOp[] = [
	"contains",
	"eq",
	"ne",
	"startsWith",
	"endsWith",
	"isNull",
	"isNotNull",
];
const NUMBER_OPS: RuleOp[] = [
	"eq",
	"ne",
	"gt",
	"gte",
	"lt",
	"lte",
	"between",
	"isNull",
	"isNotNull",
];
const BOOL_OPS: RuleOp[] = ["eq"];

function typeOf(field: RuleField): FieldType {
	return FIELDS.find((item) => item.value === field)?.type ?? "text";
}

function opsFor(field: RuleField): RuleOp[] {
	const type = typeOf(field);
	if (type === "number") return NUMBER_OPS;
	if (type === "bool") return BOOL_OPS;
	return TEXT_OPS;
}

type TermDraft =
	| {
			kind: "field";
			negate: boolean;
			field: RuleField;
			op: RuleOp;
			value: string;
			value2: string;
	  }
	| { kind: "tag"; negate: boolean; tagId: number | null }
	| { kind: "playlist"; negate: boolean; playlistId: number | null }
	| {
			kind: "group";
			negate: boolean;
			combinator: "all" | "any";
			terms: TermDraft[];
	  };

function emptyField(): TermDraft {
	return {
		kind: "field",
		negate: false,
		field: "artist",
		op: "contains",
		value: "",
		value2: "",
	};
}

/** 1つの項を読む。読めない形は捨てず、いちばん近い項にして見せる。 */
function parseTerm(rule: Rule, negate = false): TermDraft {
	if ("not" in rule) return parseTerm(rule.not, !negate);
	if ("all" in rule) {
		return {
			kind: "group",
			negate,
			combinator: "all",
			terms: rule.all.map((r) => parseTerm(r)),
		};
	}
	if ("any" in rule) {
		return {
			kind: "group",
			negate,
			combinator: "any",
			terms: rule.any.map((r) => parseTerm(r)),
		};
	}
	if ("tag" in rule) return { kind: "tag", negate, tagId: rule.tag };
	if ("in_playlist" in rule) {
		return { kind: "playlist", negate, playlistId: rule.in_playlist };
	}

	const pair = Array.isArray(rule.value) ? rule.value : null;
	return {
		kind: "field",
		negate,
		field: rule.field,
		op: rule.op,
		value: pair
			? String(pair[0])
			: rule.value == null
				? ""
				: String(rule.value),
		value2: pair ? String(pair[1]) : "",
	};
}

function initialTerms(): TermDraft[] {
	const rule = node.rule;
	if (!rule) return [emptyField()];
	if ("all" in rule) return rule.all.map((r) => parseTerm(r));
	if ("any" in rule) return rule.any.map((r) => parseTerm(r));
	return [parseTerm(rule)];
}

function initialCombinator(): "all" | "any" {
	const rule = node.rule;
	if (rule && "any" in rule) return "any";
	return "all";
}

/** 開いた時点の並び順。編集中は下書きが正で、node は見に行かない。 */
function initialSortColumn(): RuleField | "" {
	return node.sortRule?.column ?? "";
}

function initialSortDirection(): "asc" | "desc" {
	return node.sortRule?.direction ?? "asc";
}

let combinator = $state<"all" | "any">(initialCombinator());
let terms = $state<TermDraft[]>(initialTerms());
let sortColumn = $state<RuleField | "">(initialSortColumn());
let sortDirection = $state<"asc" | "desc">(initialSortDirection());
let _saving = $state(false);
let _localError = $state<string | null>(null);

/** 自分を参照する規則は循環するので、選べる先から自分を外す。 */
let _playlistOptions = $derived(nodes.filter((item) => item.id !== node.id));

function wrap(rule: Rule, negate: boolean): Rule {
	return negate ? { not: rule } : rule;
}

function buildTerm(draft: TermDraft): Rule | null {
	if (draft.kind === "group") {
		const inner = draft.terms
			.map(buildTerm)
			.filter((rule): rule is Rule => rule != null);
		if (inner.length === 0) return null;
		const group: Rule =
			draft.combinator === "all" ? { all: inner } : { any: inner };
		return wrap(group, draft.negate);
	}

	if (draft.kind === "tag") {
		if (draft.tagId == null) return null;
		return wrap({ tag: draft.tagId }, draft.negate);
	}

	if (draft.kind === "playlist") {
		if (draft.playlistId == null) return null;
		return wrap({ in_playlist: draft.playlistId }, draft.negate);
	}

	const type = typeOf(draft.field);
	if (draft.op === "isNull" || draft.op === "isNotNull") {
		return wrap({ field: draft.field, op: draft.op }, draft.negate);
	}

	if (type === "bool") {
		return wrap(
			{ field: draft.field, op: draft.op, value: draft.value === "true" },
			draft.negate,
		);
	}

	if (type === "number") {
		if (draft.op === "between") {
			const low = Number(draft.value);
			const high = Number(draft.value2);
			if (!Number.isFinite(low) || !Number.isFinite(high)) return null;
			return wrap(
				{ field: draft.field, op: "between", value: [low, high] },
				draft.negate,
			);
		}
		const num = Number(draft.value);
		if (draft.value.trim() === "" || !Number.isFinite(num)) return null;
		return wrap({ field: draft.field, op: draft.op, value: num }, draft.negate);
	}

	if (draft.value.trim() === "") return null;
	return wrap(
		{ field: draft.field, op: draft.op, value: draft.value },
		draft.negate,
	);
}

function buildRule(): Rule | null {
	const built = terms
		.map(buildTerm)
		.filter((rule): rule is Rule => rule != null);
	if (built.length === 0) return null;
	if (built.length === 1) return built[0];
	return combinator === "all" ? { all: built } : { any: built };
}

function _addTerm(list: TermDraft[], kind: TermDraft["kind"]) {
	if (kind === "field") list.push(emptyField());
	else if (kind === "tag")
		list.push({ kind: "tag", negate: false, tagId: null });
	else if (kind === "playlist")
		list.push({ kind: "playlist", negate: false, playlistId: null });
	else
		list.push({
			kind: "group",
			negate: false,
			combinator: "any",
			terms: [emptyField()],
		});
}

function _removeAt(list: TermDraft[], index: number) {
	list.splice(index, 1);
}

/** 種類を変えたら演算子も合わせる。文字列に「より大きい」は要らない。 */
function _onFieldChange(draft: TermDraft) {
	if (draft.kind !== "field") return;
	const allowed = opsFor(draft.field);
	if (!allowed.includes(draft.op)) draft.op = allowed[0];
	if (typeOf(draft.field) === "bool")
		draft.value = draft.value === "true" ? "true" : "false";
}

async function _save() {
	_saving = true;
	_localError = null;
	try {
		const rule = buildRule();
		const sort: SortRule | null = sortColumn
			? { column: sortColumn, direction: sortDirection }
			: null;
		await playlistSetRule(node.id, rule, sort);
		onsaved();
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		_localError = message;
		onerror?.(message);
	} finally {
		_saving = false;
	}
}

let backdropDismissArmed = false;

function _onBackdropPointerDown(event: PointerEvent) {
	backdropDismissArmed = event.target === event.currentTarget;
}

function _onBackdropPointerUp(event: PointerEvent) {
	if (backdropDismissArmed && event.target === event.currentTarget) onclose();
	backdropDismissArmed = false;
}
</script>

{#snippet termRow(draft: TermDraft, list: TermDraft[], index: number, nested: boolean)}
  <div class="term" class:nested>
    <label class="negate">
      <input type="checkbox" bind:checked={draft.negate} />
      でない
    </label>

    {#if draft.kind === "field"}
      <select bind:value={draft.field} onchange={() => onFieldChange(draft)}>
        {#each FIELDS as field (field.value)}
          <option value={field.value}>{field.label}</option>
        {/each}
      </select>
      <select bind:value={draft.op}>
        {#each opsFor(draft.field) as op (op)}
          <option value={op}>{OP_LABELS[op]}</option>
        {/each}
      </select>
      {#if draft.op !== "isNull" && draft.op !== "isNotNull"}
        {#if typeOf(draft.field) === "bool"}
          <select bind:value={draft.value}>
            <option value="true">はい</option>
            <option value="false">いいえ</option>
          </select>
        {:else if draft.op === "between"}
          <input class="value" type="number" bind:value={draft.value} placeholder="下限" />
          <input class="value" type="number" bind:value={draft.value2} placeholder="上限" />
        {:else if typeOf(draft.field) === "number"}
          <input class="value" type="number" bind:value={draft.value} />
        {:else}
          <input class="value" type="text" bind:value={draft.value} />
        {/if}
      {/if}
    {:else if draft.kind === "tag"}
      <span class="term-kind">タグ</span>
      <select class="wide" bind:value={draft.tagId}>
        <option value={null}>選択してください</option>
        {#each axes as axis (axis.id)}
          <optgroup label={axis.name}>
            {#each axis.tags as tag (tag.id)}
              <option value={tag.id}>{tag.name}</option>
            {/each}
          </optgroup>
        {/each}
      </select>
    {:else if draft.kind === "playlist"}
      <span class="term-kind">に入っている</span>
      <select class="wide" bind:value={draft.playlistId}>
        <option value={null}>選択してください</option>
        {#each playlistOptions as option (option.id)}
          <option value={option.id}>{option.name}</option>
        {/each}
      </select>
    {:else}
      <select bind:value={draft.combinator}>
        <option value="all">すべてを満たす</option>
        <option value="any">いずれかを満たす</option>
      </select>
    {/if}

    <button
      type="button"
      class="icon-btn"
      title="この条件を消す"
      onclick={() => removeAt(list, index)}
    >
      ✕
    </button>
  </div>

  {#if draft.kind === "group"}
    <div class="group-body">
      {#each draft.terms as inner, innerIndex (innerIndex)}
        {@render termRow(inner, draft.terms, innerIndex, true)}
      {/each}
      <div class="add-row">
        <button type="button" class="add-btn" onclick={() => addTerm(draft.terms, "field")}>
          ＋ 条件
        </button>
        <button type="button" class="add-btn" onclick={() => addTerm(draft.terms, "tag")}>
          ＋ タグ
        </button>
        <button type="button" class="add-btn" onclick={() => addTerm(draft.terms, "playlist")}>
          ＋ 所属
        </button>
      </div>
    </div>
  {/if}
{/snippet}

<div
  class="modal-backdrop"
  role="presentation"
  onpointerdown={onBackdropPointerDown}
  onpointerup={onBackdropPointerUp}
>
  <div class="modal" role="dialog" aria-modal="true" aria-label="規則の編集">
    <h3>{node.name} の規則</h3>
    <p class="lead">
      条件に合う曲が自動で集まる。集合なので、同じ曲が2回並ぶことはない。
    </p>

    <div class="combinator">
      <select bind:value={combinator}>
        <option value="all">すべてを満たす</option>
        <option value="any">いずれかを満たす</option>
      </select>
    </div>

    <div class="terms">
      {#each terms as draft, index (index)}
        {@render termRow(draft, terms, index, false)}
      {/each}
    </div>

    <div class="add-row">
      <button type="button" class="add-btn" onclick={() => addTerm(terms, "field")}>
        ＋ 条件
      </button>
      <button type="button" class="add-btn" onclick={() => addTerm(terms, "tag")}>
        ＋ タグ
      </button>
      <button type="button" class="add-btn" onclick={() => addTerm(terms, "playlist")}>
        ＋ 所属
      </button>
      <button type="button" class="add-btn" onclick={() => addTerm(terms, "group")}>
        ＋ まとまり
      </button>
    </div>

    <div class="sort-row">
      <span class="sort-label">並び</span>
      <select bind:value={sortColumn}>
        <option value="">既定（アーティスト → タイトル）</option>
        {#each FIELDS as field (field.value)}
          <option value={field.value}>{field.label}</option>
        {/each}
      </select>
      <select bind:value={sortDirection} disabled={!sortColumn}>
        <option value="asc">昇順</option>
        <option value="desc">降順</option>
      </select>
    </div>

    {#if localError}
      <p class="error">{localError}</p>
    {/if}

    <div class="modal-actions">
      <button type="button" class="side-btn" onclick={onclose}>キャンセル</button>
      <button type="button" class="side-btn primary" disabled={saving} onclick={() => void save()}>
        保存
      </button>
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 25;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.45);
  }

  .modal {
    width: min(44rem, 94vw);
    max-height: 86vh;
    overflow: auto;
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

  .lead {
    margin: 0;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .terms {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .term {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    flex-wrap: wrap;
  }

  .term.nested {
    padding-left: 1.2rem;
  }

  .group-body {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin: 0.2rem 0 0.4rem;
    padding-left: 0.5rem;
    border-left: 2px solid var(--border);
  }

  .term-kind {
    font-size: 0.78rem;
    color: var(--text-muted);
  }

  .negate {
    display: flex;
    align-items: center;
    gap: 0.2rem;
    font-size: 0.7rem;
    color: var(--text-muted);
    white-space: nowrap;
  }

  select,
  .value {
    padding: 0.3rem 0.4rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.78rem;
  }

  .value {
    width: 8rem;
  }

  select.wide {
    min-width: 12rem;
  }

  .add-row {
    display: flex;
    gap: 0.3rem;
    flex-wrap: wrap;
  }

  .add-btn,
  .side-btn {
    padding: 0.3rem 0.55rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.75rem;
    cursor: pointer;
  }

  .add-btn:hover,
  .side-btn:hover:not(:disabled) {
    background: var(--surface-hover);
  }

  .side-btn.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }

  .side-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .icon-btn {
    width: 22px;
    height: 22px;
    padding: 0;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-muted);
    font-size: 0.72rem;
    cursor: pointer;
  }

  .icon-btn:hover {
    background: var(--surface-active);
    color: var(--text);
  }

  .sort-row {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding-top: 0.35rem;
    border-top: 1px solid var(--border);
  }

  .sort-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .error {
    margin: 0;
    font-size: 0.75rem;
    color: var(--danger);
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.4rem;
  }
</style>
