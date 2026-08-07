const NATIVE_HOST = "com.catra.ytdlp";

function downloadViaYtDlp(trackUrl) {
  return new Promise((resolve) => {
    let settled = false;

    const finish = (result) => {
      if (settled) {
        return;
      }
      settled = true;
      resolve(result);
    };

    let port;
    try {
      port = chrome.runtime.connectNative(NATIVE_HOST);
    } catch (error) {
      finish({
        success: false,
        error: error?.message ?? "Native Messaging Host に接続できませんでした",
      });
      return;
    }

    port.onMessage.addListener((response) => {
      finish(response);
      port.disconnect();
    });

    port.onDisconnect.addListener(() => {
      if (settled) {
        return;
      }

      finish({
        success: false,
        error:
          chrome.runtime.lastError?.message ??
          "Native Messaging Host との接続が切断されました。host/install.ps1 を実行してください。",
      });
    });

    port.postMessage({
      type: "DOWNLOAD",
      url: trackUrl,
    });
  });
}

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message.type !== "DOWNLOAD_TRACK") {
    return false;
  }

  downloadViaYtDlp(message.trackUrl)
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
