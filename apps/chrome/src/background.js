const CATRA_API = "http://127.0.0.1:17340";

async function ensureCatraAvailable() {
  try {
    const health = await fetch(`${CATRA_API}/health`);
    if (!health.ok) {
      throw new Error("Catra が起動していません");
    }
  } catch {
    throw new Error("Catra が起動していません。デスクトップアプリを起動してください。");
  }
}

async function postJson(path, body) {
  await ensureCatraAvailable();

  const response = await fetch(`${CATRA_API}${path}`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
  });

  const result = await response.json().catch(() => null);
  if (!response.ok || !result?.success) {
    throw new Error(result?.error ?? "ダウンロードの開始に失敗しました");
  }

  return result;
}

async function getDownloadProgress(trackUrl) {
  await ensureCatraAvailable();

  const response = await fetch(
    `${CATRA_API}/download/progress?url=${encodeURIComponent(trackUrl)}`,
  );
  const result = await response.json().catch(() => null);
  if (!response.ok || !result?.success) {
    throw new Error(result?.error ?? "進捗の取得に失敗しました");
  }

  return result;
}

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message.type === "GET_DOWNLOAD_PROGRESS") {
    getDownloadProgress(message.trackUrl)
      .then((result) => {
        sendResponse(result);
      })
      .catch((error) => {
        sendResponse({
          success: false,
          error: error?.message ?? "進捗の取得に失敗しました",
        });
      });

    return true;
  }

  if (message.type === "DOWNLOAD_TRACK") {
    postJson("/download", { url: message.trackUrl })
      .then((result) => {
        sendResponse(result);
      })
      .catch((error) => {
        sendResponse({
          success: false,
          error: error?.message ?? "ダウンロードに失敗しました",
        });
      });

    return true;
  }

  if (message.type === "DOWNLOAD_PLAYLIST") {
    postJson("/download/playlist", { url: message.playlistUrl })
      .then((result) => {
        sendResponse(result);
      })
      .catch((error) => {
        sendResponse({
          success: false,
          error: error?.message ?? "プレイリストのダウンロードに失敗しました",
        });
      });

    return true;
  }

  return false;
});
