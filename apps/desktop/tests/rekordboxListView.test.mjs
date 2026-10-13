import assert from "node:assert/strict";
import { test } from "node:test";
import { formatAddedAt } from "../src/lib/format.ts";
import { rekordboxContentToTrack } from "../src/lib/rekordboxListView.ts";

const content = {
	id: "42",
	folderPath: "track.mp3",
	fileName: "track.mp3",
	title: "Track",
	artist: null,
	album: null,
	genre: null,
	bpm: 128,
	lengthSecs: 180,
	trackNo: null,
	bitRate: 320,
	bitDepth: null,
	comment: null,
	fileType: 1,
	rating: null,
	releaseYear: null,
	key: null,
	remixer: null,
	label: null,
	composer: null,
	fileSize: null,
	discNo: null,
	artworkPath: null,
	addedAt: 1_782_876_896,
};

test("Rekordbox addition time is passed to the shared track list", () => {
	const track = rekordboxContentToTrack(content, 3);
	assert.equal(track.id, 3);
	assert.equal(track.addedAt, content.addedAt);
	assert.notEqual(formatAddedAt(track.addedAt), "—");
});

test("playlist rows retain the collection addition time", () => {
	const track = rekordboxContentToTrack(
		{ ...content, songPlaylistId: "song-42" },
		0,
	);
	assert.equal(track.addedAt, content.addedAt);
});

test("missing Rekordbox addition time uses the empty display value", () => {
	const track = rekordboxContentToTrack({ ...content, addedAt: null }, 0);
	assert.equal(track.addedAt, 0);
	assert.equal(formatAddedAt(track.addedAt), "—");
});
