const INLINE_BUTTON_ID = "catra-sc-inline-download";
const PLAYLIST_BUTTON_ID = "catra-sc-playlist-download";
const LIST_BUTTON_CLASS = "catra-sc-list-download";
const TRACK_ACTION_CONTAINER_CLASS = "mui-16ytee5";
const PLAYABLE_TILE_ACTION_WRAPPER = ".playableTile__actionWrapper";
const MORE_MENU_SELECTOR = 'button[aria-label="More menu"]';
const CLASSIC_MORE_SELECTOR =
  '.sc-button-more, button[title="More"], button[aria-label="More"]';
const SUCCESS_RESET_MS = 2000;
const POLL_INTERVAL_MS = 500;
const PROGRESS_POLL_MS = 400;

const NON_TRACK_SEGMENTS = new Set([
  "discover",
  "search",
  "stream",
  "upload",
  "feed",
  "you",
  "sets",
  "likes",
  "albums",
  "tracks",
  "reposts",
  "comments",
  "popular-tracks",
  "following",
  "followers",
  "library",
  "notifications",
  "messages",
  "settings",
  "signin",
  "signup",
  "pages",
  "stations",
  "charts",
]);

let inlineDownloading = false;
let playlistDownloading = false;
let successResetTimeout = null;
let observer = null;
let pollTimer = null;
let refreshScheduled = false;
let lastUrl = location.href;

function collectElementsDeep(root, selector) {
  if (!root) {
    return [];
  }

  const results = [];
  try {
    results.push(...root.querySelectorAll(selector));
  } catch {
    return results;
  }

  for (const element of root.querySelectorAll("*")) {
    if (element.shadowRoot) {
      results.push(...collectElementsDeep(element.shadowRoot, selector));
    }
  }

  return results;
}

function getPageHrefForDetection() {
  try {
    if (window.top?.location?.hostname?.includes?.("soundcloud.com")) {
      return window.top.location.href;
    }
  } catch {
    // Access to top frame location can be blocked in edge cases.
  }

  return window.location.href;
}

function getPathSegmentsFromHref(href) {
  const segments = new URL(href).pathname.split("/").filter(Boolean);
  if (segments[0]?.toLowerCase() === "n") {
    return segments.slice(1);
  }

  return segments;
}

function getPathSegments() {
  return getPathSegmentsFromHref(getPageHrefForDetection());
}

function getFramePathSegments() {
  return getPathSegmentsFromHref(window.location.href);
}

function isTrackPage() {
  const segments = getFramePathSegments();
  if (segments.length < 2 || segments.length > 3) {
    return false;
  }

  return !segments.some((segment) => NON_TRACK_SEGMENTS.has(segment.toLowerCase()));
}

const PLAYLIST_HEADER_ACTION_XPATH =
  "/html/body/div[1]/div[2]/div[2]/div/div[2]/div[1]/div/div[1]/div/div/div[2]/div/div[1]";

function evaluateXPath(xpath) {
  try {
    const result = document.evaluate(
      xpath,
      document,
      null,
      XPathResult.FIRST_ORDERED_NODE_TYPE,
      null,
    );
    const node = result.singleNodeValue;
    return node instanceof Element ? node : null;
  } catch {
    return null;
  }
}

function findPlaylistHeaderActionContainer() {
  const container = evaluateXPath(PLAYLIST_HEADER_ACTION_XPATH);
  if (!container) {
    return null;
  }

  const controls = getPlaylistActionBarControls(container);
  if (controls.length < 3) {
    return null;
  }

  return {
    container,
    templateButton: controls.at(-1),
  };
}

function isPlaylistPage() {
  for (const href of [getPageHrefForDetection(), window.location.href]) {
    const segments = getPathSegmentsFromHref(href);
    if (segments.length === 3 && segments[1].toLowerCase() === "sets") {
      return true;
    }
  }

  return false;
}

