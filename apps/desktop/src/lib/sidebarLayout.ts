export const SIDEBAR_MIN_WIDTH = 224;
export const SIDEBAR_MAX_WIDTH = 480;
export const SIDEBAR_DEFAULT_WIDTH = 280;
const CONTENT_MIN_WIDTH = 320;

export type SidebarLayout = {
	width: number;
	playlistsOpen: boolean;
	tagsOpen: boolean;
	facetsOpen: boolean;
};

export type SidebarSection = Exclude<keyof SidebarLayout, "width">;

export function defaultSidebarLayout(): SidebarLayout {
	return {
		width: SIDEBAR_DEFAULT_WIDTH,
		playlistsOpen: true,
		tagsOpen: true,
		facetsOpen: true,
	};
}

export function sidebarMaxWidth(containerWidth: number): number {
	return Math.max(
		SIDEBAR_MIN_WIDTH,
		Math.min(SIDEBAR_MAX_WIDTH, containerWidth - CONTENT_MIN_WIDTH),
	);
}

export function clampSidebarWidth(
	width: number,
	maxWidth = SIDEBAR_MAX_WIDTH,
): number {
	const value = Number.isFinite(width) ? width : SIDEBAR_DEFAULT_WIDTH;
	return Math.round(Math.max(SIDEBAR_MIN_WIDTH, Math.min(maxWidth, value)));
}

export function parseSidebarLayout(raw: unknown): SidebarLayout {
	const next = defaultSidebarLayout();
	if (!raw || typeof raw !== "object") return next;
	const record = raw as Record<string, unknown>;
	if (typeof record.width === "number") {
		next.width = clampSidebarWidth(record.width);
	}
	for (const key of ["playlistsOpen", "tagsOpen", "facetsOpen"] as const) {
		if (typeof record[key] === "boolean") next[key] = record[key];
	}
	return next;
}
