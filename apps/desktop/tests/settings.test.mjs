import assert from "node:assert/strict";
import { test } from "node:test";
import {
	CONVERT_BITRATES,
	CONVERT_FORMATS,
	defaultSettings,
	isLossyFormat,
	parseSettings,
	readSettings,
	resolveStartupTab,
	SETTINGS_STORAGE_KEY,
	writeSettings,
} from "../src/lib/settings.ts";

function memoryStorage() {
	const values = new Map();
	return {
		getItem: (key) => values.get(key) ?? null,
		setItem: (key, value) => values.set(key, value),
	};
}

test("defaults preserve existing startup, playback and conversion behavior", () => {
	assert.deepEqual(defaultSettings(), {
		startupTab: "restore",
		previewVolume: 100,
		previewMasterTempo: true,
		convertFormat: "mp3",
		convertBitrateKbps: 320,
		convertAddToRekordbox: true,
	});
	const first = defaultSettings();
	first.previewVolume = 0;
	assert.equal(defaultSettings().previewVolume, 100);
});

test("settings are restored from persistent storage", () => {
	const storage = memoryStorage();
	const settings = {
		startupTab: "rekordbox",
		previewVolume: 35,
		previewMasterTempo: false,
		convertFormat: "flac",
		convertBitrateKbps: 192,
		convertAddToRekordbox: false,
	};
	assert.deepEqual(writeSettings(settings, storage), settings);
	assert.deepEqual(readSettings(storage), settings);
	assert.deepEqual(JSON.parse(storage.getItem(SETTINGS_STORAGE_KEY)), settings);
});

test("missing, corrupted and unavailable storage use defaults", () => {
	const storage = memoryStorage();
	assert.deepEqual(readSettings(storage), defaultSettings());
	for (const value of ["{", "null", "[]", '"invalid"']) {
		storage.setItem(SETTINGS_STORAGE_KEY, value);
		assert.deepEqual(readSettings(storage), defaultSettings());
	}
	assert.deepEqual(
		readSettings({
			getItem: () => {
				throw new Error("Access denied");
			},
		}),
		defaultSettings(),
	);
});

test("invalid fields fall back independently without losing valid fields", () => {
	assert.deepEqual(
		parseSettings({
			startupTab: "unknown",
			previewVolume: -1,
			previewMasterTempo: false,
			convertFormat: "exe",
			convertBitrateKbps: 999,
			convertAddToRekordbox: "false",
			unknownSetting: true,
		}),
		{ ...defaultSettings(), previewMasterTempo: false },
	);
});

test("volume accepts silence and full volume, rounds and rejects invalid values", () => {
	for (const value of [0, 50, 100]) {
		assert.equal(parseSettings({ previewVolume: value }).previewVolume, value);
	}
	assert.equal(parseSettings({ previewVolume: 42.6 }).previewVolume, 43);
	for (const value of [
		-1,
		101,
		Number.NaN,
		Number.POSITIVE_INFINITY,
		"50",
		null,
	]) {
		assert.equal(parseSettings({ previewVolume: value }).previewVolume, 100);
	}
});

test("all supported conversion formats and bitrates are accepted", () => {
	for (const { value } of CONVERT_FORMATS) {
		assert.equal(parseSettings({ convertFormat: value }).convertFormat, value);
	}
	for (const value of CONVERT_BITRATES) {
		assert.equal(
			parseSettings({ convertBitrateKbps: value }).convertBitrateKbps,
			value,
		);
	}
	for (const format of ["mp3", "aac", "ogg", "opus"]) {
		assert.equal(isLossyFormat(format), true);
	}
	for (const format of ["flac", "wav", "aiff"]) {
		assert.equal(isLossyFormat(format), false);
	}
});

test("startup can restore the previous tab or select an explicit tab", () => {
	for (const previous of ["library", "rekordbox", "trash"]) {
		assert.equal(resolveStartupTab(defaultSettings(), previous), previous);
		for (const startupTab of ["library", "rekordbox", "trash"]) {
			assert.equal(
				resolveStartupTab(parseSettings({ startupTab }), previous),
				startupTab,
			);
		}
	}
});

test("failed persistence is reported and does not mutate the supplied settings", () => {
	const settings = defaultSettings();
	const before = { ...settings };
	assert.throws(
		() =>
			writeSettings(settings, {
				setItem: () => {
					throw new Error("Quota exceeded");
				},
			}),
		/Quota exceeded/,
	);
	assert.deepEqual(settings, before);
});