function getCurrentPageUrl() {
  const href = isPlaylistPage() ? getPageHrefForDetection() : window.location.href;
  const normalized = CatraSC.normalizeTrackUrl(href);
  const parsed = new URL(normalized);
  const segments = parsed.pathname.split("/").filter(Boolean);

  if (segments[0]?.toLowerCase() === "n") {
    parsed.pathname = `/${segments.slice(1).join("/")}`;
    return parsed.toString().replace(/\/$/, "");
  }

  return normalized;
}

function hasEmbeddedTrackIframe() {
  for (const iframe of document.querySelectorAll("iframe[src]")) {
    const src = iframe.getAttribute("src") ?? "";
    if (/soundcloud\.com\/n\/[^/]+\/[^/?#]+/i.test(src)) {
      return true;
    }
  }

  return false;
}

function isPlaylistActionBarControl(element) {
  if (!(element instanceof HTMLElement)) {
    return false;
  }

  if (!element.querySelector("svg")) {
    return false;
  }

  return element.tagName === "BUTTON" || element.getAttribute("role") === "button";
}

function getPlaylistActionBarControls(container) {
  const directControls = [...container.children].filter(isPlaylistActionBarControl);
  if (directControls.length >= 3) {
    return directControls;
  }

  const wrappedControls = [];

  for (const child of container.children) {
    if (!(child instanceof HTMLElement)) {
      continue;
    }

    const control = child.matches("button, [role='button']")
      ? child
      : child.querySelector(":scope > button, :scope > [role='button']");

    if (isPlaylistActionBarControl(control)) {
      wrappedControls.push(control);
    }
  }

  return wrappedControls;
}

function findPrimaryMuiActionContainer() {
  let bestContainer = null;
  let bestVisibleButtons = 0;

  for (const container of collectElementsDeep(
    document.documentElement,
    `.${TRACK_ACTION_CONTAINER_CLASS}`,
  )) {
    const visibleButtons = [...container.querySelectorAll("button.MuiIconButton-root")].filter(
      (button) => button.offsetParent !== null,
    ).length;

    if (visibleButtons > bestVisibleButtons) {
      bestVisibleButtons = visibleButtons;
      bestContainer = container;
    }
  }

  return bestContainer;
}

function isUiShellFrame() {
  return (
    window.top === window &&
    !findPrimaryMuiActionContainer() &&
    hasEmbeddedTrackIframe()
  );
}

function shouldHandleInlineButton() {
  if (!isTrackPage() || isPlaylistPage()) {
    return false;
  }

  if (isUiShellFrame()) {
    return false;
  }

  return true;
}

function findMuiActionContainerWithin(root) {
  const inRoot = root.querySelector?.(`.${TRACK_ACTION_CONTAINER_CLASS}`);
  if (inRoot) {
    return inRoot;
  }

  if (root instanceof Element && root !== document.documentElement) {
    let node = root.parentElement;
    while (node && node !== document.documentElement) {
      if (node.classList?.contains(TRACK_ACTION_CONTAINER_CLASS)) {
        return node;
      }

      const nested = node.querySelector?.(`.${TRACK_ACTION_CONTAINER_CLASS}`);
      if (nested) {
        return nested;
      }

      node = node.parentElement;
    }
  }

  return null;
}

function buildActionTarget(container) {
  const moreButton = container.querySelector(MORE_MENU_SELECTOR);
  const templateButton = container.querySelector("button.MuiIconButton-root");

  return {
    container,
    insertBefore: moreButton,
    templateButton: templateButton ?? moreButton,
  };
}

function findInlineActionContainer() {
  const container = findPrimaryMuiActionContainer();
  if (!container) {
    return null;
  }

  return buildActionTarget(container);
}

function getMuiIconButtonClassName(templateButton) {
  const classes = [
    "MuiButtonBase-root",
    "MuiIconButton-root",
    "MuiIconButton-colorContrast",
    "MuiIconButton-sizeMedium",
    "catra-sc-download-btn",
  ];

  if (templateButton) {
    for (const className of templateButton.classList) {
      if (className.startsWith("mui-")) {
        classes.push(className);
        break;
      }
    }
  }

  return classes.join(" ");
}

function createDownloadIcon() {
  const icon = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  icon.setAttribute("viewBox", "0 0 24 24");
  icon.setAttribute("width", "24");
  icon.setAttribute("height", "24");
  icon.setAttribute("xmlns", "http://www.w3.org/2000/svg");
  icon.setAttribute("aria-hidden", "true");
  icon.innerHTML =
    '<path fill="currentColor" d="M12 3a1 1 0 0 1 1 1v9.59l2.3-2.3a1 1 0 1 1 1.4 1.42l-4 4a1 1 0 0 1-1.4 0l-4-4a1 1 0 1 1 1.4-1.42l2.3 2.3V4a1 1 0 0 1 1-1Zm-7 14a1 1 0 0 1 1 1v1h12v-1a1 1 0 1 1 2 0v2a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-2a1 1 0 0 1 1-1Z"/>';
  return icon;
}

function createDownloadButton({ id, className, onClick }) {
  const button = document.createElement("button");
  button.className = className;
  button.type = "button";
  button.tabIndex = 0;
  button.setAttribute("variant", "outlined");
  button.setAttribute("aria-label", "ダウンロード");

  if (id) {
    button.id = id;
  }

  const iconWrapper = document.createElement("div");
  iconWrapper.className = "catra-sc-download-icon";
  iconWrapper.appendChild(createDownloadIcon());
  button.appendChild(iconWrapper);

  const progressLabel = document.createElement("span");
  progressLabel.className = "catra-sc-progress-label";
  progressLabel.hidden = true;
  button.appendChild(progressLabel);

  button.addEventListener("click", onClick);
  return button;
}

function formatProgressLabel(progress) {
  if (!progress) {
    return "";
  }

  if (progress.status === "queued") {
    return progress.queuePosition ? `待${progress.queuePosition}` : "待";
  }

  if (progress.status === "importing") {
    if (progress.current != null && progress.total != null) {
      return `${progress.current}/${progress.total}`;
    }
    return "...";
  }

  if (progress.status === "downloading") {
    if (progress.percent != null) {
      const rounded = Math.round(progress.percent);
      if (progress.current != null && progress.total != null) {
        return `${progress.current}/${progress.total}`;
      }
      return `${rounded}%`;
    }
    return "…";
  }

  if (progress.percent != null) {
    const rounded = Math.round(progress.percent);
    if (progress.current != null && progress.total != null) {
      return `${progress.current}/${progress.total}`;
    }
    return `${rounded}%`;
  }

  if (progress.current != null && progress.total != null) {
    return `${progress.current}/${progress.total}`;
  }

  return "";
}

function updateButtonProgress(button, progress) {
  const label = button.querySelector(".catra-sc-progress-label");
  if (!label) {
    return;
  }

  const text = formatProgressLabel(progress);
  label.textContent = text;
  label.hidden = !text;

  if (progress?.message) {
    button.title = progress.message;
    button.setAttribute("aria-label", progress.message);
  }
}

function wait(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function fetchDownloadProgress(trackUrl) {
  const normalizedUrl = CatraSC.normalizeTrackUrl(trackUrl);

  const specific = await chrome.runtime
    .sendMessage({
      type: "GET_DOWNLOAD_PROGRESS",
      trackUrl: normalizedUrl,
    })
    .catch(() => null);

  if (specific?.progress && specific.progress.status !== "idle") {
    return specific.progress;
  }

  const active = await chrome.runtime
    .sendMessage({
      type: "GET_DOWNLOAD_PROGRESS",
      active: true,
    })
    .catch(() => null);

  return active?.progress ?? specific?.progress ?? null;
}

async function pollDownloadProgress(trackUrl, button) {
  const startedAt = Date.now();
  const timeoutMs = 30 * 60 * 1000;

  while (button?.isConnected) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error("ダウンロードがタイムアウトしました");
    }

    const progress = await fetchDownloadProgress(trackUrl);
    if (progress && button.classList.contains("is-loading")) {
      updateButtonProgress(button, progress);
    }

    if (progress?.status === "completed") {
      return;
    }

    if (progress?.status === "error") {
      throw new Error(progress.message ?? "ダウンロードに失敗しました");
    }

    await wait(PROGRESS_POLL_MS);
  }
}

