import assert from "node:assert/strict";
import { test } from "node:test";
import { selectTrackKeys } from "../src/lib/trackSelection.ts";

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

test("selection without an event selects a single row", () => {
	assert.deepEqual(
		selectTrackKeys(visibleKeys, new Set([30, 40]), 10, null),
		new Set([10]),
	);
});
