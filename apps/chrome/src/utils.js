function normalizeTrackUrl(url) {
  const parsed = new URL(url, window.location.origin);
  parsed.hash = "";
  parsed.search = "";
  return parsed.toString().replace(/\/$/, "");
}

window.CatraSC = {
  normalizeTrackUrl,
};