function setButtonState(button, state, progress = null) {
  button.classList.remove("is-loading", "is-success", "is-error");

  if (state === "loading") {
    button.classList.add("is-loading");
    button.title = progress?.message ?? "ダウンロード中...";
    button.setAttribute("aria-label", progress?.message ?? "ダウンロード中");
    updateButtonProgress(button, progress);
    return;
  }

  const progressLabel = button.querySelector(".catra-sc-progress-label");
  if (progressLabel) {
    progressLabel.hidden = true;
    progressLabel.textContent = "";
  }

  if (state === "success") {
    button.classList.add("is-success");
    button.title = "Catra でダウンロード開始";
    button.setAttribute("aria-label", "Catra でダウンロード開始");
    return;
  }

  if (state === "error") {
    button.classList.add("is-error");
    button.title = "エラー。再試行してください";
    button.setAttribute("aria-label", "ダウンロードエラー");
    return;
  }

  button.title = "ダウンロード";
  button.setAttribute("aria-label", "ダウンロード");
}

function resetButtonAfterSuccess(button) {
  if (successResetTimeout) {
    clearTimeout(successResetTimeout);
  }

  successResetTimeout = setTimeout(() => {
    successResetTimeout = null;
    inlineDownloading = false;
    if (button.id === PLAYLIST_BUTTON_ID) {
      button.classList.remove("is-loading", "is-success", "is-error");
      updatePlaylistButtonPresentation(button);
      return;
    }

    setButtonState(button, "idle");
  }, SUCCESS_RESET_MS);
}

