import assert from "node:assert/strict";
import { test } from "node:test";
import {
	getCheckedTracks,
	selectTrackKeys,
} from "../src/lib/trackSelection.ts";

const visibleKeys = [30, 10, 40, 20];
const modifiers = { ctrlKey: false, metaKey: false, shiftKey: false };

function select(checkedKeys, key, anchorKey = null, event = {}) {
	return selectTrackKeys(visibleKeys, checkedKeys, key, anchorKey, {
		...modifiers,
		...event,
	});
}

test("normal selection replaces the previous selection", () => {
	assert.deepEqual(select(new Set([30, 40]), 10), new Set([10]));
	assert.deepEqual(select(new Set([10]), 10), new Set([10]));
});

test("Ctrl adds or removes only the clicked row", () => {
	const checked = new Set([30, 40]);
	assert.deepEqual(
		select(checked, 10, 30, { ctrlKey: true }),
		new Set([30, 40, 10]),
	);
	assert.deepEqual(select(checked, 30, 30, { ctrlKey: true }), new Set([40]));
	assert.deepEqual(checked, new Set([30, 40]));
});

test("Ctrl can deselect the last selected row", () => {
	assert.deepEqual(select(new Set([10]), 10, 10, { ctrlKey: true }), new Set());
});

test("Command supports additive selection", () => {
	assert.deepEqual(
		select(new Set([30]), 10, 30, { metaKey: true }),
		new Set([30, 10]),
	);
});

test("Shift selects an inclusive range in the displayed order", () => {
	assert.deepEqual(
		select(new Set([20]), 40, 30, { shiftKey: true }),
		new Set([30, 10, 40]),
	);
	assert.deepEqual(
		select(new Set([30]), 10, 20, { shiftKey: true }),
		new Set([10, 40, 20]),
	);
});

test("Ctrl+Shift preserves the previous selection and adds the range", () => {
	assert.deepEqual(
		select(new Set([30]), 20, 40, { ctrlKey: true, shiftKey: true }),
		new Set([30, 40, 20]),
	);
});

test("Shift without a visible anchor falls back to single selection", () => {
	for (const anchor of [null, 999]) {
		assert.deepEqual(
			select(new Set([30]), 10, anchor, { shiftKey: true }),
			new Set([10]),
		);
	}
});

test("selection uses row keys for repeated playlist tracks", () => {
	assert.deepEqual(
		selectTrackKeys([101, 102], new Set([101]), 102, 101, {
			...modifiers,
			ctrlKey: true,
		}),
		new Set([101, 102]),
	);
});

test("checked tracks follow selection and clearing without a separate action", () => {
	const first = { id: 30 };
	const second = { id: 10 };
	const rows = [
		{ key: 30, track: first },
		{ key: 10, track: second },
	];
	let checked = select(new Set(), 30);
	assert.deepEqual(getCheckedTracks(rows, checked), [first]);
	checked = select(checked, 10, 30, { ctrlKey: true });
	assert.deepEqual(getCheckedTracks(rows, checked), [first, second]);
	checked = select(checked, 30, 10, { ctrlKey: true });
	assert.deepEqual(getCheckedTracks(rows, checked), [second]);
	assert.deepEqual(getCheckedTracks(rows, new Set()), []);
});

test("repeated playlist tracks are targeted once using checked row keys", () => {
	const first = { id: 7 };
	const second = { id: 8 };
	const rows = [
		{ key: 101, track: first },
		{ key: 102, track: first },
		{ key: 103, track: second },
	];
	assert.deepEqual(getCheckedTracks(rows, new Set([102])), [first]);
	assert.deepEqual(getCheckedTracks(rows, new Set([101, 102, 103])), [
		first,
		second,
	]);
	assert.deepEqual(getCheckedTracks(rows, new Set([7, 8])), []);
});

test("removed or filtered rows cannot remain checked track targets", () => {
	const track = { id: 7 };
	const rows = [{ key: 101, track }];
	const checked = new Set([101, 999]);
	assert.deepEqual(getCheckedTracks(rows, checked), [track]);
	assert.deepEqual(getCheckedTracks([], checked), []);
	assert.deepEqual(checked, new Set([101, 999]));
});

test("checked tracks use current metadata and displayed source order", () => {
	const checked = new Set([20, 10]);
	const rows = [
		{ key: 10, track: { id: 10, title: "updated" } },
		{ key: 20, track: { id: 20, title: "second" } },
	];
	assert.deepEqual(
		getCheckedTracks(rows, checked),
		rows.map((row) => row.track),
	);
});

test("selection without an event selects a single row", () => {
	assert.deepEqual(
		selectTrackKeys(visibleKeys, new Set([30, 40]), 10, null),
		new Set([10]),
	);
});
