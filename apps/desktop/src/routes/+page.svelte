<script lang="ts">
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { onMount } from "svelte";
import {
	consolePanel,
	pushActivityLog,
	pushActivityLogPayload,
} from "$lib/activityLog.svelte";
import {
	browseFacetTracks,
	browseTagTracks,
	checkLibraryHealth,
	convertTracks,
	deleteTracksPermanently,
	emptyTrash,
	importFromRekordbox,
	importPaths,
	libraryAddToRekordbox,
	libraryRemoveFromRekordbox,
	listTracks,
	listTrashed,
	playlistAddTracks,
	playlistEntries,
	playlistListTree,
	playlistRemoveEntries,
	playlistReorderEntries,
	rekordboxCheck,
	rekordboxGetContent,
	removeTrack,
	removeTracks,
	resolveDuplicate,
	restoreTracks,
	scanFolder,
	tagListAxes,
} from "$lib/api";
import {
	type AppTab,
	appSession,
	type BrowseMode,
	persistAppSession,
} from "$lib/appSession.svelte";
import { ancestorIds } from "$lib/playlistTree";
import { rekordboxContentToPreview } from "$lib/rekordboxListView";
import {
	buildRekordboxIdSet,
	buildRekordboxPathIndex,
} from "$lib/rekordboxMembership";
import type {
	ActivityLogPayload,
	ConvertOptions,
	ConvertProgress,
	ConvertResult,
	DownloadProgress,
	DuplicateChoice,
	DuplicateFoundPayload,
	FacetField,
	PlaylistEntry,
	PlaylistNode,
	PreviewableTrack,
	RekordboxCheck,
	RekordboxContent,
	ScanProgress,
	ScanResult,
	TagAxis,
	Track,
} from "$lib/types";

/** Rekordbox may start or quit after the initial check; keep the write lock aligned. */
const REKORDBOX_RUNNING_POLL_MS = 2000;

let activeTab = $state<AppTab>(appSession.activeTab);
let tracks = $state<Track[]>([]);
let trashedTracks = $state<Track[]>([]);
let selectedTrack = $state<Track | null>(null);
let rekordboxTracks = $state<RekordboxContent[]>([]);
let selectedRekordboxId = $state<string | null>(appSession.selectedRekordboxId);
let rekordboxStatus = $state<RekordboxCheck | null>(null);
let rekordboxLoading = $state(false);
let rekordboxBusy = $state(false);
let _loading = $state(false);
let scanning = $state(false);
let scanKind = $state<"folder" | "rekordbox" | "drop" | null>(null);
let _fileDropActive = $state(false);
let scanProgress = $state<ScanProgress | null>(null);
let duplicatePayload = $state<DuplicateFoundPayload | null>(null);
let _downloadProgress = $state<DownloadProgress | null>(null);
let convertProgress = $state<ConvertProgress | null>(null);
let convertTargets = $state<Track[]>([]);
let convertDialogOpen = $state(false);
let converting = $derived(convertProgress != null);

// 分類（集合層と属性層）。ライブラリタブの左側に出る。
let playlistNodes = $state<PlaylistNode[]>([]);
let _tagAxes = $state<TagAxis[]>([]);
let browseMode = $state<BrowseMode>(appSession.library.browseMode);
let selectedPlaylistId = $state<number | null>(
	appSession.library.selectedPlaylistId,
);
let expandedPlaylistIds = $state<number[]>([
	...appSession.library.expandedPlaylistIds,
]);
/** 選ばれているプレイリストの中身。列なら要素 ID が入っている。 */
let playlistItems = $state<PlaylistEntry[]>([]);
let _classifyBusy = $state(false);
/** タグパネルの操作対象。一覧の「タグ」ボタンで渡ってくる。 */
let tagTargets = $state<Track[]>([]);
let activeTagIds = $state<number[]>([]);
/** タグでの絞り込み結果。絞っていないときは null。 */
let tagNarrowed = $state<Track[] | null>(null);
let facetField = $state<FacetField>("album");
let facetValue = $state<string | null>(null);
let facetNarrowed = $state<Track[] | null>(null);

let _scanStatusLabel = $derived.by((): string => {
	if (duplicatePayload) return "重複の確認待ち";
	if (!scanning) return "";
	if (scanKind === "rekordbox") return "Rekordbox からインポート中";
	if (scanKind === "folder") return "フォルダをスキャン中";
	if (scanKind === "drop") return "ドロップしたファイルを取り込み中";
	return "ライブラリ取り込み中";
});

let _scanStatusCount = $derived.by((): string | null => {
	if (!scanProgress) return null;
	if (scanProgress.total != null && scanProgress.total > 0) {
		return `${scanProgress.processed}/${scanProgress.total}`;
	}
	return `${scanProgress.processed}`;
});

let _scanStatusDetail = $derived.by((): string | null => {
	if (!scanProgress) return null;
	return `追加 ${scanProgress.added} · スキップ ${scanProgress.skipped}`;
});

let _rekordboxPathIndex = $derived(buildRekordboxPathIndex(rekordboxTracks));
let rekordboxContentIds = $derived(buildRekordboxIdSet(rekordboxTracks));
let _rekordboxWritable = $derived(
	!!rekordboxStatus?.dbPath && !rekordboxStatus.rekordboxRunning,
);
let _rekordboxLockedHint = $derived.by((): string | null => {
	if (!rekordboxStatus?.dbPath) {
		return "Rekordbox ライブラリが見つかりません";
	}
	if (rekordboxStatus.rekordboxRunning) {
		return "Rekordbox を終了すると編集できます";
	}
	return null;
});