async function downloadPlaylist(button) {
  if (playlistDownloading || button?.classList.contains("is-loading")) {
    return;
  }

  playlistDownloading = true;
  const trackUrl = getCurrentPageUrl();
  if (button) {
    setButtonState(button, "loading");
  }

  try {
    const result = await chrome.runtime.sendMessage({
      type: "DOWNLOAD_PLAYLIST",
      playlistUrl: trackUrl,
    });

    if (!result?.success) {
      throw new Error(result?.error ?? "一括ダウンロードの開始に失敗しました");
    }

    if (button) {
      await pollDownloadProgress(trackUrl, button);
      setButtonState(button, "success");
      resetButtonAfterSuccess(button);
    }
  } catch (error) {
    console.error("Catra SoundCloud playlist download error:", error);
    if (button) {
      setButtonState(button, "error");
    }
  } finally {
    playlistDownloading = false;
  }
}

async function downloadTrack(trackUrl, button) {
  if (button?.classList.contains("is-loading")) {
    return;
  }

  const normalizedUrl = CatraSC.normalizeTrackUrl(trackUrl);
  if (button) {
    setButtonState(button, "loading");
  }

  try {
    const result = await chrome.runtime.sendMessage({
      type: "DOWNLOAD_TRACK",
      trackUrl: normalizedUrl,
    });

    if (!result?.success) {
      throw new Error(result?.error ?? "ダウンロードの開始に失敗しました");
    }

    if (button) {
      await pollDownloadProgress(normalizedUrl, button);
      setButtonState(button, "success");
      resetButtonAfterSuccess(button);
    }
  } catch (error) {
    console.error("Catra SoundCloud download error:", error);
    if (button) {
      inlineDownloading = false;
      setButtonState(button, "error");
    }
  }
}

function getInlineButton() {
  const existing = document.getElementById(INLINE_BUTTON_ID);
  if (existing?.isConnected) {
    return existing;
  }

  if (existing) {
    existing.remove();
  }

  return null;
}

