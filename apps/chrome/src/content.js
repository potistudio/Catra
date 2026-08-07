const INLINE_BUTTON_ID = "catra-sc-inline-download";
const LIST_BUTTON_CLASS = "catra-sc-list-download";
const ACTION_CONTAINER_CLASS = "mui-16ytee5";
const MORE_MENU_SELECTORS = [
  'button[aria-label="More menu"]',
  'button[aria-label="More"]',
  'button[title="More"]',
  ".sc-button-more",
];
const SUCCESS_RESET_MS = 2000;
const POLL_INTERVAL_MS = 500;
const POLL_DURATION_MS = 60000;

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
let successResetTimeout = null;

function getPathSegments() {
  const segments = window.location.pathname.split("/").filter(Boolean);
  if (segments[0]?.toLowerCase() === "n") {
    return segments.slice(1);
  }

  return segments;
}

function isTrackPage() {
  const segments = getPathSegments();
  if (segments.length < 2 || segments.length > 3) {
    return false;
  }

  return !segments.some((segment) => NON_TRACK_SEGMENTS.has(segment.toLowerCase()));
}

function isPlaylistPage() {
  const segments = getPathSegments();
  return segments.length === 3 && segments[1].toLowerCase() === "sets";
}

function getCurrentPageUrl() {
  return CatraSC.normalizeTrackUrl(window.location.href);
}

function queryWithinRoot(root, selector) {
  const match =
    root.querySelector?.(selector) ??
    (root.matches?.(selector) ? root : null);
  return match ?? null;
}

function findMuiActionContainer(root) {
  const inRoot = queryWithinRoot(root, `.${ACTION_CONTAINER_CLASS}`);
  if (inRoot) {
    return inRoot;
  }

  if (root instanceof Element && root !== document.documentElement) {
    let node = root.parentElement;
    while (node && node !== document.documentElement) {
      if (node.classList?.contains(ACTION_CONTAINER_CLASS)) {
        return node;
      }

      const inAncestor = node.querySelector?.(`.${ACTION_CONTAINER_CLASS}`);
      if (inAncestor) {
        return inAncestor;
      }

      node = node.parentElement;
    }
  }

  return null;
}

function buildActionTarget(container, preferredMoreButton = null) {
  const moreButton = preferredMoreButton ?? findMoreMenuButton(container);
  return {
    container,
    insertBefore: moreButton,
    templateButton:
      moreButton ?? container.querySelector("button.MuiIconButton-root"),
  };
}

function findMoreMenuButton(root) {
  for (const selector of MORE_MENU_SELECTORS) {
    const button =
      root.querySelector?.(selector) ??
      (root.matches?.(selector) ? root : null);
    if (button) {
      return button;
    }
  }

  return null;
}

function getTraversalBoundary(root) {
  if (
    root === document ||
    root === document.documentElement ||
    root === document.body
  ) {
    return document.documentElement;
  }

  return root;
}

function findActionButtonGroup(moreButton, boundary) {
  let node = moreButton.parentElement;
  while (node && node !== boundary) {
    const iconButtons = node.querySelectorAll("button.MuiIconButton-root");
    if (iconButtons.length >= 2) {
      return node;
    }

    node = node.parentElement;
  }

  return moreButton.parentElement;
}

function resolveActionContainer(root) {
  const muiContainer = findMuiActionContainer(root);
  if (muiContainer) {
    return buildActionTarget(muiContainer);
  }

  const moreButton = findMoreMenuButton(root);
  if (moreButton) {
    const container = findActionButtonGroup(
      moreButton,
      getTraversalBoundary(root),
    );

    if (container) {
      return buildActionTarget(container, moreButton);
    }
  }

  return null;
}

function findInlineActionContainer() {
  for (const container of document.querySelectorAll(`.${ACTION_CONTAINER_CLASS}`)) {
    if (
      container.querySelector("button.MuiIconButton-root") &&
      findMoreMenuButton(container)
    ) {
      return buildActionTarget(container);
    }
  }

  const engagementRoot =
    document.querySelector(".listenEngagement__actions") ||
    document.querySelector(".soundActions");

  if (engagementRoot) {
    const scopedTarget = resolveActionContainer(engagementRoot);
    if (scopedTarget) {
      return scopedTarget;
    }

    const legacyGroup = engagementRoot.querySelector(".sc-button-group");
    if (legacyGroup) {
      return {
        container: legacyGroup,
        insertBefore: legacyGroup.querySelector(".sc-button-more"),
        templateButton: legacyGroup.querySelector("button"),
      };
    }
  }

  return resolveActionContainer(document);
}

