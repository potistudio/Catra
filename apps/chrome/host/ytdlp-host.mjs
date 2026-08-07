import { spawn } from "node:child_process";

function sendMessage(message) {
  const json = JSON.stringify(message);
  const body = Buffer.from(json, "utf8");
  const header = Buffer.alloc(4);
  header.writeUInt32LE(body.length, 0);
  process.stdout.write(header);
  process.stdout.write(body);
}

async function runYtDlp(url) {
  const args = [
    "-x",
    "--audio-format",
    "mp3",
    "--embed-thumbnail",
    "--add-metadata",
    "--no-playlist",
    "-o",
    "%(uploader)s - %(title)s.%(ext)s",
    "--paths",
    "home:Downloads",
    url,
  ];

  return new Promise((resolve, reject) => {
    const child = spawn("yt-dlp", args, {
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
    });

    let stderr = "";

    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
    });

    child.on("error", (error) => {
      reject(error);
    });

    child.on("close", (code) => {
      if (code === 0) {
        resolve();
        return;
      }

      reject(new Error(stderr.trim() || `yt-dlp exited with code ${code}`));
    });
  });
}

async function handleMessage(message) {
  if (message.type !== "DOWNLOAD") {
    sendMessage({ success: false, error: `Unknown message type: ${message.type}` });
    return;
  }

  if (!message.url) {
    sendMessage({ success: false, error: "URL is required" });
    return;
  }

  try {
    await runYtDlp(message.url);
    sendMessage({ success: true });
  } catch (error) {
    sendMessage({
      success: false,
      error: error?.message ?? "yt-dlp download failed",
    });
  }
}

let buffer = Buffer.alloc(0);

process.stdin.on("data", (chunk) => {
  buffer = Buffer.concat([buffer, chunk]);

  while (buffer.length >= 4) {
    const length = buffer.readUInt32LE(0);
    if (buffer.length < 4 + length) {
      break;
    }

    const json = buffer.slice(4, 4 + length).toString("utf8");
    buffer = buffer.slice(4 + length);

    let message;
    try {
      message = JSON.parse(json);
    } catch {
      sendMessage({ success: false, error: "Invalid JSON message" });
      continue;
    }

    void handleMessage(message);
  }
});
