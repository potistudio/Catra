const INLINE_BUTTON_ID = "catra-sc-inline-download";
const LIST_BUTTON_CLASS = "catra-sc-list-download";
const SUCCESS_RESET_MS = 2000;

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
  return window.location.pathname.split("/").filter(Boolean);
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

function findActionButtonGroup() {
  return (
    document.querySelector(".listenEngagement__actions .sc-button-group") ||
    document.querySelector(".soundActions .sc-button-group")
  );
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
    button.title = "ダウンロード完了";
    button.setAttribute("aria-label", "ダウンロード完了");
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

function createInlineDownloadButton() {
  const button = document.createElement("button");
  button.id = INLINE_BUTTON_ID;
  button.type = "button";
  button.className =
    "sc-button sc-button-secondary sc-button-medium sc-button-icon sc-button-responsive catra-sc-download-btn";
  button.appendChild(createDownloadIcon());

  button.addEventListener("click", async (event) => {
    event.preventDefault();
    event.stopPropagation();

    if (inlineDownloading) {
      return;
    }

    inlineDownloading = true;
    await downloadTrack(getCurrentPageUrl(), button);
  });

  return button;
}

function ensureInlineDownloadButton() {
  if (!isTrackPage() || isPlaylistPage()) {
    removeInlineDownloadButton();
    return;
  }

  const buttonGroup = findActionButtonGroup();
  if (!buttonGroup) {
    return;
  }

  let button = document.getElementById(INLINE_BUTTON_ID);
  if (!button) {
    button = createInlineDownloadButton();
    const moreButton = buttonGroup.querySelector(".sc-button-more");
    if (moreButton) {
      buttonGroup.insertBefore(button, moreButton);
    } else {
      buttonGroup.appendChild(button);
    }
  }

  if (!inlineDownloading) {
    setButtonState(button, "idle");
  }
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
  ];

  const items = new Set();
  for (const selector of selectors) {
    for (const element of document.querySelectorAll(selector)) {
      items.add(element);
    }
  }

  return [...items];
}

function createListDownloadButton(trackUrl) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = `catra-sc-download-btn catra-sc-list-download ${LIST_BUTTON_CLASS}`;
  button.title = "ダウンロード";
  button.setAttribute("aria-label", "ダウンロード");
  button.dataset.trackUrl = trackUrl;
  button.appendChild(createDownloadIcon());

  button.addEventListener("click", async (event) => {
    event.preventDefault();
    event.stopPropagation();
    await downloadTrack(trackUrl, button);
  });

  return button;
}

function ensureListDownloadButtons() {
  for (const item of findTrackItems()) {
    if (item.querySelector(`.${LIST_BUTTON_CLASS}`)) {
      continue;
    }

    const trackUrl = extractTrackUrlFromItem(item);
    if (!trackUrl || trackUrl === getCurrentPageUrl()) {
      continue;
    }

    const actions =
      item.querySelector(".sound__actions") ||
      item.querySelector(".trackItem__actions") ||
      item.querySelector(".soundActions") ||
      item.querySelector(".sc-button-group");

    const button = createListDownloadButton(trackUrl);

    if (actions) {
      actions.appendChild(button);
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
let lastUrl = location.href;

function startObserver() {
  if (observer) {
    return;
  }

  observer = new MutationObserver(() => {
    refreshButtons();
  });

  observer.observe(document.body, {
    childList: true,
    subtree: true,
  });
}

function watchNavigation() {
  setInterval(() => {
    if (location.href !== lastUrl) {
      lastUrl = location.href;
      inlineDownloading = false;
      refreshButtons();
    }
  }, 500);
}

refreshButtons();
startObserver();
watchNavigation();
