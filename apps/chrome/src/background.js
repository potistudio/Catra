chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message.type !== "DOWNLOAD_TRACK") {
    return false;
  }

  chrome.downloads
    .download({
      url: message.downloadUrl,
      filename: message.fileName,
      saveAs: false,
    })
    .then((downloadId) => {
      sendResponse({ success: true, downloadId });
    })
    .catch((error) => {
      sendResponse({
        success: false,
        error: error?.message ?? "Download failed",
      });
    });

  return true;
});
