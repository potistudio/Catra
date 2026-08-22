import { appSession, persistAppSession } from "./appSession.svelte";
import type {
	ActivityLogEntry,
	ActivityLogLevel,
	ActivityLogPayload,
} from "./types";

let nextId = 1;
export const activityLogs = $state<ActivityLogEntry[]>([]);
export const consolePanel = $state({ open: appSession.consoleOpen });

function isActivityLogLevel(value: string): value is ActivityLogLevel {
	return (
		value === "info" ||
		value === "success" ||
		value === "warning" ||
		value === "error"
	);
}

export function setConsoleOpen(open: boolean) {
	consolePanel.open = open;
	appSession.consoleOpen = open;
	persistAppSession();
}

export function toggleConsole() {
	consolePanel.open = !consolePanel.open;
	appSession.consoleOpen = consolePanel.open;
	persistAppSession();
}

export function pushActivityLog(
	level: ActivityLogLevel,
	message: string,
	detail?: string,
): ActivityLogEntry {
	const entry: ActivityLogEntry = {
		id: nextId++,
		timestamp: Date.now(),
		level,
		message,
		detail,
	};
	activityLogs.push(entry);
	return entry;
}

export function pushActivityLogPayload(payload: ActivityLogPayload) {
	const level = isActivityLogLevel(payload.level) ? payload.level : "info";
	pushActivityLog(level, payload.message, payload.detail);
}

export function clearActivityLogs() {
	activityLogs.length = 0;
}
