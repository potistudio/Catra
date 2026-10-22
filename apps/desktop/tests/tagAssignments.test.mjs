import assert from "node:assert/strict";
import { test } from "node:test";
import {
	mergeTagAssignments,
	summarizeTagAssignments,
} from "../src/lib/tagAssignments.ts";

const initial = () =>
	mergeTagAssignments(new Map(), [
		{ trackId: 1, tagIds: [10, 20] },
		{ trackId: 2, tagIds: [20] },
		{ trackId: 3, tagIds: [] },
	]);

test("selection highlights are computed synchronously from preloaded assignments", () => {
	const assignments = initial();
	assert.deepEqual(
		[...summarizeTagAssignments([1], assignments).counts],
		[
			[10, 1],
			[20, 1],
		],
	);
	assert.deepEqual(
		[...summarizeTagAssignments([2], assignments).counts],
		[[20, 1]],
	);
	assert.deepEqual([...summarizeTagAssignments([3], assignments).counts], []);
	assert.equal(summarizeTagAssignments([3], assignments).ready, true);
});

test("multi-selection counts distinguish all, some and unassigned tags", () => {
	const summary = summarizeTagAssignments([1, 2], initial());
	assert.equal(summary.ready, true);
	assert.equal(summary.counts.get(10), 1);
	assert.equal(summary.counts.get(20), 2);
	assert.equal(summary.counts.get(30) ?? 0, 0);
});

test("clearing selection returns an empty ready summary without any read", () => {
	const summary = summarizeTagAssignments([], new Map());
	assert.equal(summary.ready, true);
	assert.equal(summary.counts.size, 0);
});

test("unknown tracks are not confused with known empty assignments", () => {
	const summary = summarizeTagAssignments([1, 99], initial());
	assert.equal(summary.ready, false);
	assert.equal(summary.counts.size, 0);
});

test("repeated playlist rows and tag IDs count each track and tag once", () => {
	const assignments = new Map([
		[1, [10, 10, 20]],
		[2, [20]],
	]);
	const summary = summarizeTagAssignments([1, 1, 2], assignments);
	assert.deepEqual(
		[...summary.counts],
		[
			[10, 1],
			[20, 2],
		],
	);
});

test("confirmed assignment changes replace prior tags without mutating other tracks", () => {
	const original = initial();
	const rows = [
		{ trackId: 1, tagIds: [30] },
		{ trackId: 2, tagIds: [] },
	];
	const updated = mergeTagAssignments(original, rows);
	assert.deepEqual(
		[...summarizeTagAssignments([1], updated).counts],
		[[30, 1]],
	);
	assert.equal(summarizeTagAssignments([2], updated).counts.size, 0);
	assert.deepEqual(original.get(1), [10, 20]);
	assert.deepEqual(original.get(2), [20]);
	assert.deepEqual(updated.get(3), []);
	rows[0].tagIds.push(40);
	assert.deepEqual(updated.get(1), [30]);
});

test("an update for a previous selection cannot replace the current selection", () => {
	const updated = mergeTagAssignments(initial(), [
		{ trackId: 1, tagIds: [30] },
	]);
	assert.deepEqual(
		[...summarizeTagAssignments([2], updated).counts],
		[[20, 1]],
	);
	assert.deepEqual(
		[...summarizeTagAssignments([1], updated).counts],
		[[30, 1]],
	);
});

test("a full reload removes deleted tags and includes newly added tracks", () => {
	const reloaded = mergeTagAssignments(new Map(), [
		{ trackId: 1, tagIds: [] },
		{ trackId: 2, tagIds: [] },
		{ trackId: 4, tagIds: [40] },
	]);
	assert.equal(summarizeTagAssignments([1, 2], reloaded).counts.size, 0);
	assert.equal(reloaded.has(3), false);
	assert.deepEqual(
		[...summarizeTagAssignments([4], reloaded).counts],
		[[40, 1]],
	);
});
