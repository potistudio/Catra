function normalizeTrackUrl(url) {
	const parsed = new URL(url, window.location.origin);
	parsed.hash = "";
	parsed.search = "";
	return parsed.toString().replace(/\/$/, "");
}

function normalizeSoundCloudTrackUrl(value) {
	try {
		if (typeof value !== "string" || !value.trim()) return null;
		const parsed = new URL(value.trim(), window.location.origin);
		if (
			!["http:", "https:"].includes(parsed.protocol) ||
			!["soundcloud.com", "www.soundcloud.com", "m.soundcloud.com"].includes(
				parsed.hostname,
			) ||
			parsed.username ||
			parsed.password ||
			parsed.port
		)
			return null;
		const parts = parsed.pathname.split("/").filter(Boolean);
		if (parts[0] === "n") parts.shift();
		if (parts.length === 3 && parts[2].startsWith("s-")) parts.pop();
		const reserved = new Set([
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
		if (parts.length !== 2 || parts.some((part) => reserved.has(part)))
			return null;
		return `https://soundcloud.com/${parts.join("/")}`;
	} catch {
		return null;
	}
}

window.CatraExtension = {
	normalizeTrackUrl,
	normalizeSoundCloudTrackUrl,
};
