const CLIENT_ID_TTL_MS = 60 * 60 * 1000;
const CLIENT_ID_PATTERNS = [
  /client_id[=:]["']([A-Za-z0-9_-]{20,})["']/g,
  /["']client_id["']\s*:\s*["']([A-Za-z0-9_-]{20,})["']/g,
];

let cachedClientId = null;
let cachedClientIdAt = 0;

function findClientIdInText(text) {
  for (const pattern of CLIENT_ID_PATTERNS) {
    pattern.lastIndex = 0;
    let match = pattern.exec(text);

    while (match) {
      if (match[1] && match[1] !== "MISSING_CLIENT_ID") {
        return match[1];
      }
      match = pattern.exec(text);
    }
  }

  return null;
}

function isClientIdCacheValid() {
  return Boolean(cachedClientId) && Date.now() - cachedClientIdAt < CLIENT_ID_TTL_MS;
}

function setCachedClientId(clientId) {
  cachedClientId = clientId;
  cachedClientIdAt = Date.now();
}

async function fetchClientIdFromScripts() {
  const scriptUrls = [
    ...document.querySelectorAll('script[src*="a-v2.sndcdn.com"]'),
  ].map((script) => script.src);

  for (const scriptUrl of [...scriptUrls].reverse()) {
    try {
      const scriptContent = await fetch(scriptUrl, { cache: "force-cache" }).then(
        (response) => response.text(),
      );
      const clientId = findClientIdInText(scriptContent);

      if (clientId) {
        setCachedClientId(clientId);
        return clientId;
      }
    } catch {
      // Try the next bundle.
    }
  }

  return null;
}

async function getClientId() {
  if (isClientIdCacheValid()) {
    return cachedClientId;
  }

  const inlineClientId = findClientIdInText(document.documentElement.innerHTML);
  if (inlineClientId) {
    setCachedClientId(inlineClientId);
    return inlineClientId;
  }

  return fetchClientIdFromScripts();
}

function normalizeTrackUrl(url) {
  const parsed = new URL(url, window.location.origin);
  parsed.hash = "";
  parsed.search = "";
  return parsed.toString().replace(/\/$/, "");
}

function pickTranscoding(transcodings) {
  const progressive = transcodings.find(
    (item) => item.format?.protocol === "progressive",
  );
  if (progressive) {
    return progressive;
  }

  return transcodings.find((item) => item.format?.protocol === "hls") ?? transcodings[0];
}

async function resolveTrack(trackUrl) {
  const clientId = await getClientId();
  if (!clientId) {
    throw new Error("SoundCloud client_id を取得できませんでした");
  }

  const resolveUrl = new URL("https://api-v2.soundcloud.com/resolve");
  resolveUrl.searchParams.set("url", normalizeTrackUrl(trackUrl));
  resolveUrl.searchParams.set("client_id", clientId);

  const response = await fetch(resolveUrl);
  if (!response.ok) {
    throw new Error(`トラック情報の取得に失敗しました (${response.status})`);
  }

  const track = await response.json();
  const transcodings = track.media?.transcodings ?? [];
  if (!transcodings.length) {
    throw new Error("ダウンロード可能なストリームが見つかりませんでした");
  }

  const transcoding = pickTranscoding(transcodings);
  const streamUrl = new URL(transcoding.url);
  streamUrl.searchParams.set("client_id", clientId);

  const streamResponse = await fetch(streamUrl, {
    headers: track.track_authorization
      ? { Authorization: `OAuth ${track.track_authorization}` }
      : undefined,
  });

  if (!streamResponse.ok) {
    throw new Error(`ストリームURLの取得に失敗しました (${streamResponse.status})`);
  }

  const streamData = await streamResponse.json();
  if (!streamData.url) {
    throw new Error("ストリームURLが空です");
  }

  const artist = track.user?.username ?? "Unknown Artist";
  const title = track.title ?? "Unknown Track";
  const extension =
    transcoding.format?.protocol === "progressive"
      ? "mp3"
      : streamData.url.includes(".m4a")
        ? "m4a"
        : "mp3";

  return {
    downloadUrl: streamData.url,
    fileName: sanitizeFileName(`${artist} - ${title}.${extension}`),
    title,
    artist,
  };
}

function sanitizeFileName(fileName) {
  return fileName.replace(/[<>:"/\\|?*\u0000-\u001f]/g, " ").replace(/\s+/g, " ").trim();
}

window.CatraSC = {
  getClientId,
  resolveTrack,
  normalizeTrackUrl,
};