function insertDownloadButton(actionTarget, button) {
  if (actionTarget.insertBefore?.isConnected) {
    actionTarget.container.insertBefore(button, actionTarget.insertBefore);
    return;
  }

  actionTarget.container.appendChild(button);
}

function createInlineDownloadButton(templateButton) {
  const button = createDownloadButton({
    id: INLINE_BUTTON_ID,
    className: getMuiIconButtonClassName(templateButton),
    onClick: async (event) => {
      event.preventDefault();
      event.stopPropagation();

      if (inlineDownloading) {
        return;
      }

      inlineDownloading = true;
      await downloadTrack(getCurrentPageUrl(), button);
    },
  });

  return button;
}

function ensureInlineDownloadButton() {
  if (!shouldHandleInlineButton()) {
    if (!isUiShellFrame()) {
      removeInlineDownloadButton();
    }

    return false;
  }

  const actionTarget = findInlineActionContainer();
  if (!actionTarget) {
    return false;
  }

  let button = getInlineButton();
  if (button && !actionTarget.container.contains(button)) {
    insertDownloadButton(actionTarget, button);
  }

  if (!button) {
    button = createInlineDownloadButton(actionTarget.templateButton);
    insertDownloadButton(actionTarget, button);
  }

  if (!inlineDownloading) {
    setButtonState(button, "idle");
  }

  return true;
}

function removeInlineDownloadButton() {
  inlineDownloading = false;
  document.getElementById(INLINE_BUTTON_ID)?.remove();
}

function extractTrackUrlFromItem(item) {
  const permalinkLink = item.querySelector('a[href*="/"][href]:not([href^="http"])');
  if (!permalinkLink) {
    return null;
  }

  const href = permalinkLink.getAttribute("href");
  if (!href || href.startsWith("#")) {
    return null;
  }

  const url = getCurrentPageUrlFromHref(href);
  if (!url) {
    return null;
  }

  const segments = getPathSegmentsFromUrl(url);
  if (segments.length < 2 || segments.length > 3) {
    return null;
  }

  if (segments.some((segment) => NON_TRACK_SEGMENTS.has(segment.toLowerCase()))) {
    return null;
  }

  if (segments[1].toLowerCase() === "sets") {
    return null;
  }

  return url;
}

function getPathSegmentsFromUrl(url) {
  const segments = new URL(url).pathname.split("/").filter(Boolean);
  if (segments[0]?.toLowerCase() === "n") {
    return segments.slice(1);
  }

  return segments;
}

function getCurrentPageUrlFromHref(href) {
  const normalized = CatraSC.normalizeTrackUrl(href);
  const parsed = new URL(normalized, window.location.origin);
  const segments = parsed.pathname.split("/").filter(Boolean);

  if (segments[0]?.toLowerCase() === "n") {
    parsed.pathname = `/${segments.slice(1).join("/")}`;
    return parsed.toString().replace(/\/$/, "");
  }

  return normalized;
}

function findTrackItems() {
  const items = new Set();

  for (const container of collectElementsDeep(
    document.documentElement,
    `.${TRACK_ACTION_CONTAINER_CLASS}`,
  )) {
    const row = container.closest(
      "article, tr, li, [class*='item'], [class*='row'], [class*='track']",
    );
    if (row && row !== container) {
      items.add(row);
    }
  }

  return [...items];
}

function findListActionContainer(item) {
  const container = findMuiActionContainerWithin(item);
  if (!container) {
    return null;
  }

  return buildActionTarget(container);
}

function createListDownloadButton(trackUrl, templateButton) {
  const button = createDownloadButton({
    className: `${getMuiIconButtonClassName(templateButton)} ${LIST_BUTTON_CLASS}`,
    onClick: async (event) => {
      event.preventDefault();
      event.stopPropagation();
      await downloadTrack(trackUrl, button);
    },
  });
  button.dataset.trackUrl = trackUrl;
  return button;
}

