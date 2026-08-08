const INLINE_BUTTON_ID = "catra-sc-inline-download";
const PLAYLIST_BUTTON_ID = "catra-sc-playlist-download";
const LIST_BUTTON_CLASS = "catra-sc-list-download";
const TRACK_ACTION_CONTAINER_CLASS = "mui-16ytee5";
const MORE_MENU_SELECTOR = 'button[aria-label="More menu"]';
const SUCCESS_RESET_MS = 2000;
const POLL_INTERVAL_MS = 500;

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

function isTrackPage() {
  const segments = getPathSegments();
  if (segments.length < 2 || segments.length > 3) {
    return false;
  }

  return !segments.some((segment) => NON_TRACK_SEGMENTS.has(segment.toLowerCase()));
}

const PLAYLIST_ACTION_CONTROL_COUNT = 4;

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
  const normalized = CatraSC.normalizeTrackUrl(getPageHrefForDetection());
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

function isPlaylistHeaderActionContainer(container) {
  if (container.querySelector(`.${TRACK_ACTION_CONTAINER_CLASS}`)) {
    return false;
  }

  const row = container.closest(
    "article, tr, li, [class*='item'], [class*='row'], [class*='track']",
  );

  if (row && extractTrackUrlFromItem(row)) {
    return false;
  }

  return true;
}

function getAncestorDistance(element, ancestor) {
  let distance = 0;
  let node = element;

  while (node && node !== ancestor) {
    distance += 1;
    node = node.parentElement;
  }

  return node === ancestor ? distance : Number.POSITIVE_INFINITY;
}

function findPlaylistHeaderActionContainer() {
  const titles = collectElementsDeep(document.documentElement, "h1");
  const candidates = [];

  for (const div of collectElementsDeep(document.documentElement, "div")) {
    const controls = getPlaylistActionBarControls(div);
    if (controls.length < 3 || controls.length > 6) {
      continue;
    }

    if (!isPlaylistHeaderActionContainer(div)) {
      continue;
    }

    candidates.push({
      container: div,
      templateButton: controls.at(-1),
      controlCount: controls.length,
    });
  }

  if (candidates.length === 0) {
    return null;
  }

  const title = titles[0] ?? null;
  if (!title) {
    return [...candidates].sort(
      (left, right) =>
        Math.abs(left.controlCount - PLAYLIST_ACTION_CONTROL_COUNT) -
        Math.abs(right.controlCount - PLAYLIST_ACTION_CONTROL_COUNT),
    )[0];
  }

  let best = null;
  let bestScore = Number.POSITIVE_INFINITY;

  for (const candidate of candidates) {
    const distance = getAncestorDistance(candidate.container, title);
    const score =
      Math.abs(candidate.controlCount - PLAYLIST_ACTION_CONTROL_COUNT) * 1000 + distance;
    if (score < bestScore) {
      bestScore = score;
      best = candidate;
    }
  }

  return best;
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
  iconWrapper.appendChild(createDownloadIcon());
  button.appendChild(iconWrapper);
  button.addEventListener("click", onClick);
  return button;
}

function setButtonState(button, state) {
  button.classList.remove("is-loading", "is-success", "is-error");

  if (state === "loading") {
    button.classList.add("is-loading");
    button.title = "ダウンロード中...";
    button.setAttribute("aria-label", "ダウンロード中");
    return;
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
  if (button) {
    setButtonState(button, "loading");
  }

  try {
    const result = await chrome.runtime.sendMessage({
      type: "DOWNLOAD_PLAYLIST",
      playlistUrl: getCurrentPageUrl(),
    });

    if (!result?.success) {
      throw new Error(result?.error ?? "一括ダウンロードの開始に失敗しました");
    }

    if (button) {
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

  if (button) {
    setButtonState(button, "loading");
  }

  try {
    const result = await chrome.runtime.sendMessage({
      type: "DOWNLOAD_TRACK",
      trackUrl: CatraSC.normalizeTrackUrl(trackUrl),
    });

    if (!result?.success) {
      throw new Error(result?.error ?? "ダウンロードの開始に失敗しました");
    }

    if (button) {
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
    if (!trackUrl || trackUrl === getCurrentPageUrl()) {
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
  ensurePlaylistDownloadButton();
  ensureListDownloadButtons();
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
      element.matches?.(MORE_MENU_SELECTOR) ||
      element.querySelector?.(`.${TRACK_ACTION_CONTAINER_CLASS}`) ||
      element.querySelector?.(MORE_MENU_SELECTOR)
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
