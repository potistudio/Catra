const SAVED_COVER_MARK = "catra-sc-saved-marker";
const SAVED_LOOKUP_INTERVAL = 3000;
const SAVED_COVER_SELECTOR = [
	".fullHero__artwork",
	".sound__artwork",
	".soundBadge__artwork",
	".playableTile__artwork",
	".playableTile__artworkLink",
	"[data-testid='artwork']",
	"[data-testid='track-artwork']",
	"img[src*='sndcdn.com/artworks']",
	"[style*='sndcdn.com/artworks']",
].join(",");
const SAVED_TITLE_SELECTOR =
	"a.soundTitle__title, a.playableTile__mainHeading, a.playableTile__heading, a[data-testid*='title']";
const savedCoverCache = new Map();
const savedCoverMarks = new Map();
const savedCoverPositions = new WeakMap();
let savedCoverTimer = null;
let savedLookupRunning = false;
let savedLookupRetryAt = 0;
let savedLookupRevision = 0;

function savedCoverTrackUrl(cover) {
	const normalize = CatraExtension.normalizeSoundCloudTrackUrl;
	const direct = cover.closest("a[href]");
	if (direct && !direct.getAttribute("href")?.startsWith("#")) {
		return normalize(direct.getAttribute("href"));
	}
	if (cover.closest(".fullHero__artwork, [data-testid='track-header']")) {
		const pageUrl = normalize(location.href);
		if (pageUrl) return pageUrl;
	}
	let parent = cover.parentElement;
	for (
		let depth = 0;
		parent && depth < 7;
		depth += 1, parent = parent.parentElement
	) {
		const titles = [...parent.querySelectorAll(SAVED_TITLE_SELECTOR)];
		const urls = new Set(
			titles
				.map((link) => normalize(link.getAttribute("href")))
				.filter(Boolean),
		);
		if (titles.length > 0) return urls.size === 1 ? [...urls][0] : null;
		if (parent.querySelector("h1")) {
			const artwork = [...parent.querySelectorAll(SAVED_COVER_SELECTOR)].filter(
				(element) => !element.querySelector(SAVED_COVER_SELECTOR),
			);
			if (artwork.length === 1 && artwork[0] === cover) {
				const pageUrl = normalize(location.href);
				if (pageUrl) return pageUrl;
			}
		}
		if (
			parent.matches(
				"article, tr, li, .sound, .soundList__item, .soundBadge, .playableTile, [data-testid*='track'], [class*='item'], [class*='row'], [class*='track'], [class*='TrackItem']",
			) ||
			parent.querySelector(`.${TRACK_ACTION_CONTAINER_CLASS}`)
		) {
			const links = parent.querySelectorAll("a[href]");
			for (const link of links) {
				const url = normalize(link.getAttribute("href"));
				if (url) urls.add(url);
			}
			if (urls.size > 0 || links.length > 0)
				return urls.size === 1 ? [...urls][0] : null;
		}
	}
	return null;
}

function savedCoverCandidates() {
	const covers = new Map();
	for (const cover of collectElementsDeep(
		document.documentElement,
		SAVED_COVER_SELECTOR,
	)) {
		if (cover.querySelector(SAVED_COVER_SELECTOR)) continue;
		const rect = cover.getBoundingClientRect();
		if (
			rect.width < 24 ||
			rect.height < 24 ||
			rect.bottom < -200 ||
			rect.top > innerHeight + 200 ||
			rect.right < 0 ||
			rect.left > innerWidth
		)
			continue;
		const url = savedCoverTrackUrl(cover);
		if (url) covers.set(cover, url);
	}
	return covers;
}

function removeSavedCoverMark(cover) {
	const record = savedCoverMarks.get(cover);
	if (!record) return;
	record.badge.remove();
	if (
		!record.host.querySelector(`.${SAVED_COVER_MARK}`) &&
		savedCoverPositions.has(record.host)
	) {
		if (record.host.style.position === "relative") {
			record.host.style.position = savedCoverPositions.get(record.host);
		}
		savedCoverPositions.delete(record.host);
	}
	savedCoverMarks.delete(cover);
}