function getClassicButtonClassName(templateButton) {
  const classes = new Set([LIST_BUTTON_CLASS, "catra-sc-download-btn", "catra-sc-classic-download"]);

  if (templateButton?.classList) {
    for (const className of templateButton.classList) {
      if (
        className === "sc-button-more" ||
        className === "sc-button-download" ||
        className === "sc-button-selected" ||
        className === LIST_BUTTON_CLASS ||
        className === "catra-sc-download-btn" ||
        className === "catra-sc-classic-download"
      ) {
        continue;
      }
      classes.add(className);
    }
  } else {
    for (const className of [
      "sc-button-small",
      "sc-button-icon",
      "sc-button-responsive",
      "sc-button",
    ]) {
      classes.add(className);
    }
  }

  return [...classes].join(" ");
}

function createClassicDownloadIcon(templateButton) {
  const templateSvg = templateButton?.querySelector?.("svg");
  const width =
    templateSvg?.getAttribute("width") ||
    templateSvg?.getAttribute("viewBox")?.split(/\s+/)[2] ||
    "16";
  const height =
    templateSvg?.getAttribute("height") ||
    templateSvg?.getAttribute("viewBox")?.split(/\s+/)[3] ||
    "16";

  const icon = createDownloadIcon();
  icon.setAttribute("width", String(width));
  icon.setAttribute("height", String(height));
  icon.setAttribute("viewBox", "0 0 24 24");
  icon.style.width = `${width}px`;
  icon.style.height = `${height}px`;

  if (templateSvg) {
    for (const attr of ["class", "fill", "stroke", "aria-hidden", "focusable", "role"]) {
      const value = templateSvg.getAttribute(attr);
      if (value != null) {
        icon.setAttribute(attr, value);
      }
    }
  } else {
    icon.setAttribute("aria-hidden", "true");
    icon.setAttribute("focusable", "false");
  }

  return icon;
}

function createClassicListDownloadButton(trackUrl, templateButton) {
  const button = document.createElement("button");
  button.type = "button";
  button.tabIndex = 0;
  button.className = getClassicButtonClassName(templateButton);
  button.title = "Download";
  button.setAttribute("aria-label", "ダウンロード");
  button.dataset.trackUrl = trackUrl;

  const iconWrapper = document.createElement("div");
  iconWrapper.className = "catra-sc-download-icon";
  iconWrapper.appendChild(createClassicDownloadIcon(templateButton));
  button.appendChild(iconWrapper);

  const progressLabel = document.createElement("span");
  progressLabel.className = "catra-sc-progress-label";
  progressLabel.hidden = true;
  button.appendChild(progressLabel);

  button.addEventListener("click", async (event) => {
    event.preventDefault();
    event.stopPropagation();
    await downloadTrack(trackUrl, button);
  });

  return button;
}

function findPlayableTileMoreButton(wrapper) {
  return (
    wrapper.querySelector(CLASSIC_MORE_SELECTOR) ??
    [...wrapper.querySelectorAll("button")].find((button) => {
      const label = `${button.getAttribute("aria-label") ?? ""} ${button.title ?? ""} ${button.textContent ?? ""}`;
      return /\bmore\b/i.test(label);
    }) ??
    null
  );
}

function extractTrackUrlFromPlayableTile(wrapper) {
  const tile =
    wrapper.closest(".playableTile, .soundBadge, .soundList__item, li, article") ??
    wrapper.parentElement;
  if (!tile) {
    return null;
  }

  const preferredLinks = tile.querySelectorAll(
    "a.playableTile__mainHeading, a.playableTile__heading, a.playableTile__artworkLink, a.soundTitle__title, a[href]",
  );

  for (const link of preferredLinks) {
    const href = link.getAttribute("href");
    if (!href || href.startsWith("#") || href.startsWith("javascript:")) {
      continue;
    }

    const url = getCurrentPageUrlFromHref(href);
    if (!url) {
      continue;
    }

    const segments = getPathSegmentsFromUrl(url);
    if (segments.length < 2 || segments.length > 3) {
      continue;
    }

    if (segments.some((segment) => NON_TRACK_SEGMENTS.has(segment.toLowerCase()))) {
      continue;
    }

    if (segments[1].toLowerCase() === "sets") {
      continue;
    }

    return url;
  }

  return extractTrackUrlFromItem(tile);
}

