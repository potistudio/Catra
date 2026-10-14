import assert from "node:assert/strict";
import { test } from "node:test";
import {
	audioFilePaths,
	hasTrackDrag,
	hasTrackPathsDrag,
	readTrackDrag,
	readTrackPathsDrag,
	TRACK_DRAG_MIME,
	TRACK_PATHS_DRAG_MIME,
	writeTrackDrag,
} from "../src/lib/trackDrag.ts";

function dragEvent() {
	const data = new Map();
	return {
		dataTransfer: {
			get types() {
				return [...data.keys()];
			},
			setData: (type, value) => data.set(type, value),
			getData: (type) => data.get(type) ?? "",
		},
	};
}

test("track drags retain IDs and paths in selection order, including duplicates", () => {
	const event = dragEvent();
	const ids = [3, 1, 3];
	const paths = ["C:\\音楽\\a.mp3", "C:\\音楽\\b.flac", "C:\\音楽\\a.mp3"];
	writeTrackDrag(event, ids, paths);
	assert.equal(hasTrackDrag(event), true);
	assert.equal(hasTrackPathsDrag(event), true);
	assert.deepEqual(readTrackDrag(event), ids);
	assert.deepEqual(readTrackPathsDrag(event), paths);
});

test("legacy ID-only drags are not treated as filesystem paths", () => {
	const event = dragEvent();
	writeTrackDrag(event, [0, 1]);
	assert.equal(hasTrackDrag(event), true);
	assert.equal(hasTrackPathsDrag(event), false);
	assert.deepEqual(readTrackPathsDrag(event), []);
});

test("path payloads are independent of list-local numeric IDs", () => {
	const event = dragEvent();
	event.dataTransfer.setData(TRACK_DRAG_MIME, "[0]");
	event.dataTransfer.setData(TRACK_PATHS_DRAG_MIME, '["C:/Music/track.mp3"]');
	assert.deepEqual(readTrackPathsDrag(event), ["C:/Music/track.mp3"]);
});

test("invalid path payloads are ignored", () => {
	for (const payload of ["", "not json", "null", "42", "{}", '"a.mp3"']) {
		const event = dragEvent();
		event.dataTransfer.setData(TRACK_PATHS_DRAG_MIME, payload);
		assert.deepEqual(readTrackPathsDrag(event), []);
	}
});

test("non-string and blank paths are excluded", () => {
	const event = dragEvent();
	event.dataTransfer.setData(
		TRACK_PATHS_DRAG_MIME,
		JSON.stringify([null, 0, {}, "", " ", "C:/Music/a.mp3"]),
	);
	assert.deepEqual(readTrackPathsDrag(event), ["C:/Music/a.mp3"]);
});

test("missing DataTransfer is safe", () => {
	const event = { dataTransfer: null };
	assert.doesNotThrow(() => writeTrackDrag(event, [1], ["a.mp3"]));
	assert.equal(hasTrackPathsDrag(event), false);
	assert.deepEqual(readTrackPathsDrag(event), []);
});

test("audio file filtering matches supported formats regardless of case", () => {
	const paths = [
		"a.mp3",
		"b.WAV",
		"c.flac",
		"d.aiff",
		"e.AIF",
		"f.m4a",
		"g.AAC",
	];
	assert.deepEqual(audioFilePaths(paths), paths);
});

test("non-audio files are excluded without changing audio order or duplicates", () => {
	assert.deepEqual(
		audioFilePaths([
			"a.mp3",
			"cover.png",
			"folder",
			"b.flac",
			"a.mp3",
			"a.mp3.exe",
		]),
		["a.mp3", "b.flac", "a.mp3"],
	);
});
