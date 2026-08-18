import type { SortColumn, SortDirection, ViewMode } from "$lib/trackListView";

const STORAGE_KEY = "catra.app-session.v1";
const PERSIST_MS = 250;

export type AppTab = "library" | "rekordbox" | "trash";
export type MembershipFilter = "all" | "missing" | "present";
export type BrowseMode = "all" | "playlist";

export type TrackListSession = {
  query: string;
  viewMode: ViewMode;
  sortColumn: SortColumn;
  sortDirection: SortDirection;
  membershipFilter: MembershipFilter;
};

export type AppSession = {
  activeTab: AppTab;
  selectedTrackId: number | null;
  selectedRekordboxId: string | null;
  consoleOpen: boolean;
  library: TrackListSession;
  trash: TrackListSession;
  rekordbox: {
    browseMode: BrowseMode;
    selectedPlaylistId: string | null;
    selectedFolderId: string | null;
    list: TrackListSession;
  };
};

const SORT_COLUMNS: readonly SortColumn[] = [
  "title",
  "artist",
  "album",
  "bpm",
  "bitrateKbps",
  "key",
  "genre",
  "rating",
  "durationMs",
  "addedAt",
];

function defaultList(): TrackListSession {
  return {
    query: "",
    viewMode: "list",
    sortColumn: "artist",
    sortDirection: "asc",
    membershipFilter: "all",
  };
}

function defaultSession(): AppSession {
  return {
    activeTab: "library",
    selectedTrackId: null,
    selectedRekordboxId: null,
    consoleOpen: false,
    library: defaultList(),
    trash: defaultList(),
    rekordbox: {
      browseMode: "all",
      selectedPlaylistId: null,
      selectedFolderId: null,
      list: defaultList(),
    },
  };
}

function isAppTab(value: unknown): value is AppTab {
  return value === "library" || value === "rekordbox" || value === "trash";
}

function isViewMode(value: unknown): value is ViewMode {
  return value === "list" || value === "grid";
}

function isSortColumn(value: unknown): value is SortColumn {
  return typeof value === "string" && (SORT_COLUMNS as readonly string[]).includes(value);
}

function isSortDirection(value: unknown): value is SortDirection {
  return value === "asc" || value === "desc";
}

function isMembershipFilter(value: unknown): value is MembershipFilter {
  return value === "all" || value === "missing" || value === "present";
}

function isBrowseMode(value: unknown): value is BrowseMode {
  return value === "all" || value === "playlist";
}

function asString(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function asNullableId(value: unknown): number | null {
  return typeof value === "number" && Number.isInteger(value) ? value : null;
}

function parseList(raw: unknown): TrackListSession {
  const next = defaultList();
  if (!raw || typeof raw !== "object") return next;
  const record = raw as Record<string, unknown>;
  if (typeof record.query === "string") next.query = record.query;
  if (isViewMode(record.viewMode)) next.viewMode = record.viewMode;
  if (isSortColumn(record.sortColumn)) next.sortColumn = record.sortColumn;
  if (isSortDirection(record.sortDirection)) next.sortDirection = record.sortDirection;
  if (isMembershipFilter(record.membershipFilter)) {
    next.membershipFilter = record.membershipFilter;
  }
  return next;
}

function parseSession(raw: unknown): AppSession {
  const next = defaultSession();
  if (!raw || typeof raw !== "object") return next;
  const record = raw as Record<string, unknown>;
  if (isAppTab(record.activeTab)) next.activeTab = record.activeTab;
  next.selectedTrackId = asNullableId(record.selectedTrackId);
  next.selectedRekordboxId = asString(record.selectedRekordboxId);
  if (typeof record.consoleOpen === "boolean") next.consoleOpen = record.consoleOpen;
  next.library = parseList(record.library);
  next.trash = parseList(record.trash);
  if (record.rekordbox && typeof record.rekordbox === "object") {
    const rekordbox = record.rekordbox as Record<string, unknown>;
    if (isBrowseMode(rekordbox.browseMode)) next.rekordbox.browseMode = rekordbox.browseMode;
    next.rekordbox.selectedPlaylistId = asString(rekordbox.selectedPlaylistId);
    next.rekordbox.selectedFolderId = asString(rekordbox.selectedFolderId);
    next.rekordbox.list = parseList(rekordbox.list);
  }
  return next;
}

function readStored(): AppSession {
  if (typeof localStorage === "undefined") return defaultSession();
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return defaultSession();
    return parseSession(JSON.parse(raw));
  } catch {
    return defaultSession();
  }
}

export const appSession = $state<AppSession>(readStored());

let persistTimer: ReturnType<typeof setTimeout> | null = null;

export function flushAppSession() {
  if (persistTimer != null) {
    clearTimeout(persistTimer);
    persistTimer = null;
  }
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(STORAGE_KEY, JSON.stringify(appSession));
}

export function persistAppSession() {
  if (typeof localStorage === "undefined") return;
  if (persistTimer != null) clearTimeout(persistTimer);
  persistTimer = setTimeout(() => {
    persistTimer = null;
    localStorage.setItem(STORAGE_KEY, JSON.stringify(appSession));
  }, PERSIST_MS);
}

if (typeof window !== "undefined") {
  window.addEventListener("pagehide", flushAppSession);
}