function findPlayableTileActionTarget(wrapper) {
  const moreButton = findPlayableTileMoreButton(wrapper);
  return {
    container: wrapper,
    insertBefore: moreButton,
    templateButton: moreButton ?? wrapper.querySelector("button"),
  };
}

function ensurePlayableTileDownloadButtons() {
  for (const wrapper of collectElementsDeep(
    document.documentElement,
    PLAYABLE_TILE_ACTION_WRAPPER,
  )) {
    const existing = wrapper.querySelector(`.${LIST_BUTTON_CLASS}`);
    if (existing?.querySelector("svg")) {
      continue;
    }
    existing?.remove();

    const trackUrl = extractTrackUrlFromPlayableTile(wrapper);
    if (!trackUrl) {
      continue;
    }

    if (trackUrl === getCurrentPageUrl() && shouldHandleInlineButton()) {
      continue;
    }

    const actionTarget = findPlayableTileActionTarget(wrapper);
    const button = createClassicListDownloadButton(
      trackUrl,
      actionTarget.templateButton,
    );
    insertDownloadButton(actionTarget, button);
  }
}

function getPlaylistButtonClassName(templateButton) {
  const classes = ["catra-sc-download-btn", "catra-sc-playlist-download-btn"];

  if (templateButton) {
    classes.unshift(templateButton.className);
  }

  return classes.join(" ");
}

function createPlaylistDownloadButton({ templateButton }) {
  const button = createDownloadButton({
    id: PLAYLIST_BUTTON_ID,
    className: getPlaylistButtonClassName(templateButton),
    onClick: async (event) => {
      event.preventDefault();
      event.stopPropagation();
      await downloadPlaylist(button);
    },
  });
  button.title = "プレイリストを一括ダウンロード";
  button.setAttribute("aria-label", "一括ダウンロード");
  return button;
}

function getPlaylistButton() {
  const existing = document.getElementById(PLAYLIST_BUTTON_ID);
  if (existing?.isConnected) {
    return existing;
  }

  if (existing) {
    existing.remove();
  }

  return null;
}

function removePlaylistDownloadButton() {
  playlistDownloading = false;
  document.getElementById(PLAYLIST_BUTTON_ID)?.remove();
}

function updatePlaylistButtonPresentation(button) {
  button.title = "プレイリストを一括ダウンロード";
  button.setAttribute("aria-label", "一括ダウンロード");
}

function ensurePlaylistDownloadButton() {
  if (!isPlaylistPage()) {
    removePlaylistDownloadButton();
    return false;
  }

  const actionTarget = findPlaylistHeaderActionContainer();
  if (!actionTarget) {
    removePlaylistDownloadButton();
    return false;
  }

  let button = getPlaylistButton();
  if (!button) {
    button = createPlaylistDownloadButton({
      templateButton: actionTarget.templateButton,
    });
    actionTarget.container.appendChild(button);
  } else if (!actionTarget.container.contains(button)) {
    actionTarget.container.appendChild(button);
  }

  if (!playlistDownloading) {
    button.classList.remove("is-loading", "is-success", "is-error");
    updatePlaylistButtonPresentation(button);
  }

  return true;
}

function ensureListDownloadButtons() {
  for (const item of findTrackItems()) {
    const trackUrl = extractTrackUrlFromItem(item);
    if (!trackUrl) {
      continue;
    }

    if (trackUrl === getCurrentPageUrl()) {
      continue;
    }

    const actionTarget = findListActionContainer(item);
    const existingButton = item.querySelector(`.${LIST_BUTTON_CLASS}`);

    if (existingButton?.isConnected) {
      continue;
    }

    if (existingButton && !existingButton.isConnected) {
      existingButton.remove();
    }

    const button = createListDownloadButton(
      trackUrl,
      actionTarget?.templateButton ?? null,
    );

    if (actionTarget) {
      insertDownloadButton(actionTarget, button);
      continue;
    }

    item.style.position = "relative";
    button.classList.add("catra-sc-list-download--floating");
    item.appendChild(button);
  }
}

