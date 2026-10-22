import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import vm from "node:vm";

const utils = readFileSync(new URL("../src/utils.js", import.meta.url), "utf8");
const saved = readFileSync(new URL("../src/saved.js", import.meta.url), "utf8");
const background = readFileSync(
	new URL("../src/background.js", import.meta.url),
	"utf8",
);

class Element {
	constructor(tag = "DIV", attributes = {}) {
		this.tagName = tag;
		this.attributes = attributes;
		this.style = { position: "" };
		this.className = "";
		this.children = [];
		this.parentElement = null;
		this.isConnected = true;
		this.scrollLeft = this.scrollTop = this.clientLeft = this.clientTop = 0;
		this.rect = {
			left: 0,
			top: 0,
			right: 100,
			bottom: 100,
			width: 100,
			height: 100,
		};
		this.titles = [];
		this.artwork = [];
		this.classList = {
			contains: (name) => this.className.split(" ").includes(name),
		};
	}
	getAttribute(name) {
		return this.attributes[name] ?? null;
	}
	setAttribute(name, value) {
		this.attributes[name] = value;
	}
	getBoundingClientRect() {
		return this.rect;
	}
	appendChild(child) {
		child.parentElement = this;
		child.isConnected = true;
		this.children.push(child);
		return child;
	}
	remove() {
		this.isConnected = false;
		this.parentElement.children = this.parentElement.children.filter(
			(child) => child !== this,
		);
	}
	closest(selector) {
		for (let node = this; node; node = node.parentElement) {
			if (selector === "a[href]" && node.tagName === "A") return node;
			if (
				selector.includes("catra-sc-saved-marker") &&
				node.classList.contains("catra-sc-saved-marker")
			)
				return node;
			if (selector.includes("fullHero__artwork") && node.hero) return node;
		}
		return null;
	}
	querySelector(selector) {
		if (selector === ".catra-sc-saved-marker")
			return (
				this.children.find((node) =>
					node.classList.contains("catra-sc-saved-marker"),
				) ?? null
			);
		if (selector === "h1") return this.heading ?? null;
		if (selector === ".mui-16ytee5") return this.actionBar ?? null;
		return this.nestedArtwork ?? null;
	}
	querySelectorAll(selector) {
		if (selector.includes("soundTitle__title")) return this.titles;
		if (selector === "a[href]") return this.links ?? [];
		return this.artwork;
	}
	matches() {
		return this.row ?? false;
	}
}

function artwork(url = "/artist/song") {
	const host = new Element("A", { href: url });
	const cover = host.appendChild(new Element("IMG"));
	return { host, cover };
}

function environment(
	covers = [],
	handler = () => ({ success: true, savedUrls: [] }),
	href = "https://soundcloud.com/artist/song",
) {
	let now = 1000;
	let respond = handler;
	const calls = [];
	const context = vm.createContext({
		URL,
		console,
		TRACK_ACTION_CONTAINER_CLASS: "mui-16ytee5",
		Date: { now: () => now },
		location: { href, origin: "https://soundcloud.com" },
		innerHeight: 1000,
		innerWidth: 1000,
		document: {
			documentElement: new Element(),
			createElement: (tag) => new Element(tag.toUpperCase()),
		},
		collectElementsDeep: () => covers,
		getComputedStyle: (node) => ({ position: node.style.position || "static" }),
		setTimeout: () => 1,
		setInterval: () => 1,
		MutationObserver: class {
			observe() {}
		},
		Node: { ELEMENT_NODE: 1 },
		addEventListener() {},
		chrome: {
			runtime: {
				sendMessage: async (message) => {
					calls.push(JSON.parse(JSON.stringify(message)));
					return respond(message);
				},
			},
		},
	});
	context.window = context;
	vm.runInContext(utils, context);
	vm.runInContext(saved, context);
	return {
		context,
		calls,
		refresh: () => vm.runInContext("refreshSavedCovers()", context),
		invalidate: () => vm.runInContext("invalidateSavedCovers()", context),
		advance: (ms) => {
			now += ms;
		},
		handler: (next) => {
			respond = next;
		},
		normalize: (url) => context.CatraExtension.normalizeSoundCloudTrackUrl(url),
		marks: () => vm.runInContext("savedCoverMarks.size", context),
	};
}

