import { type AppSettings, readSettings, writeSettings } from "./settings";

export const appSettings = $state<AppSettings>(readSettings());

export function saveAppSettings(settings: AppSettings) {
	const saved = writeSettings(settings);
	Object.assign(appSettings, saved);
}