function refreshButtons() {
  ensureInlineDownloadButton();
  ensureListDownloadButtons();
  ensurePlayableTileDownloadButtons();
  ensurePlaylistDownloadButton();
}

function scheduleRefresh() {
  if (refreshScheduled) {
    return;
  }

  refreshScheduled = true;
  requestAnimationFrame(() => {
    refreshScheduled = false;
    refreshButtons();
  });
}

function startPolling() {
  if (pollTimer) {
    clearInterval(pollTimer);
  }

  pollTimer = setInterval(() => {
    refreshButtons();
  }, POLL_INTERVAL_MS);
}

function onNavigation() {
  const currentUrl = location.href;
  if (currentUrl === lastUrl) {
    scheduleRefresh();
    return;
  }

  lastUrl = currentUrl;
  inlineDownloading = false;
  playlistDownloading = false;
  removeInlineDownloadButton();
  removePlaylistDownloadButton();
  scheduleRefresh();
  startPolling();
}

function shouldScheduleRefreshFromMutation(mutation) {
  if (mutation.type !== "childList") {
    return false;
  }

  for (const node of mutation.addedNodes) {
    if (node.nodeType !== Node.ELEMENT_NODE) {
      continue;
    }

    const element = node;
    if (
      element.id === INLINE_BUTTON_ID ||
      element.id === PLAYLIST_BUTTON_ID ||
      element.classList?.contains(TRACK_ACTION_CONTAINER_CLASS) ||
      element.classList?.contains("playableTile__actionWrapper") ||
      element.matches?.(MORE_MENU_SELECTOR) ||
      element.matches?.(CLASSIC_MORE_SELECTOR) ||
      element.querySelector?.(`.${TRACK_ACTION_CONTAINER_CLASS}`) ||
      element.querySelector?.(PLAYABLE_TILE_ACTION_WRAPPER) ||
      element.querySelector?.(MORE_MENU_SELECTOR) ||
      element.querySelector?.(CLASSIC_MORE_SELECTOR)
    ) {
      return true;
    }
  }

  for (const node of mutation.removedNodes) {
    if (node.nodeType !== Node.ELEMENT_NODE) {
      continue;
    }

    const element = node;
    if (
      element.id === INLINE_BUTTON_ID ||
      element.id === PLAYLIST_BUTTON_ID ||
      element.querySelector?.(`#${INLINE_BUTTON_ID}`) ||
      element.querySelector?.(`#${PLAYLIST_BUTTON_ID}`)
    ) {
      return true;
    }
  }

  return false;
}

function startObserver() {
  if (observer) {
    return;
  }

  observer = new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      if (shouldScheduleRefreshFromMutation(mutation)) {
        scheduleRefresh();
        return;
      }
    }
  });

  observer.observe(document.documentElement, {
    childList: true,
    subtree: true,
  });
}

function hookHistory() {
  const notify = () => {
    if (location.href === lastUrl) {
      scheduleRefresh();
      return;
    }

    onNavigation();
  };

  const { pushState, replaceState } = history;
  history.pushState = function pushStatePatched(...args) {
    const result = pushState.apply(this, args);
    notify();
    return result;
  };
  history.replaceState = function replaceStatePatched(...args) {
    const result = replaceState.apply(this, args);
    notify();
    return result;
  };

  window.addEventListener("popstate", notify);
}

function watchNavigation() {
  setInterval(() => {
    if (location.href !== lastUrl) {
      onNavigation();
    }
  }, POLL_INTERVAL_MS);
}

function bootstrap() {
  onNavigation();
  startObserver();
  hookHistory();
  watchNavigation();
  startPolling();
}

bootstrap();