const savedResponse = (message) => ({ success: true, savedUrls: message.urls });

test("SoundCloud track URLs are canonical across iframe, mobile and private links", () => {
	const env = environment();
	for (const url of [
		"/artist/song?si=x#player",
		"http://www.soundcloud.com/artist/song/",
		"https://m.soundcloud.com/artist/song",
		"/n/artist/song",
		"/artist/song/s-secret",
	]) {
		assert.equal(env.normalize(url), "https://soundcloud.com/artist/song");
	}
	for (const url of [
		"/artist",
		"/artist/likes",
		"/artist/sets/album",
		"https://soundcloud.com.evil.test/artist/song",
		"javascript:alert(1)",
		"https://user@soundcloud.com/artist/song",
		"/artist/song/likes",
	])
		assert.equal(env.normalize(url), null);
});

test("saved covers get one accessible click-through mark and duplicate URLs are batched", async () => {
	const first = artwork();
	const second = artwork("/n/artist/song?si=x");
	const env = environment([first.cover, second.cover], savedResponse);
	await env.refresh();
	assert.deepEqual(env.calls[0], {
		type: "GET_SAVED_TRACKS",
		urls: ["https://soundcloud.com/artist/song"],
	});
	assert.equal(env.marks(), 2);
	const badge = first.host.querySelector(".catra-sc-saved-marker");
	assert.equal(badge.getAttribute("aria-label"), "Catraに保存済み");
	assert.equal(badge.style.pointerEvents, "none");
	assert.equal(badge.style.left, "74px");
	await env.refresh();
	assert.equal(env.calls.length, 1);
	assert.equal(
		first.host.children.filter((node) =>
			node.classList.contains("catra-sc-saved-marker"),
		).length,
		1,
	);
});

test("virtualized artwork reuse removes the previous track mark", async () => {
	const item = artwork();
	const env = environment([item.cover], savedResponse);
	await env.refresh();
	item.host.setAttribute("href", "/other/track");
	env.handler(() => ({ success: true, savedUrls: [] }));
	await env.refresh();
	assert.equal(env.marks(), 0);
	assert.equal(item.host.style.position, "");
	assert.deepEqual(env.calls[1].urls, ["https://soundcloud.com/other/track"]);
});

test("a refreshed library result removes marks after trashing or deleting", async () => {
	const item = artwork();
	item.host.style.position = "absolute";
	const env = environment([item.cover], savedResponse);
	await env.refresh();
	env.advance(3001);
	env.handler(() => ({ success: true, savedUrls: [] }));
	await env.refresh();
	assert.equal(env.marks(), 0);
	assert.equal(item.host.style.position, "absolute");
});

test("Catra being unavailable retains the last confirmed state and backs off", async () => {
	const item = artwork();
	const env = environment([item.cover], savedResponse);
	await env.refresh();
	env.advance(3001);
	env.handler(() => {
		throw new Error("offline");
	});
	await env.refresh();
	assert.equal(env.marks(), 1);
	const requests = env.calls.length;
	await env.refresh();
	assert.equal(env.calls.length, requests);
	env.advance(5001);
	env.handler(() => ({ success: true, savedUrls: [] }));
	await env.refresh();
	assert.equal(env.marks(), 0);
});

test("visible covers are requested in bounded batches", async () => {
	const items = Array.from({ length: 201 }, (_, id) =>
		artwork(`/artist/song-${id}`),
	);
	const env = environment(items.map((item) => item.cover));
	await env.refresh();
	assert.deepEqual(
		env.calls.map((call) => call.urls.length),
		[200, 1],
	);
});

test("an invalidated in-flight response cannot restore a stale mark", async () => {
	const item = artwork();
	let finish;
	const env = environment(
		[item.cover],
		() =>
			new Promise((resolve) => {
				finish = resolve;
			}),
	);
	const pending = env.refresh();
	env.invalidate();
	finish({ success: true, savedUrls: ["https://soundcloud.com/artist/song"] });
	await pending;
	assert.equal(env.marks(), 0);
	env.handler(() => ({ success: true, savedUrls: [] }));
	await env.refresh();
	assert.equal(env.marks(), 0);
});

