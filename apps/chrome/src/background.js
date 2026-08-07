const CATRA_API = "http://127.0.0.1:17340";

async function downloadViaCatra(trackUrl) {
  try {
    const health = await fetch(`${CATRA_API}/health`);
    if (!health.ok) {
      throw new Error("Catra が起動していません");
    }
  } catch {
    throw new Error("Catra が起動していません。デスクトップアプリを起動してください。");
  }

  const response = await fetch(`${CATRA_API}/download`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ url: trackUrl }),
  });

  const result = await response.json().catch(() => null);
  if (!response.ok || !result?.success) {
    throw new Error(result?.error ?? "ダウンロードの開始に失敗しました");
  }

  return result;
}

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message.type !== "DOWNLOAD_TRACK") {
    return false;
  }

  downloadViaCatra(message.trackUrl)
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
});
