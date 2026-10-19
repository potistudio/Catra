import assert from "node:assert/strict";
import { test } from "node:test";
import {
	clampSidebarWidth,
	defaultSidebarLayout,
	parseSidebarLayout,
	SIDEBAR_DEFAULT_WIDTH,
	SIDEBAR_MAX_WIDTH,
	SIDEBAR_MIN_WIDTH,
	sidebarMaxWidth,
} from "../src/lib/sidebarLayout.ts";

test("sidebar defaults provide a wider layout with all sections open", () => {
	assert.deepEqual(defaultSidebarLayout(), {
		width: 280,
		playlistsOpen: true,
		tagsOpen: true,
		facetsOpen: true,
	});
	const layout = defaultSidebarLayout();
	layout.tagsOpen = false;
	assert.equal(defaultSidebarLayout().tagsOpen, true);
});

test("saved width and collapsed sections are restored independently", () => {
	const saved = {
		width: 360,
		playlistsOpen: false,
		tagsOpen: true,
		facetsOpen: false,
	};
	assert.deepEqual(
		parseSidebarLayout(JSON.parse(JSON.stringify(saved))),
		saved,
	);
	assert.deepEqual(parseSidebarLayout({ tagsOpen: false }), {
		...defaultSidebarLayout(),
		tagsOpen: false,
	});
});

test("missing and invalid sidebar state preserve defaults", () => {
	for (const raw of [undefined, null, [], false, 42, "invalid"]) {
		assert.deepEqual(parseSidebarLayout(raw), defaultSidebarLayout());
	}
	assert.deepEqual(
		parseSidebarLayout({
			width: "360",
			playlistsOpen: 0,
			tagsOpen: "false",
			facetsOpen: null,
		}),
		defaultSidebarLayout(),
	);
	for (const width of [Number.NaN, Number.POSITIVE_INFINITY]) {
		assert.equal(parseSidebarLayout({ width }).width, SIDEBAR_DEFAULT_WIDTH);
	}
});

test("width is rounded and constrained on restore and resize", () => {
	assert.equal(clampSidebarWidth(320.6), 321);
	for (const width of [-100, 0, SIDEBAR_MIN_WIDTH]) {
		assert.equal(clampSidebarWidth(width), SIDEBAR_MIN_WIDTH);
		assert.equal(parseSidebarLayout({ width }).width, SIDEBAR_MIN_WIDTH);
	}
	for (const width of [SIDEBAR_MAX_WIDTH, 10000]) {
		assert.equal(clampSidebarWidth(width), SIDEBAR_MAX_WIDTH);
		assert.equal(parseSidebarLayout({ width }).width, SIDEBAR_MAX_WIDTH);
	}
	assert.equal(clampSidebarWidth(450, 320), 320);
});

test("available width reserves track space without losing preferred width", () => {
	assert.equal(sidebarMaxWidth(1400), SIDEBAR_MAX_WIDTH);
	assert.equal(sidebarMaxWidth(680), 360);
	assert.equal(sidebarMaxWidth(400), SIDEBAR_MIN_WIDTH);
	const saved = parseSidebarLayout({ width: 440 });
	assert.equal(clampSidebarWidth(saved.width, sidebarMaxWidth(680)), 360);
	assert.equal(clampSidebarWidth(saved.width, sidebarMaxWidth(1400)), 440);
	assert.equal(saved.width, 440);
});