test("main artwork in an embedded track page uses the current permalink", async () => {
	const cover = new Element();
	cover.hero = true;
	const env = environment(
		[cover],
		savedResponse,
		"https://soundcloud.com/n/artist/song?embedded=true",
	);
	await env.refresh();
	assert.equal(env.marks(), 1);
	assert.deepEqual(env.calls[0].urls, ["https://soundcloud.com/artist/song"]);
});

test("row artwork follows its own title rather than the current track page", async () => {
	const row = new Element();
	row.titles = [new Element("A", { href: "/other/song" })];
	const cover = row.appendChild(new Element("IMG"));
	const env = environment([cover], savedResponse);
	await env.refresh();
	assert.deepEqual(env.calls[0].urls, ["https://soundcloud.com/other/song"]);
	assert.equal(env.marks(), 1);
});

test("Mui action-bar rows resolve links outside the artwork wrapper", async () => {
	const row = new Element();
	row.actionBar = new Element();
	row.links = [
		new Element("A", { href: "/artist" }),
		new Element("A", { href: "/artist/other-song" }),
	];
	const wrapper = row.appendChild(new Element());
	wrapper.row = true;
	const cover = wrapper.appendChild(new Element("IMG"));
	const env = environment([cover], savedResponse);
	await env.refresh();
	assert.deepEqual(env.calls[0].urls, [
		"https://soundcloud.com/artist/other-song",
	]);
	assert.equal(env.marks(), 1);
});

test("ambiguous rows and playlist artwork do not receive a guessed mark", async () => {
	const row = new Element();
	row.titles = [
		new Element("A", { href: "/a/song" }),
		new Element("A", { href: "/b/song" }),
	];
	const ambiguous = row.appendChild(new Element("IMG"));
	const playlist = artwork("/artist/sets/album");
	const env = environment([ambiguous, playlist.cover], savedResponse);
	await env.refresh();
	assert.equal(env.calls.length, 0);
	assert.equal(env.marks(), 0);
});

test("Mui-style main artwork can be identified by a single cover next to h1", async () => {
	const host = new Element();
	const cover = host.appendChild(new Element("IMG"));
	host.heading = new Element("H1");
	host.artwork = [cover];
	const env = environment([cover], savedResponse);
	await env.refresh();
	assert.equal(env.marks(), 1);
});

test("offscreen artwork releases its mark and reuses a valid cache on return", async () => {
	const item = artwork();
	const env = environment([item.cover], savedResponse);
	await env.refresh();
	item.cover.rect = { ...item.cover.rect, top: 2000, bottom: 2100 };
	await env.refresh();
	assert.equal(env.marks(), 0);
	item.cover.rect = { ...item.cover.rect, top: 0, bottom: 100 };
	await env.refresh();
	assert.equal(env.marks(), 1);
	assert.equal(env.calls.length, 1);
});

test("the background bridge queries only the loopback membership endpoint", async () => {
	const calls = [];
	const context = vm.createContext({
		AbortSignal,
		chrome: { runtime: { onMessage: { addListener() {} } } },
		fetch: async (url, options) => {
			calls.push({ url, options });
			return { ok: true, json: async () => ({ success: true, savedUrls: [] }) };
		},
	});
	vm.runInContext(background, context);
	await vm.runInContext(
		'getSavedTracks(["https://soundcloud.com/artist/song"])',
		context,
	);
	assert.equal(calls.length, 1);
	assert.equal(calls[0].url, "http://127.0.0.1:17340/library/saved-tracks");
	assert.equal(calls[0].options.method, "POST");
	assert.deepEqual(JSON.parse(calls[0].options.body), {
		urls: ["https://soundcloud.com/artist/song"],
	});
	await assert.rejects(
		vm.runInContext('getSavedTracks(new Array(201).fill("url"))', context),
	);
	assert.equal(calls.length, 1);
});
