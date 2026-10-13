import assert from "node:assert/strict";
import { test } from "node:test";
import {
	fadeMotion,
	motionDuration,
	panelMotion,
	revealMotion,
} from "../src/lib/motion.ts";

function mockEnvironment(t, { reduced = false, ...style } = {}) {
	const previousWindow = globalThis.window;
	const previousGetComputedStyle = globalThis.getComputedStyle;
	globalThis.window = {
		matchMedia: (query) => {
			assert.equal(query, "(prefers-reduced-motion: reduce)");
			return { matches: reduced };
		},
	};
	globalThis.getComputedStyle = () => ({
		display: "flex",
		opacity: "1",
		transform: "none",
		width: "360px",
		paddingLeft: "0px",
		paddingRight: "0px",
		marginLeft: "0px",
		marginRight: "0px",
		borderLeftWidth: "1px",
		borderRightWidth: "0px",
		...style,
	});
	t.after(() => {
		if (previousWindow === undefined) delete globalThis.window;
		else globalThis.window = previousWindow;
		if (previousGetComputedStyle === undefined)
			delete globalThis.getComputedStyle;
		else globalThis.getComputedStyle = previousGetComputedStyle;
	});
	return {};
}

test("motion duration is safe without a browser", () => {
	assert.equal(motionDuration(200), 200);
});

test("reveal preserves existing opacity and transforms", (t) => {
	const node = mockEnvironment(t, {
		opacity: "0.8",
		transform: "matrix(1, 0, 0, 1, 10, 0)",
	});
	const config = revealMotion(node);
	assert.equal(config.duration, 200);
	assert.match(config.css(0, 1), /opacity: 0;/);
	assert.match(config.css(0, 1), /translateY\(8px\) scale\(0.98\)/);
	assert.match(config.css(1, 0), /opacity: 0.8;/);
	assert.match(config.css(1, 0), /matrix\(1, 0, 0, 1, 10, 0\)/);
	assert.match(config.css(1, 0), /translateY\(0px\) scale\(1\)/);
});

test("metadata and command bars can reveal without scaling", (t) => {
	const node = mockEnvironment(t);
	const config = revealMotion(node, { scale: 1, y: 4, duration: 160 });
	assert.equal(config.duration, 160);
	assert.match(config.css(0, 1), /translateY\(4px\) scale\(1\)/);
});

test("console expands horizontally and fade has a short exit option", (t) => {
	const node = mockEnvironment(t);
	const panel = panelMotion(node);
	assert.equal(panel.duration, 200);
	assert.match(panel.css(0, 1), /width: 0px/);
	assert.match(panel.css(1, 0), /width: 360px/);
	assert.equal(fadeMotion(node, { duration: 100 }).duration, 100);
});

test("all Svelte transitions are immediate with reduced motion", (t) => {
	const node = mockEnvironment(t, { reduced: true });
	assert.equal(motionDuration(200), 0);
	assert.equal(fadeMotion(node).duration, 0);
	assert.equal(fadeMotion(node, { duration: 100 }).duration, 0);
	assert.equal(revealMotion(node).duration, 0);
	assert.equal(panelMotion(node).duration, 0);
});

test("motion preference is read again for each transition", (t) => {
	const node = mockEnvironment(t);
	assert.equal(revealMotion(node).duration, 200);
	globalThis.window.matchMedia = () => ({ matches: true });
	assert.equal(revealMotion(node).duration, 0);
});
