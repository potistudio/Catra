const YOUTUBE_BUTTON_ID = "catra-yt-download";
const POLL_INTERVAL_MS = 500;
const PROGRESS_POLL_MS = 400;
const SUCCESS_RESET_MS = 2000;

let observer = null;
let refreshScheduled = false;
let lastUrl = location.href;

function getYouTubeTarget(href = location.href) {
	const url = new URL(href);

	if (url.pathname === "/playlist") {
		const playlistId = url.searchParams.get("list");
		if (!playlistId) return null;
		return {
			kind: "playlist",
			url: `https://www.youtube.com/playlist?list=${encodeURIComponent(playlistId)}`,
		};
	}

	if (url.pathname === "/watch") {
		const videoId = url.searchParams.get("v");
		if (!videoId) return null;
		return {
			kind: "track",
			url: `https://www.youtube.com/watch?v=${encodeURIComponent(videoId)}`,
		};
	}

	const shortsMatch = url.pathname.match(/^\/shorts\/([^/?#]+)/);
	if (shortsMatch) {
		return {
			kind: "track",
			url: `https://www.youtube.com/shorts/${encodeURIComponent(shortsMatch[1])}`,
		};
	}

	return null;
}

function findVisibleElement(selectors) {
	for (const selector of selectors) {
		for (const element of document.querySelectorAll(selector)) {
			if (element instanceof HTMLElement && element.offsetParent !== null) {
				return element;
			}
		}
	}

	return null;
}

function findActionContainer(target) {
	if (target.kind === "playlist") {
		return findVisibleElement([
			"yt-page-header-view-model yt-flexible-actions-view-model",
			"ytd-playlist-header-renderer #buttons",
			"ytd-playlist-header-renderer #menu",
		]);
	}

	if (location.pathname.startsWith("/shorts/")) {
		return findVisibleElement([
			"yt-reel-player-overlay-view-model reel-action-bar-view-model",
			"yt-reel-player-overlay-view-model .ytReelPlayerOverlayViewModelActionsContainer",
			"ytd-reel-video-renderer[is-active] #actions",
			"ytd-reel-video-renderer[is-active] #actions-container",
			"ytd-reel-video-renderer #actions",
		]);
	}

	return findVisibleElement([
		"ytd-watch-metadata #actions-inner",
		"ytd-watch-metadata #actions",
	]);
}

function createDownloadIcon() {
	const icon = document.createElementNS("http://www.w3.org/2000/svg", "svg");
	icon.setAttribute("viewBox", "0 0 24 24");
	icon.setAttribute("width", "20");
	icon.setAttribute("height", "20");
	icon.setAttribute("aria-hidden", "true");
	icon.innerHTML =
		'<path fill="currentColor" d="M12 3a1 1 0 0 1 1 1v9.59l2.3-2.3a1 1 0 1 1 1.4 1.42l-4 4a1 1 0 0 1-1.4 0l-4-4a1 1 0 1 1 1.4-1.42l2.3 2.3V4a1 1 0 0 1 1-1Zm-7 14a1 1 0 0 1 1 1v1h12v-1a1 1 0 1 1 2 0v2a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-2a1 1 0 0 1 1-1Z"/>';
	return icon;
}

function formatProgress(progress) {
	if (progress?.status === "queued") {
		return progress.queuePosition ? `待${progress.queuePosition}` : "待機";
	}

	if (progress?.current != null && progress?.total != null) {
		return `${progress.current}/${progress.total}`;
	}

	if (progress?.percent != null) {
		return `${Math.round(progress.percent)}%`;
	}

	if (progress?.status === "importing") return "取込";
	return "…";
}

function setButtonState(button, state, progress = null) {
	button.classList.remove("is-loading", "is-success", "is-error");
	const label = button.querySelector(".catra-yt-download-label");

	if (state === "loading") {
		button.classList.add("is-loading");
		label.textContent = formatProgress(progress);
		button.title = progress?.message ?? "Catraでダウンロード中";
		return;
	}

	if (state === "success") {
		button.classList.add("is-success");
		label.textContent = "完了";
		button.title = "Catraへの追加が完了しました";
		return;
	}

	if (state === "error") {
		button.classList.add("is-error");
		label.textContent = "再試行";
		button.title = "ダウンロードに失敗しました";
		return;
	}

	label.textContent = "Catra";
	button.title =
		button.dataset.kind === "playlist"
			? "プレイリストをCatraでダウンロード"
			: "Catraでダウンロード";
}

function wait(ms) {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

async function fetchDownloadProgress(targetUrl) {
	return chrome.runtime
		.sendMessage({
			type: "GET_DOWNLOAD_PROGRESS",
			trackUrl: targetUrl,
		})
		.catch(() => null);
}

async function pollDownloadProgress(targetUrl, button) {
	const startedAt = Date.now();
	const timeoutMs = 30 * 60 * 1000;

	while (button.isConnected) {
		if (Date.now() - startedAt > timeoutMs) {
			throw new Error("ダウンロードがタイムアウトしました");
		}

		const result = await fetchDownloadProgress(targetUrl);
		const progress = result?.progress;
		if (progress) setButtonState(button, "loading", progress);

		if (progress?.status === "completed") return;
		if (progress?.status === "error") {
			throw new Error(progress.message ?? "ダウンロードに失敗しました");
		}

		await wait(PROGRESS_POLL_MS);
	}
}

async function startDownload(target, button) {
	if (button.classList.contains("is-loading")) return;
	setButtonState(button, "loading");

	try {
		const message =
			target.kind === "playlist"
				? { type: "DOWNLOAD_PLAYLIST", playlistUrl: target.url }
				: { type: "DOWNLOAD_TRACK", trackUrl: target.url };
		const result = await chrome.runtime.sendMessage(message);
		if (!result?.success) {
			throw new Error(result?.error ?? "ダウンロードの開始に失敗しました");
		}

		await pollDownloadProgress(target.url, button);
		setButtonState(button, "success");
		setTimeout(() => {
			if (button.isConnected) setButtonState(button, "idle");
		}, SUCCESS_RESET_MS);
	} catch (error) {
		console.error("Catra YouTube download error:", error);
		if (button.isConnected) setButtonState(button, "error");
	}
}

function createDownloadButton(target) {
	const button = document.createElement("button");
	button.id = YOUTUBE_BUTTON_ID;
	button.type = "button";
	button.className = "catra-yt-download-btn";
	button.dataset.kind = target.kind;
	button.dataset.targetUrl = target.url;
	button.setAttribute("aria-label", "Catraでダウンロード");
	button.appendChild(createDownloadIcon());

	const label = document.createElement("span");
	label.className = "catra-yt-download-label";
	button.appendChild(label);
	setButtonState(button, "idle");

	button.addEventListener("click", (event) => {
		event.preventDefault();
		event.stopPropagation();
		void startDownload(target, button);
	});

	return button;
}

function refreshButton() {
	const target = getYouTubeTarget();
	const existing = document.getElementById(YOUTUBE_BUTTON_ID);

	if (!target) {
		existing?.remove();
		return;
	}

	const container = findActionContainer(target);
	if (!container) return;

	if (
		existing?.dataset.targetUrl === target.url &&
		existing.dataset.kind === target.kind
	) {
		if (!container.contains(existing)) container.appendChild(existing);
		return;
	}

	existing?.remove();
	container.appendChild(createDownloadButton(target));
}

function scheduleRefresh() {
	if (refreshScheduled) return;
	refreshScheduled = true;
	requestAnimationFrame(() => {
		refreshScheduled = false;
		refreshButton();
	});
}

function onNavigation() {
	if (location.href !== lastUrl) {
		lastUrl = location.href;
		document.getElementById(YOUTUBE_BUTTON_ID)?.remove();
	}
	setTimeout(scheduleRefresh, 0);
}

function bootstrap() {
	observer = new MutationObserver(scheduleRefresh);
	observer.observe(document.documentElement, {
		childList: true,
		subtree: true,
	});
	document.addEventListener("yt-navigate-finish", onNavigation);
	window.addEventListener("popstate", onNavigation);
	setInterval(onNavigation, POLL_INTERVAL_MS);
	scheduleRefresh();
}

bootstrap();
