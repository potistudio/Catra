# Catra

SoundCloud・YouTubeの音源をダウンロード・管理するデスクトップアプリと Chrome 拡張機能のモノレポ。

## 構成

```
apps/
  desktop/   Tauri + SvelteKit デスクトップアプリ（ライブラリ管理・yt-dlp ダウンロード）
  chrome/    SoundCloud・YouTube 用 Chrome 拡張機能（デスクトップアプリへ URL を送信）
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

macOS では SQLCipher 用の OpenSSL をソースからビルドするため、Perl と C コンパイラが必要。
Windows では [Shining Light OpenSSL Light](https://slproweb.com/products/Win32OpenSSL.html) をインストールし、ビルド前に PowerShell で設定する。

```powershell
$env:OPENSSL_DIR = "C:\Program Files\OpenSSL-Win64"
$env:OPENSSL_INCLUDE_DIR = "$env:OPENSSL_DIR\include"
# pnpm dev (debug):
$env:OPENSSL_LIB_DIR = "$env:OPENSSL_DIR\lib\VC\x64\MDd"
# pnpm build (release):
# $env:OPENSSL_LIB_DIR = "$env:OPENSSL_DIR\lib\VC\x64\MD"
```

OpenSSL の設定を変更した後は `apps/desktop/src-tauri` で `cargo clean` を実行する。

Tauri CLI はプロジェクトルートから実行できる。サブコマンドとオプションは `apps/desktop` の CLI に転送される。

```bash
pnpm tauri dev
pnpm tauri build
pnpm tauri --help
```

Chrome 拡張機能は `chrome://extensions` から `apps/chrome/` を「パッケージ化されていない拡張機能を読み込む」で読み込む。

## ライセンス

MIT