function findActionButtonContainer() {
  const actionTarget = findInlineActionContainer();
  if (actionTarget) {
    return actionTarget;
  }

  const legacyGroup =
    document.querySelector(".listenEngagement__actions .sc-button-group") ||
    document.querySelector(".soundActions .sc-button-group");

  if (legacyGroup) {
    return {
      container: legacyGroup,
      insertBefore: legacyGroup.querySelector(".sc-button-more"),
      templateButton: legacyGroup.querySelector("button"),
    };
  }

  return null;
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

function getDownloadButtonClassName(templateButton) {
  if (templateButton?.classList.contains("MuiIconButton-root")) {
    return getMuiIconButtonClassName(templateButton);
  }

  if (templateButton?.classList.contains("sc-button")) {
  const sizeClass = templateButton.classList.contains("sc-button-small")
    ? "sc-button-small"
    : "sc-button-medium";
  return [
    "sc-button-secondary",
    "sc-button",
    sizeClass,
    "sc-button-icon",
    "sc-button-responsive",
    "catra-sc-download-btn",
  ].join(" ");
  }

  return getMuiIconButtonClassName(templateButton);
}

function createMuiDownloadButton({ id, className, onClick }) {
  const button = document.createElement("button");
  button.className = className;
  button.type = "button";
  button.tabIndex = 0;
  button.setAttribute("variant", "outlined");
  button.setAttribute("aria-label", "ダウンロード");
  if (id) {
    button.id = id;
  }
  button.appendChild(createDownloadIcon());
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
    setButtonState(button, "idle");
  }, SUCCESS_RESET_MS);
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

function getInlineButton(container) {
  const buttonInContainer = container.querySelector(`#${INLINE_BUTTON_ID}`);
  if (buttonInContainer?.isConnected) {
    return buttonInContainer;
  }

  const detached = document.getElementById(INLINE_BUTTON_ID);
  if (detached && !detached.isConnected) {
    detached.remove();
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
  const button = createMuiDownloadButton({
    id: INLINE_BUTTON_ID,
    className: getDownloadButtonClassName(templateButton),
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
  if (!isTrackPage() || isPlaylistPage()) {
    removeInlineDownloadButton();
    return false;
  }

  const actionTarget = findActionButtonContainer();
  if (!actionTarget) {
    return false;
  }

  let button = getInlineButton(actionTarget.container);
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

  const url = CatraSC.normalizeTrackUrl(href);
  const path = new URL(url).pathname;
  const segments = path.split("/").filter(Boolean);

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

function findTrackItems() {
  const selectors = [
    "article.sound",
    ".sound__content",
    ".trackItem__content",
    ".soundList__item",
    ".searchList__item",
    ".lazyLoadingList__item",
    `[class*="${ACTION_CONTAINER_CLASS}"]`,
  ];

  const items = new Set();
  for (const selector of selectors) {
    for (const element of document.querySelectorAll(selector)) {
      if (element.classList?.contains(ACTION_CONTAINER_CLASS)) {
        const row = element.closest(
          "article, tr, li, [class*='item'], [class*='row'], [class*='track']",
        );
        items.add(row ?? element.parentElement ?? element);
        continue;
      }

      items.add(element);
    }
  }

  return [...items];
}

function findListActionContainer(item) {
  const actionTarget = resolveActionContainer(item);
  if (actionTarget) {
    return actionTarget;
  }

  const legacyActions =
    item.querySelector(".sound__actions") ||
    item.querySelector(".trackItem__actions") ||
    item.querySelector(".soundActions") ||
    item.querySelector(".sc-button-group");

  if (legacyActions) {
    return {
      container: legacyActions,
      insertBefore: legacyActions.querySelector(".sc-button-more"),
      templateButton: legacyActions.querySelector("button"),
    };
  }

  return null;
}

function createListDownloadButton(trackUrl, templateButton) {
  const button = createMuiDownloadButton({
    className: `${getDownloadButtonClassName(templateButton)} ${LIST_BUTTON_CLASS}`,
    onClick: async (event) => {
      event.preventDefault();
      event.stopPropagation();
      await downloadTrack(trackUrl, button);
    },
  });
  button.dataset.trackUrl = trackUrl;
  return button;
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
  ensureListDownloadButtons();
}

let observer = null;
let pollTimer = null;
let pollStartedAt = 0;
let refreshScheduled = false;
let lastUrl = location.href;

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

function stopPolling() {
  if (pollTimer) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
}

function startPolling() {
  stopPolling();
  pollStartedAt = Date.now();

  pollTimer = setInterval(() => {
    refreshButtons();
    const inlineReady =
      document.getElementById(INLINE_BUTTON_ID)?.isConnected ||
      document.querySelector(`.${ACTION_CONTAINER_CLASS}`)
        ?.querySelector(`#${INLINE_BUTTON_ID}`)
        ?.isConnected;

    if (inlineReady || Date.now() - pollStartedAt > POLL_DURATION_MS) {
      stopPolling();
    }
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
  removeInlineDownloadButton();
  scheduleRefresh();
  startPolling();
}

function startObserver() {
  if (observer) {
    return;
  }

  observer = new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      if (mutation.type !== "childList" || mutation.addedNodes.length === 0) {
        continue;
      }

      for (const node of mutation.addedNodes) {
        if (node.nodeType !== Node.ELEMENT_NODE) {
          continue;
        }

        const element = node;
        if (
          element.classList?.contains(ACTION_CONTAINER_CLASS) ||
          element.querySelector?.(`.${ACTION_CONTAINER_CLASS}`) ||
          findMoreMenuButton(element)
        ) {
          scheduleRefresh();
          return;
        }
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

onNavigation();
startObserver();
hookHistory();
watchNavigation();