let selectedPlaylist = $derived(
	selectedPlaylistId == null
		? null
		: (playlistNodes.find((node) => node.id === selectedPlaylistId) ?? null),
);

/**
 * プレイリストが空なのと、ライブラリが空なのは別の状態。
 * 同じ「トラックがありません」を出すと、プレイリストを選んだだけで曲が消えたように見える。
 */
let _libraryEmptyTitle = $derived(
	selectedPlaylist
		? `「${selectedPlaylist.name}」に曲がありません`
		: "ライブラリにトラックがありません",
);
let _libraryEmptyHint = $derived(
	selectedPlaylist
		? "曲をドラッグして追加してください"
		: "フォルダを追加するか、ファイルをドロップしてください",
);

/** 絞り込みの結果。タグとブラウズは重ねて効く（両方を満たす曲だけ残る）。 */
let narrowIds = $derived.by((): Set<number> | null => {
	const groups: Set<number>[] = [];
	if (tagNarrowed) groups.push(new Set(tagNarrowed.map((track) => track.id)));
	if (facetNarrowed)
		groups.push(new Set(facetNarrowed.map((track) => track.id)));
	if (groups.length === 0) return null;
	return groups.reduce(
		(left, right) => new Set([...left].filter((id) => right.has(id))),
	);
});

let _libraryTracks = $derived(
	narrowIds ? tracks.filter((track) => narrowIds.has(track.id)) : tracks,
);

let _libraryEntries = $derived.by((): PlaylistEntry[] | null => {
	if (selectedPlaylistId == null) return null;
	if (!narrowIds) return playlistItems;
	return playlistItems.filter((entry) => narrowIds.has(entry.track.id));
});

/**
 * 並べ替えは列の要素をすべて渡す約束なので、絞り込み中は触らせない。
 * 見えている分だけ渡すと、隠れた要素が消えたことになってしまう。
 */
let _canReorder = $derived(
	selectedPlaylist?.kind === "static" && narrowIds == null,
);

let _previewTrack = $derived.by((): PreviewableTrack | null => {
	if (activeTab === "library" || activeTab === "trash") {
		return selectedTrack;
	}

	if (!selectedRekordboxId) return null;
	const content = rekordboxTracks.find(
		(track) => track.id === selectedRekordboxId,
	);
	return content ? rekordboxContentToPreview(content) : null;
});

async function loadTracks(silent = true) {
	_loading = true;
	try {
		tracks = await listTracks();
		const selectedId = selectedTrack?.id ?? appSession.selectedTrackId;
		const fromLibrary =
			selectedId != null
				? (tracks.find((t) => t.id === selectedId) ?? null)
				: null;
		const fromTrash =
			selectedId != null
				? (trashedTracks.find((t) => t.id === selectedId) ?? null)
				: null;
		selectedTrack = fromLibrary ?? fromTrash;
		appSession.selectedTrackId = selectedTrack?.id ?? null;
		persistAppSession();
		if (!silent) {
			pushActivityLog(
				"info",
				`ライブラリを読み込みました (${tracks.length} 曲)`,
			);
		}
	} catch (e) {
		pushActivityLog("error", "ライブラリの読み込みに失敗しました", String(e));
	} finally {
		_loading = false;
	}
}

async function loadTrashed(silent = true) {
	try {
		trashedTracks = await listTrashed();
		if (!silent) {
			pushActivityLog(
				"info",
				`ゴミ箱を読み込みました (${trashedTracks.length} 曲)`,
			);
		}
	} catch (e) {
		pushActivityLog("error", "ゴミ箱の読み込みに失敗しました", String(e));
	}
}

async function loadClassification(silent = true) {
	try {
		const [nodes, axes] = await Promise.all([
			playlistListTree(),
			tagListAxes(),
		]);
		playlistNodes = nodes;
		_tagAxes = axes;
		if (selectedPlaylistId != null) {
			if (nodes.some((node) => node.id === selectedPlaylistId)) {
				expandAncestorsOf(selectedPlaylistId);
			} else {
				// 消えたプレイリストを指したままにしない。
				selectPlaylist(null);
			}
		}
		if (!silent) {
			pushActivityLog(
				"info",
				`プレイリストを読み込みました (${nodes.length} 件)`,
			);
		}
	} catch (e) {
		pushActivityLog("error", "プレイリストの読み込みに失敗しました", String(e));
	}
}

async function loadPlaylistItems() {
	const id = selectedPlaylistId;
	if (id == null) {
		playlistItems = [];
		return;
	}
	try {
		playlistItems = await playlistEntries(id);
	} catch (e) {
		playlistItems = [];
		pushActivityLog(
			"error",
			"プレイリストの中身を読み込めませんでした",
			String(e),
		);
	}
}

/** 絞り込みを引き直す。曲が増減したときも呼ぶ。 */
async function reloadNarrowing() {
	try {
		tagNarrowed =
			activeTagIds.length > 0 ? await browseTagTracks(activeTagIds) : null;
		facetNarrowed = facetNarrowed
			? await browseFacetTracks(facetField, facetValue)
			: null;
	} catch (e) {
		pushActivityLog("error", "絞り込みに失敗しました", String(e));
	}
}