function renderSavedCoverMarks(covers) {
	for (const [cover, record] of savedCoverMarks) {
		if (
			!cover.isConnected ||
			covers.get(cover) !== record.url ||
			!savedCoverCache.get(record.url)?.saved
		) {
			removeSavedCoverMark(cover);
		}
	}
	for (const [cover, url] of covers) {
		if (!savedCoverCache.get(url)?.saved) continue;
		let record = savedCoverMarks.get(cover);
		if (record && !record.badge.isConnected) {
			removeSavedCoverMark(cover);
			record = null;
		}
		if (!record) {
			const host = cover.tagName === "IMG" ? cover.parentElement : cover;
			if (!host) continue;
			if (getComputedStyle(host).position === "static") {
				savedCoverPositions.set(host, host.style.position);
				host.style.position = "relative";
			}
			const badge = document.createElement("span");
			badge.className = SAVED_COVER_MARK;
			badge.textContent = "✓";
			badge.title = "Catraに保存済み";
			badge.setAttribute("role", "img");
			badge.setAttribute("aria-label", "Catraに保存済み");
			// Shadow DOM 内でも、クリックを妨げない同じ表示を適用する。
			Object.assign(badge.style, {
				position: "absolute",
				display: "flex",
				alignItems: "center",
				justifyContent: "center",
				boxSizing: "border-box",
				borderRadius: "50%",
				background: "#18854b",
				color: "#fff",
				border: "2px solid #fff",
				boxShadow: "0 1px 5px #0008",
				zIndex: "5",
				fontFamily: "Arial, sans-serif",
				fontWeight: "700",
				lineHeight: "1",
				pointerEvents: "none",
			});
			host.appendChild(badge);
			record = { host, badge, url };
			savedCoverMarks.set(cover, record);
		}
		const rect = cover.getBoundingClientRect();
		const hostRect = record.host.getBoundingClientRect();
		const size = Math.min(26, Math.max(16, rect.width / 5));
		const gap = rect.width >= 80 ? 6 : 2;
		Object.assign(record.badge.style, {
			left: `${rect.right - hostRect.left + record.host.scrollLeft - record.host.clientLeft - size - gap}px`,
			top: `${rect.bottom - hostRect.top + record.host.scrollTop - record.host.clientTop - size - gap}px`,
			width: `${size}px`,
			height: `${size}px`,
			fontSize: `${size * 0.65}px`,
		});
	}
}

async function refreshSavedCovers() {
	const covers = savedCoverCandidates();
	renderSavedCoverMarks(covers);
	if (savedLookupRunning || Date.now() < savedLookupRetryAt) return;
	const urls = [...new Set(covers.values())].filter(
		(url) => (savedCoverCache.get(url)?.expires ?? 0) <= Date.now(),
	);
	if (urls.length === 0) return;
	savedLookupRunning = true;
	const revision = savedLookupRevision;
	try {
		for (let start = 0; start < urls.length; start += 200) {
			const batch = urls.slice(start, start + 200);
			const result = await chrome.runtime.sendMessage({
				type: "GET_SAVED_TRACKS",
				urls: batch,
			});
			if (!result?.success || !Array.isArray(result.savedUrls))
				throw new Error("Library lookup unavailable");
			if (revision !== savedLookupRevision) return;
			const saved = new Set(
				result.savedUrls
					.map(CatraExtension.normalizeSoundCloudTrackUrl)
					.filter(Boolean),
			);
			for (const url of batch) {
				savedCoverCache.delete(url);
				savedCoverCache.set(url, {
					saved: saved.has(url),
					expires: Date.now() + SAVED_LOOKUP_INTERVAL,
				});
			}
			while (savedCoverCache.size > 2000)
				savedCoverCache.delete(savedCoverCache.keys().next().value);
			renderSavedCoverMarks(savedCoverCandidates());
		}
	} catch {
		// 連携が利用できないときは、最後に確認した保存状態を維持する。
		savedLookupRetryAt = Date.now() + 5000;
	} finally {
		savedLookupRunning = false;
	}
}

function scheduleSavedCoverRefresh() {
	if (savedCoverTimer !== null) return;
	savedCoverTimer = setTimeout(() => {
		savedCoverTimer = null;
		void refreshSavedCovers();
	}, 100);
}

function invalidateSavedCovers() {
	savedLookupRevision += 1;
	savedLookupRetryAt = 0;
	for (const cached of savedCoverCache.values()) cached.expires = 0;
	scheduleSavedCoverRefresh();
}

function startSavedCoverMarkers() {
	const observer = new MutationObserver((mutations) => {
		if (
			mutations.some((mutation) => {
				if (mutation.target.closest?.(`.${SAVED_COVER_MARK}`)) return false;
				if (mutation.type === "attributes") return true;
				return [...mutation.addedNodes, ...mutation.removedNodes].some(
					(node) =>
						node.nodeType === Node.ELEMENT_NODE &&
						!node.classList.contains(SAVED_COVER_MARK),
				);
			})
		)
			scheduleSavedCoverRefresh();
	});
	observer.observe(document.documentElement, {
		childList: true,
		subtree: true,
		attributes: true,
		attributeFilter: ["href", "src", "style"],
	});
	window.addEventListener("scroll", scheduleSavedCoverRefresh, {
		capture: true,
		passive: true,
	});
	window.addEventListener("resize", scheduleSavedCoverRefresh, {
		passive: true,
	});
	window.addEventListener("catra-library-updated", invalidateSavedCovers);
	setInterval(() => void refreshSavedCovers(), SAVED_LOOKUP_INTERVAL);
	scheduleSavedCoverRefresh();
}

startSavedCoverMarkers();
