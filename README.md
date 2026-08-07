# Catra

SoundCloud トラックをダウンロード・管理するデスクトップアプリと Chrome 拡張機能のモノレポ。

## 構成

```
apps/
  desktop/   Tauri + SvelteKit デスクトップアプリ（ライブラリ管理・yt-dlp ダウンロード）
  chrome/    SoundCloud 用 Chrome 拡張機能（デスクトップアプリへ URL を送信）
```

Chrome 拡張機能は `http://127.0.0.1:17340` で動作するデスクトップアプリのローカル API と連携する。詳細は [apps/chrome/ARCHITECTURE.md](apps/chrome/ARCHITECTURE.md) を参照。

## 開発

[mise](https://mise.jdx.dev/) で Node.js / pnpm / Biome を管理している。

```bash
mise install
pnpm install
pnpm dev          # デスクトップアプリを起動
pnpm check        # Svelte の型チェック
pnpm lint         # Biome によるリント
```

Chrome 拡張機能は `chrome://extensions` から `apps/chrome/` を「パッケージ化されていない拡張機能を読み込む」で読み込む。

## ライセンス

MIT