/** 選んだプレイリストが畳まれた枝の中にいても見えるように、祖先を開く。 */
function expandAncestorsOf(id: number) {
	const missing = ancestorIds(playlistNodes, id).filter(
		(ancestorId) => !expandedPlaylistIds.includes(ancestorId),
	);
	if (missing.length > 0) {
		expandedPlaylistIds = [...expandedPlaylistIds, ...missing];
	}
}

function selectPlaylist(id: number | null) {
	selectedPlaylistId = id;
	browseMode = id == null ? "all" : "playlist";
	if (id != null) expandAncestorsOf(id);
	void loadPlaylistItems();
}

function _togglePlaylistExpanded(id: number) {
	expandedPlaylistIds = expandedPlaylistIds.includes(id)
		? expandedPlaylistIds.filter((current) => current !== id)
		: [...expandedPlaylistIds, id];
}

async function _handlePlaylistChanged() {
	await loadClassification();
	await loadPlaylistItems();
}

async function _handleDropTracksOnPlaylist(
	playlistId: number,
	trackIds: number[],
) {
	_classifyBusy = true;
	try {
		await playlistAddTracks(playlistId, trackIds);
		const name =
			playlistNodes.find((node) => node.id === playlistId)?.name ??
			"プレイリスト";
		pushActivityLog(
			"success",
			`${name} に ${trackIds.length} 曲を追加しました`,
		);
		await loadClassification();
		if (playlistId === selectedPlaylistId) await loadPlaylistItems();
	} catch (e) {
		pushActivityLog("error", "プレイリストに追加できませんでした", String(e));
	} finally {
		_classifyBusy = false;
	}
}

async function _handleReorderEntries(entryIds: number[]) {
	const id = selectedPlaylistId;
	if (id == null) return;
	try {
		await playlistReorderEntries(id, entryIds);
		await loadPlaylistItems();
	} catch (e) {
		pushActivityLog("error", "並べ替えに失敗しました", String(e));
		await loadPlaylistItems();
	}
}

async function _handleRemoveEntries(entryIds: number[]) {
	const id = selectedPlaylistId;
	if (id == null || entryIds.length === 0) return;
	try {
		const removed = await playlistRemoveEntries(id, entryIds);
		pushActivityLog("success", `プレイリストから ${removed} 件外しました`);
		await loadClassification();
		await loadPlaylistItems();
	} catch (e) {
		pushActivityLog("error", "プレイリストから外せませんでした", String(e));
	}
}

async function _handleTagFilterChange(tagIds: number[]) {
	activeTagIds = tagIds;
	try {
		tagNarrowed = tagIds.length > 0 ? await browseTagTracks(tagIds) : null;
	} catch (e) {
		pushActivityLog("error", "タグでの絞り込みに失敗しました", String(e));
	}
}

async function _handleTagsChanged() {
	await Promise.all([loadClassification(), reloadNarrowing()]);
	await loadPlaylistItems();
	if (tagTargets.length > 0) {
		// 付け外しの結果を点灯に反映させるため、対象を作り直して読み込みを促す。
		tagTargets = [...tagTargets];
	}
}

async function _handleFacetSelect(field: FacetField, value: string | null) {
	if (field !== facetField) {
		// フィールドを変えただけ。前の絞り込みは外す。
		facetField = field;
		facetValue = null;
		facetNarrowed = null;
		return;
	}
	facetValue = value;
	try {
		facetNarrowed = await browseFacetTracks(field, value);
	} catch (e) {
		pushActivityLog("error", "ブラウズに失敗しました", String(e));
	}
}

function clearFacet() {
	facetValue = null;
	facetNarrowed = null;
}

/** タグとブラウズの絞り込みをまとめて外す。プレイリストの選択は残す。 */
function _clearNarrowing() {
	activeTagIds = [];
	tagNarrowed = null;
	clearFacet();
}

function rekordboxLockUnchanged(
	prev: RekordboxCheck | null,
	next: RekordboxCheck,
): boolean {
	return (
		prev != null &&
		prev.rekordboxRunning === next.rekordboxRunning &&
		prev.dbPath === next.dbPath
	);
}

async function refreshRekordboxStatus(): Promise<RekordboxCheck | null> {
	try {
		const next = await rekordboxCheck();
		const prev = rekordboxStatus;
		if (rekordboxLockUnchanged(prev, next)) {
			return prev;
		}

		rekordboxStatus = next;

		if (prev && prev.rekordboxRunning !== next.rekordboxRunning) {
			if (next.rekordboxRunning) {
				pushActivityLog(
					"warning",
					"Rekordbox が起動したため編集をロックしました",
				);
			} else if (next.dbPath) {
				pushActivityLog("info", "Rekordbox が終了したため編集できます");
			}
		}

		return next;
	} catch {
		return rekordboxStatus;
	}
}

async function loadRekordbox(silent = true) {
	rekordboxLoading = true;
	try {
		rekordboxStatus = await rekordboxCheck();
		rekordboxTracks = await rekordboxGetContent();
		if (selectedRekordboxId) {
			const updated = rekordboxTracks.find(
				(track) => track.id === selectedRekordboxId,
			);
			if (!updated) {
				selectedRekordboxId = null;
			}
		}
		if (!silent) {
			pushActivityLog(
				"info",
				`Rekordbox を読み込みました (${rekordboxTracks.length} 曲)`,
			);
		}
	} catch (e) {
		pushActivityLog("error", "Rekordbox の読み込みに失敗しました", String(e));
	} finally {
		rekordboxLoading = false;
	}
}

