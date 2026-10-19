import type { AppTab } from "./appSession.svelte";
import type { ConvertFormat } from "./types";

export const SETTINGS_STORAGE_KEY = "catra.settings.v1";

export const STARTUP_TABS = [
	{ value: "restore", label: "前回開いていたタブ" },
	{ value: "library", label: "Library" },
	{ value: "rekordbox", label: "Rekordbox" },
	{ value: "trash", label: "ゴミ箱" },
] as const;

export const CONVERT_FORMATS: { value: ConvertFormat; label: string }[] = [
	{ value: "mp3", label: "MP3" },
	{ value: "aac", label: "AAC (M4A)" },
	{ value: "flac", label: "FLAC" },
	{ value: "wav", label: "WAV" },
	{ value: "aiff", label: "AIFF" },
	{ value: "ogg", label: "OGG Vorbis" },
	{ value: "opus", label: "Opus" },
];

export const CONVERT_BITRATES = [128, 192, 256, 320] as const;

export interface AppSettings {
	startupTab: "restore" | AppTab;
	previewVolume: number;
	previewMasterTempo: boolean;
	convertFormat: ConvertFormat;
	convertBitrateKbps: (typeof CONVERT_BITRATES)[number];
	convertAddToRekordbox: boolean;
}

export function defaultSettings(): AppSettings {
	return {
		startupTab: "restore",
		previewVolume: 100,
		previewMasterTempo: true,
		convertFormat: "mp3",
		convertBitrateKbps: 320,
		convertAddToRekordbox: true,
	};
}

export function isLossyFormat(format: ConvertFormat): boolean {
	return ["mp3", "aac", "ogg", "opus"].includes(format);
}

export function parseSettings(raw: unknown): AppSettings {
	const next = defaultSettings();
	if (!raw || typeof raw !== "object") return next;
	const record = raw as Record<string, unknown>;
	const startupTab = STARTUP_TABS.find(
		(tab) => tab.value === record.startupTab,
	);
	if (startupTab) next.startupTab = startupTab.value;
	if (
		typeof record.previewVolume === "number" &&
		Number.isFinite(record.previewVolume) &&
		record.previewVolume >= 0 &&
		record.previewVolume <= 100
	) {
		next.previewVolume = Math.round(record.previewVolume);
	}
	if (typeof record.previewMasterTempo === "boolean") {
		next.previewMasterTempo = record.previewMasterTempo;
	}
	const format = CONVERT_FORMATS.find(
		(item) => item.value === record.convertFormat,
	);
	if (format) next.convertFormat = format.value;
	const bitrate = CONVERT_BITRATES.find(
		(rate) => rate === record.convertBitrateKbps,
	);
	if (bitrate) next.convertBitrateKbps = bitrate;
	if (typeof record.convertAddToRekordbox === "boolean") {
		next.convertAddToRekordbox = record.convertAddToRekordbox;
	}
	return next;
}

export function readSettings(storage?: Pick<Storage, "getItem">): AppSettings {
	try {
		const raw = (storage ?? globalThis.localStorage)?.getItem(
			SETTINGS_STORAGE_KEY,
		);
		return raw ? parseSettings(JSON.parse(raw)) : defaultSettings();
	} catch {
		return defaultSettings();
	}
}

export function writeSettings(
	settings: AppSettings,
	storage?: Pick<Storage, "setItem">,
): AppSettings {
	const next = parseSettings(settings);
	const target = storage ?? globalThis.localStorage;
	if (!target) throw new Error("設定の保存先を利用できません");
	target.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(next));
	return next;
}

export function resolveStartupTab(
	settings: AppSettings,
	previousTab: AppTab,
): AppTab {
	return settings.startupTab === "restore" ? previousTab : settings.startupTab;
}