function _switchTab(tab: AppTab) {
	activeTab = tab;
	if (tab === "library") {
		void loadTracks(true);
	}
	if (
		tab === "rekordbox" &&
		rekordboxTracks.length === 0 &&
		!rekordboxLoading
	) {
		void loadRekordbox(false);
	}
	if (tab === "trash") {
		void loadTrashed(true);
	}
}

function _handleSelectRekordbox(track: RekordboxContent) {
	selectedRekordboxId = track.id;
}

async function ensureRekordboxWritable(): Promise<boolean> {
	await refreshRekordboxStatus();
	if (!rekordboxStatus?.dbPath) {
		pushActivityLog("warning", "Rekordbox ライブラリが見つかりません");
		return false;
	}
	if (rekordboxStatus.rekordboxRunning) {
		pushActivityLog(
			"warning",
			"Rekordbox が起動中です。終了してから編集してください",
		);
		return false;
	}
	return true;
}

async function _handleAddToRekordbox(selected: Track[]) {
	if (selected.length === 0 || rekordboxBusy) return;
	if (!(await ensureRekordboxWritable())) return;

	rekordboxBusy = true;
	let added = 0;
	let skipped = 0;
	let failed = 0;
	try {
		for (const track of selected) {
			if (
				track.rekordboxContentId &&
				rekordboxContentIds.has(track.rekordboxContentId)
			) {
				skipped += 1;
				continue;
			}
			try {
				await libraryAddToRekordbox(track.id);
				added += 1;
			} catch (e) {
				failed += 1;
				pushActivityLog(
					"error",
					`Rekordbox への追加に失敗: ${track.title ?? track.path}`,
					String(e),
				);
			}
		}
		await loadRekordbox(true);
		await loadTracks();
		if (added > 0) {
			pushActivityLog("success", `${added} 曲を Rekordbox に追加しました`);
		}
		if (skipped > 0) {
			pushActivityLog("info", `${skipped} 曲は既に Rekordbox に登録済みです`);
		}
		if (failed > 0 && added === 0) {
			pushActivityLog("error", `${failed} 曲の追加に失敗しました`);
		}
	} finally {
		rekordboxBusy = false;
	}
}

async function _handleRemoveFromRekordbox(selected: Track[]) {
	if (selected.length === 0 || rekordboxBusy) return;
	if (!(await ensureRekordboxWritable())) return;

	rekordboxBusy = true;
	let removed = 0;
	let failed = 0;
	try {
		for (const track of selected) {
			try {
				const didRemove = await libraryRemoveFromRekordbox(track.id);
				if (didRemove) {
					removed += 1;
				}
			} catch (e) {
				failed += 1;
				pushActivityLog(
					"error",
					`Rekordbox からの削除に失敗: ${track.title ?? track.path}`,
					String(e),
				);
			}
		}
		await loadRekordbox(true);
		await loadTracks();
		if (removed > 0) {
			pushActivityLog("success", `${removed} 曲を Rekordbox から削除しました`);
		}
		if (failed > 0 && removed === 0) {
			pushActivityLog("error", `${failed} 曲の削除に失敗しました`);
		}
	} finally {
		rekordboxBusy = false;
	}
}

function _handleOpenConvert(selected: Track[]) {
	if (selected.length === 0 || converting || scanning) return;
	convertTargets = selected;
	convertDialogOpen = true;
}

async function _handleConvertConfirm(options: ConvertOptions) {
	const ids = convertTargets.map((track) => track.id);
	convertDialogOpen = false;
	convertTargets = [];
	if (ids.length === 0) return;

	convertProgress = {
		processed: 0,
		total: ids.length,
		converted: 0,
		skipped: 0,
		failed: 0,
		currentPath: "",
		percent: 0,
		message: "変換を開始しています...",
	};

	try {
		await convertTracks(ids, options);
	} catch (e) {
		convertProgress = null;
		pushActivityLog("error", "変換を開始できませんでした", String(e));
	}
}

async function _handleAddFolder() {
	const selected = await open({
		directory: true,
		multiple: false,
		title: "Select music folder",
	});

	if (!selected || typeof selected !== "string") return;

	scanning = true;
	scanKind = "folder";
	scanProgress = null;
	pushActivityLog("info", "フォルダをスキャン中...", selected);

	try {
		await scanFolder(selected);
	} catch (e) {
		scanning = false;
		scanKind = null;
		scanProgress = null;
		pushActivityLog("error", "スキャンに失敗しました", String(e));
	}
}

async function _handleImportFromRekordbox() {
	if (!(await ensureRekordboxWritable())) return;

	scanning = true;
	scanKind = "rekordbox";
	scanProgress = {
		processed: 0,
		added: 0,
		skipped: 0,
		currentPath: "",
		total: rekordboxTracks.length > 0 ? rekordboxTracks.length : null,
	};
	pushActivityLog("info", "Rekordbox からライブラリへインポート中...");

	try {
		await importFromRekordbox();
	} catch (e) {
		scanning = false;
		scanKind = null;
		scanProgress = null;
		pushActivityLog(
			"error",
			"Rekordbox からのインポートに失敗しました",
			String(e),
		);
	}
}

function isFileDrag(event: DragEvent): boolean {
	return Array.from(event.dataTransfer?.types ?? []).includes("Files");
}

function dropImportBlocked(): boolean {
	return (
		scanning || converting || duplicatePayload != null || convertDialogOpen
	);
}

async function handleDroppedPaths(paths: string[]) {
	_fileDropActive = false;
	if (paths.length === 0) return;
	if (dropImportBlocked()) {
		pushActivityLog("warning", "取り込み中はドロップできません");
		return;
	}

	activeTab = "library";
	scanning = true;
	scanKind = "drop";
	scanProgress = null;
	pushActivityLog("info", "ドロップされたファイルを取り込み中...");

	try {
		await importPaths(paths);
	} catch (e) {
		scanning = false;
		scanKind = null;
		scanProgress = null;
		pushActivityLog("error", "取り込みに失敗しました", String(e));
	}
}

function handleSelect(track: Track) {
	selectedTrack = track;
	appSession.selectedTrackId = track.id;
	persistAppSession();
}

let _playToken = $state(0);

function _handlePlayTrack(track: Track) {
	handleSelect(track);
	_playToken += 1;
}

$effect(() => {
	appSession.activeTab = activeTab;
	appSession.selectedRekordboxId = selectedRekordboxId;
	appSession.consoleOpen = consolePanel.open;
	appSession.library.browseMode = browseMode;
	appSession.library.selectedPlaylistId = selectedPlaylistId;
	appSession.library.expandedPlaylistIds = expandedPlaylistIds;
	persistAppSession();
});

async function _handleRemove(track: Track) {
	try {
		await removeTrack(track.id);
		if (selectedTrack?.id === track.id) {
			selectedTrack = null;
			appSession.selectedTrackId = null;
		}
		pushActivityLog(
			"success",
			`ゴミ箱へ移しました: ${track.title ?? track.path}`,
			track.path,
		);
	} catch (e) {
		pushActivityLog("error", "トラックの削除に失敗しました", String(e));
	}
	await loadTracks();
	await loadTrashed();
}

async function _handleBulkRemove(ids: number[]) {
	try {
		const count = await removeTracks(ids);
		if (selectedTrack && ids.includes(selectedTrack.id)) {
			selectedTrack = null;
			appSession.selectedTrackId = null;
		}
		pushActivityLog("success", `${count} 曲をゴミ箱へ移しました`);
	} catch (e) {
		pushActivityLog("error", "トラックの一括削除に失敗しました", String(e));
	}
	await loadTracks();
	await loadTrashed();
}

async function _handleDuplicateChoice(choice: DuplicateChoice) {
	try {
		await resolveDuplicate(choice);
		duplicatePayload = null;
		const message =
			choice === "existing"
				? "既存のトラックを残しました"
				: choice === "altFormat"
					? "別フォーマットとして取り込みます"
					: "新しいトラックをライブラリに追加します";
		pushActivityLog("info", message);
	} catch (e) {
		pushActivityLog("error", "重複の解決に失敗しました", String(e));
	}
}

async function _handleRestore(track: Track) {
	try {
		await restoreTracks([track.id]);
		pushActivityLog("success", `復元しました: ${track.title ?? track.path}`);
	} catch (e) {
		pushActivityLog("error", "復元に失敗しました", String(e));
	}
	await loadTracks();
	await loadTrashed();
}

async function _handleBulkRestore(ids: number[]) {
	try {
		const count = await restoreTracks(ids);
		pushActivityLog("success", `${count} 曲を復元しました`);
	} catch (e) {
		pushActivityLog("error", "復元に失敗しました", String(e));
	}
	await loadTracks();
	await loadTrashed();
}

async function _handlePermanentDelete(ids: number[]) {
	try {
		const count = await deleteTracksPermanently(ids);
		if (selectedTrack && ids.includes(selectedTrack.id)) {
			selectedTrack = null;
			appSession.selectedTrackId = null;
		}
		pushActivityLog("success", `${count} 曲を完全に削除しました`);
	} catch (e) {
		pushActivityLog("error", "完全削除に失敗しました", String(e));
	}
	await loadTracks();
	await loadTrashed();
}

async function _handleEmptyTrash() {
	try {
		const count = await emptyTrash();
		selectedTrack = null;
		appSession.selectedTrackId = null;
		pushActivityLog("success", `ゴミ箱を空にしました (${count} 曲)`);
	} catch (e) {
		pushActivityLog("error", "ゴミ箱を空にできませんでした", String(e));
	}
	await loadTracks();
	await loadTrashed();
}

async function _handleHealthCheck() {
	try {
		const report = await checkLibraryHealth();
		if (!report.missing && !report.hashMismatch) {
			pushActivityLog(
				"success",
				`健全性チェック: ${report.checked} 曲問題なし`,
			);
			return;
		}
		pushActivityLog(
			"warning",
			`健全性チェック: 欠損 ${report.missing} · ハッシュ不一致 ${report.hashMismatch}`,
			report.issues
				.slice(0, 5)
				.map((issue) => `${issue.kind}: ${issue.path}`)
				.join(" | ") || undefined,
		);
	} catch (e) {
		pushActivityLog("error", "健全性チェックに失敗しました", String(e));
	}
}

onMount(() => {
	pushActivityLog("info", "Catra を起動しました");
	void loadTracks(false);
	void loadTrashed(true);
	void loadRekordbox(true);
	void loadClassification().then(() => loadPlaylistItems());

	let rekordboxPollInFlight = false;
	const rekordboxPollId = window.setInterval(() => {
		if (rekordboxPollInFlight || rekordboxLoading) return;
		rekordboxPollInFlight = true;
		void refreshRekordboxStatus().finally(() => {
			rekordboxPollInFlight = false;
		});
	}, REKORDBOX_RUNNING_POLL_MS);

	let unlistenUpdated: (() => void) | undefined;
	let unlistenActivity: (() => void) | undefined;
	let unlistenScanComplete: (() => void) | undefined;
	let unlistenScanError: (() => void) | undefined;
	let unlistenScanProgress: (() => void) | undefined;
	let unlistenDuplicate: (() => void) | undefined;
	let unlistenDownloadProgress: (() => void) | undefined;
	let unlistenConvertProgress: (() => void) | undefined;
	let unlistenConvertComplete: (() => void) | undefined;
	let unlistenConvertError: (() => void) | undefined;
	let unlistenDragDrop: (() => void) | undefined;

	const onWindowDragOver = (event: DragEvent) => {
		if (!isFileDrag(event)) return;
		event.preventDefault();
	};
	const onWindowDrop = (event: DragEvent) => {
		if (!isFileDrag(event)) return;
		event.preventDefault();
	};
	window.addEventListener("dragover", onWindowDragOver);
	window.addEventListener("drop", onWindowDrop);

	void getCurrentWebview()
		.onDragDropEvent((event) => {
			const payload = event.payload;
			if (payload.type === "enter" || payload.type === "over") {
				_fileDropActive = true;
				return;
			}
			if (payload.type === "drop") {
				void handleDroppedPaths(payload.paths);
				return;
			}
			_fileDropActive = false;
		})
		.then((unlisten) => {
			unlistenDragDrop = unlisten;
		})
		.catch(() => {
			// getCurrentWebview throws outside the Tauri webview.
		});

	void listen("library-updated", () => {
		void loadTracks();
		void loadTrashed();
		void loadClassification();
		void loadPlaylistItems();
		void reloadNarrowing();
	}).then((unlisten) => {
		unlistenUpdated = unlisten;
	});

	void listen<ActivityLogPayload>("activity-log", (event) => {
		pushActivityLogPayload(event.payload);
	}).then((unlisten) => {
		unlistenActivity = unlisten;
	});

	void listen<ScanResult>("library-scan-complete", (event) => {
		scanning = false;
		scanKind = null;
		scanProgress = null;
		if (event.payload.added > 0) {
			activeTab = "library";
		}
	}).then((unlisten) => {
		unlistenScanComplete = unlisten;
	});

	void listen<string>("library-scan-error", (event) => {
		scanning = false;
		scanKind = null;
		scanProgress = null;
		pushActivityLog(
			"error",
			"ライブラリの取り込みに失敗しました",
			event.payload,
		);
	}).then((unlisten) => {
		unlistenScanError = unlisten;
	});

	void listen<ScanProgress>("library-scan-progress", (event) => {
		// Clone into a new object so Svelte 5 always sees a state change.
		const next = event.payload;
		scanProgress = {
			processed: next.processed,
			added: next.added,
			skipped: next.skipped,
			currentPath: next.currentPath,
			total: next.total ?? null,
		};
	}).then((unlisten) => {
		unlistenScanProgress = unlisten;
	});

	void listen<DuplicateFoundPayload>("library-duplicate-found", (event) => {
		duplicatePayload = event.payload;
	}).then((unlisten) => {
		unlistenDuplicate = unlisten;
	});

	void listen<DownloadProgress>("download-progress", (event) => {
		const progress = event.payload;
		if (
			progress.status === "idle" &&
			progress.percent == null &&
			!progress.message
		) {
			_downloadProgress = null;
			return;
		}

		_downloadProgress = progress;
	}).then((unlisten) => {
		unlistenDownloadProgress = unlisten;
	});

	void listen<ConvertProgress>("library-convert-progress", (event) => {
		const next = event.payload;
		convertProgress = {
			processed: next.processed,
			total: next.total,
			converted: next.converted,
			skipped: next.skipped,
			failed: next.failed,
			currentPath: next.currentPath,
			percent: next.percent ?? convertProgress?.percent ?? null,
			message: next.message ?? convertProgress?.message ?? null,
		};
	}).then((unlisten) => {
		unlistenConvertProgress = unlisten;
	});

	void listen<ConvertResult>("library-convert-complete", (event) => {
		convertProgress = null;
		if (event.payload.rekordboxAdded > 0) {
			void loadRekordbox(true);
		}
	}).then((unlisten) => {
		unlistenConvertComplete = unlisten;
	});

	void listen<string>("library-convert-error", () => {
		convertProgress = null;
	}).then((unlisten) => {
		unlistenConvertError = unlisten;
	});

	return () => {
		window.clearInterval(rekordboxPollId);
		unlistenUpdated?.();
		unlistenActivity?.();
		unlistenScanComplete?.();
		unlistenScanError?.();
		unlistenScanProgress?.();
		unlistenDuplicate?.();
		unlistenDownloadProgress?.();
		unlistenConvertProgress?.();
		unlistenConvertComplete?.();
		unlistenConvertError?.();
		unlistenDragDrop?.();
		window.removeEventListener("dragover", onWindowDragOver);
		window.removeEventListener("drop", onWindowDrop);
	};
});
</script>

{#if duplicatePayload}
  <DuplicateTrackDialog payload={duplicatePayload} onchoose={handleDuplicateChoice} />
{/if}

{#if convertDialogOpen}
  <ConvertDialog
    trackCount={convertTargets.length}
    {rekordboxWritable}
    {rekordboxLockedHint}
    oncancel={() => {
      convertDialogOpen = false;
      convertTargets = [];
    }}
    onconfirm={handleConvertConfirm}
  />
{/if}

<div class="app">
  <header class="toolbar">
    <h1 class="logo">Catra</h1>
    <div class="tabs" role="tablist" aria-label="ライブラリ切替">
      <button
        type="button"
        class="tab"
        class:active={activeTab === "library"}
        role="tab"
        aria-selected={activeTab === "library"}
        onclick={() => switchTab("library")}
      >
        Library
      </button>
      <button
        type="button"
        class="tab"
        class:active={activeTab === "rekordbox"}
        role="tab"
        aria-selected={activeTab === "rekordbox"}
        onclick={() => switchTab("rekordbox")}
      >
        Rekordbox
      </button>
      <button
        type="button"
        class="tab"
        class:active={activeTab === "trash"}
        role="tab"
        aria-selected={activeTab === "trash"}
        onclick={() => switchTab("trash")}
      >
        ゴミ箱
      </button>
    </div>
    {#if scanning || duplicatePayload}
      <div class="download-status" aria-live="polite">
        <span class="download-message">
          {scanStatusLabel}
          {#if !duplicatePayload && scanProgress?.currentPath}
            — {scanProgress.currentPath}
          {/if}
        </span>
        {#if scanStatusDetail}
          <span class="download-percent">{scanStatusDetail}</span>
        {/if}
        {#if scanStatusCount}
          <span class="download-percent">{scanStatusCount}</span>
        {/if}
        {#if scanProgress?.total != null && scanProgress.total > 0}
          <progress
            class="download-bar"
            max={scanProgress.total}
            value={scanProgress.processed}
          ></progress>
        {/if}
      </div>
    {:else if convertProgress}
      <div class="download-status" aria-live="polite">
        <span class="download-message">
          {convertProgress.message ?? "変換中"}
        </span>
        {#if convertProgress.percent != null}
          <span class="download-percent">{Math.round(convertProgress.percent)}%</span>
        {:else if convertProgress.total > 0}
          <span class="download-percent">
            {convertProgress.processed}/{convertProgress.total}
          </span>
        {/if}
        {#if convertProgress.percent != null}
          <progress
            class="download-bar"
            max="100"
            value={Math.round(convertProgress.percent)}
          ></progress>
        {:else if convertProgress.total > 0}
          <progress
            class="download-bar"
            max={convertProgress.total}
            value={convertProgress.processed}
          ></progress>
        {/if}
      </div>
    {:else if downloadProgress}
      <div class="download-status" aria-live="polite">
        <span class="download-message">
          {downloadProgress.message ?? "ダウンロード中"}
        </span>
        {#if downloadProgress.percent != null}
          <span class="download-percent">{Math.round(downloadProgress.percent)}%</span>
        {:else if downloadProgress.current != null && downloadProgress.total != null}
          <span class="download-percent">
            {downloadProgress.current}/{downloadProgress.total}
          </span>
        {/if}
        {#if downloadProgress.percent != null}
          <progress
            class="download-bar"
            max="100"
            value={Math.round(downloadProgress.percent)}
          ></progress>
        {/if}
      </div>
    {/if}
    <button
      class="btn primary"
      onclick={handleAddFolder}
      disabled={loading || scanning || converting || activeTab !== "library"}
    >
      {scanning && scanKind === "folder" ? "スキャン中..." : "Add Folder"}
    </button>
    <button
      class="btn primary"
      onclick={handleImportFromRekordbox}
      disabled={loading || scanning || converting || activeTab !== "rekordbox" || !rekordboxStatus?.dbPath}
    >
      {scanning && scanKind === "rekordbox" ? "インポート中..." : "ライブラリにインポート"}
    </button>
  </header>

  <div class="body">
    <main class="content">
      <div
        class="tab-panel"
        hidden={activeTab !== "library"}
        inert={activeTab !== "library" ? true : undefined}
        aria-hidden={activeTab !== "library"}
      >
        <aside class="side">
          <PlaylistTree
            nodes={playlistNodes}
            axes={tagAxes}
            selectedId={selectedPlaylistId}
            expandedIds={expandedPlaylistIds}
            busy={classifyBusy}
            onselect={selectPlaylist}
            ontoggleexpand={togglePlaylistExpanded}
            onchanged={handlePlaylistChanged}
            onerror={(message) => pushActivityLog("error", message)}
            ondroptracks={handleDropTracksOnPlaylist}
          />
          <TagPanel
            axes={tagAxes}
            selectedTracks={tagTargets}
            {activeTagIds}
            busy={classifyBusy}
            onchanged={handleTagsChanged}
            onfilterchange={(tagIds) => void handleTagFilterChange(tagIds)}
            onerror={(message) => pushActivityLog("error", message)}
          />
          <FacetBrowser
            field={facetField}
            value={facetValue}
            active={facetNarrowed != null}
            onselect={(field, value) => void handleFacetSelect(field, value)}
            onclear={clearFacet}
            onchanged={handlePlaylistChanged}
            onerror={(message) => pushActivityLog("error", message)}
          />
        </aside>
        <TrackList
          tracks={libraryTracks}
          entries={libraryEntries}
          selectedId={selectedTrack?.id ?? null}
          sessionScope="library"
          emptyTitle={libraryEmptyTitle}
          emptyHint={libraryEmptyHint}
          {rekordboxPathIndex}
          {rekordboxContentIds}
          {rekordboxWritable}
          {rekordboxBusy}
          {rekordboxLockedHint}
          onselect={handleSelect}
          onplay={handlePlayTrack}
          onremove={handleRemove}
          onbulkremove={handleBulkRemove}
          onAddToRekordbox={handleAddToRekordbox}
          onRemoveFromRekordbox={handleRemoveFromRekordbox}
          onconvert={handleOpenConvert}
          convertBusy={converting || scanning}
          onreorder={canReorder ? handleReorderEntries : undefined}
          onremoveentries={selectedPlaylist?.kind === "static" ? handleRemoveEntries : undefined}
          ontagtracks={(selected) => (tagTargets = selected)}
        >
          {#snippet headerExtra()}
            {#if selectedPlaylist}
              <span class="scope-name">{selectedPlaylist.name}</span>
            {/if}
            {#if narrowIds}
              <button type="button" onclick={clearNarrowing}>絞り込みを外す</button>
            {/if}
            <button
              type="button"
              onclick={handleHealthCheck}
              disabled={loading || scanning || converting}
            >
              健全性チェック
            </button>
          {/snippet}
        </TrackList>
      </div>
      <div
        class="tab-panel"
        hidden={activeTab !== "rekordbox"}
        inert={activeTab !== "rekordbox" ? true : undefined}
        aria-hidden={activeTab !== "rekordbox"}
      >
        <RekordboxList
          tracks={rekordboxTracks}
          selectedId={selectedRekordboxId}
          loading={rekordboxLoading}
          status={rekordboxStatus}
          onselect={handleSelectRekordbox}
          onrefresh={() => loadRekordbox(false)}
          onensurewritable={ensureRekordboxWritable}
        />
      </div>
      <div
        class="tab-panel"
        hidden={activeTab !== "trash"}
        inert={activeTab !== "trash" ? true : undefined}
        aria-hidden={activeTab !== "trash"}
      >
        <TrackList
          tracks={trashedTracks}
          selectedId={selectedTrack?.id ?? null}
          sessionScope="trash"
          emptyTitle="ゴミ箱は空です"
          emptyHint="ライブラリから外した曲がここに入ります"
          removeTitle="復元"
          bulkRemoveLabel="復元"
          bulkRemoveConfirmMessage="曲をライブラリに戻しますか？"
          onselect={handleSelect}
          onplay={handlePlayTrack}
          onremove={handleRestore}
          onbulkremove={handleBulkRestore}
          onbulkpermanent={handlePermanentDelete}
          onemptytrash={handleEmptyTrash}
        />
      </div>
    </main>
    <ActivityConsole />
  </div>

  <StatusBar />
  <PreviewPlayer track={previewTrack} autoplayToken={playToken} />

  {#if fileDropActive}
    <div class="drop-overlay" aria-live="polite">
      <p>
        {dropImportBlocked()
          ? "取り込みが終わるまでドロップできません"
          : "楽曲ファイルまたはフォルダをドロップしてライブラリに追加"}
      </p>
    </div>
  {/if}
</div>

<style>
  .app {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.75rem 1.25rem;
    border-bottom: 1px solid var(--border);
    background: var(--surface-raised);
  }

  .logo {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 700;
    letter-spacing: -0.02em;
    flex-shrink: 0;
  }

  .tabs {
    display: flex;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
    flex-shrink: 0;
  }

  .tab {
    padding: 0.4rem 0.85rem;
    border: none;
    background: var(--surface);
    color: var(--text-muted);
    font-size: 0.82rem;
    font-weight: 500;
    cursor: pointer;
  }

  .tab:not(:last-child) {
    border-right: 1px solid var(--border);
  }

  .tab:hover {
    background: var(--surface-hover);
    color: var(--text);
  }

  .tab.active {
    background: var(--surface-selected);
    color: var(--accent);
  }

  .tab.active:hover {
    background: var(--surface-selected-hover);
  }

  .download-status {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    min-width: 0;
    flex: 1;
    padding: 0.35rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
  }

  .download-message {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.8rem;
    color: var(--text-muted);
  }

  .download-percent {
    flex-shrink: 0;
    font-size: 0.8rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    color: var(--text);
  }

  .download-bar {
    width: 120px;
    height: 6px;
    flex-shrink: 0;
    border: none;
    border-radius: 999px;
    overflow: hidden;
    background: var(--surface-hover);
    accent-color: var(--accent);
  }

  .btn {
    padding: 0.45rem 1rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
  }

  .btn:hover:not(:disabled) {
    background: var(--surface-hover);
    border-color: var(--surface-active);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }

  .btn.primary:hover:not(:disabled) {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }

  .body {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .content {
    display: flex;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .tab-panel {
    display: flex;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .tab-panel[hidden] {
    display: none;
  }

  .side {
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    width: 15rem;
    min-height: 0;
    overflow: hidden;
    border-right: 1px solid var(--border);
    background: var(--surface-raised);
  }

  .scope-name {
    max-width: 12rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--accent);
  }

  .drop-overlay {
    position: absolute;
    inset: 0;
    z-index: 40;
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
    background: var(--overlay-strong);
    border: 2px dashed color-mix(in srgb, var(--accent) 70%, var(--border));
  }

  .drop-overlay p {
    margin: 0;
    padding: 0.85rem 1.25rem;
    border-radius: 8px;
    background: var(--surface-raised);
    color: var(--text);
    font-size: 0.95rem;
    font-weight: 600;
  }
</style>
