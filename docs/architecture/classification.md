# 分類（プレイリストとタグ）

Catra が曲を分類する仕組みを定める。Rekordbox の `djmdPlaylist` / `djmdMyTag` とは独立し、正は `library.db` にある。Rekordbox が未インストールでも全機能が動く。

この文書は方針の確定事項と未決事項を記録する。実装手順ではない。`local-library.md` の「対象外」にあった「Catra 自身のプレイリスト」はこの文書で解禁する。Cue は引き続き対象外である。

## ドメイン

分類には性質の違う3つの層がある。層を混ぜると上書き規則が書けなくなる。

### 属性層 — 曲が持つ値

事実が曲に宿るもの。権威がどこにあるかで2つに分かれる。

| | 例 | 権威 | 再取り込み・タグ再読み込み | 同一性 |
|---|---|---|---|---|
| ファイル由来 | アルバム、ジャンル、BPM、Key、レーティング | ファイル（その先はリリース） | **上書きされる** | 値そのもの |
| ユーザー由来（タグ） | テンション、テーマ、場面 | ユーザーだけ | **絶対に上書きしない** | タグが実体として持つ |

ファイル由来の値は曲が直接持つ文字列・数値である。Catra 側に同一性を作れない。「Dance & EDM」を「EDM」に直すことは、その値を持つ全曲の書き換えである。

ユーザー由来のタグは実体であり ID を持つ。曲はそれを参照する。タグの改名は1箇所の更新で済み、参照している曲はすべて追従する。見た目はジャンルと似ているが、構造は逆である。

### 集合層 — ユーザーが所有する実体

同一性を持ち、中身が空でも存在し続けるもの。外延の決め方が2通りある。

3種類あるが、**「集合」で揃っていない**。静的プレイリストだけが列である。

| 種類 | 実体 | 外延の決め方 | 順序 | 同じ曲の重複 | 要素の同一性 |
|---|---|---|---|---|---|
| 静的プレイリスト | **列** | 曲を並べる | 手動。順序自体が意味を持つ（DJ Mix、セットリスト） | **何度でも現れる** | **要素自身が持つ** |
| スマートプレイリスト | 集合 | 属性への述語を書く | 規則で決める。手動順を持たない | 概念として存在しない | トラック |
| フォルダ | 集合 | 子の集合を束ねる | 子の並び順 | 合併で畳む | トラック |

順序が意味を持つということは、それが集合ではなく列だということである。DJ セットでは同じ曲が複数回鳴る。アカペラを2回落とす、ツール曲をブリッジで使い回す、終盤でリプライズする。列の要素は位置ではなく要素自身が同一性を持つ（並べ替えで位置は動くため）。

この非対称のせいで、静的プレイリストだけが要素 ID を必要とする。同じ曲が3番目と9番目にあるとき、トラック ID では「どちらの要素か」を指せない。

### 同値類 — 実体ではないもの

アルバム、ジャンル、アーティスト、年。属性の値から誘導される集合である。所有者がおらず、同一性は値そのものであり、その値を持つ曲が消えれば集合ごと消える。あとに残る箱が存在しない。

したがって**実体として登録しない**。改名も削除も、問い合わせる相手がいないため意味が決まらない。必要なときは `album = "X"` という述語でスマートプレイリストにできる。そのとき初めて、ユーザーが所有者になる。

## 不変条件

1. 分類の正は `library.db` である。Rekordbox の DB を読み書きしない。
2. プレイリストもタグも、実体ファイルとアートワークを持たない。分類の操作で管轄ディレクトリを一切触らない。
3. **ユーザー由来のタグは、取り込み・再取り込み・タグ再読み込み・Rekordbox 連携のどの経路でも上書きも削除もされない。** ユーザーだけが変えられる。
4. ファイル由来の属性は、再読み込みで上書きされてよい。それが正しい振る舞いである。
5. 静的プレイリストの曲順はユーザーが決めた順である。表示ソートはこの順を書き換えない。
6. トラックのゴミ箱移動で、プレイリストのメンバーシップもタグの割り当ても消えない。消えるのは永久削除のときだけである。

## 決定事項

- **フォルダは合併である。** フォルダを選ぶと、子孫の集合に入っている曲がすべて見える（重複は1回だけ）。単なる入れ物にしない。合併に手動順はないため、既定の並びはライブラリと同じ（アーティスト → タイトル）とする。
- **タグの軸は、軸ごとに単一選択か複数選択かを持つ。** 「テンション」は 低/中/高 のどれか1つ（単一選択）、「テーマ」はメロディックかつ攻撃的がありうる（複数選択）。両方が要る。
- **タグの階層は「軸 > タグ」の2層固定、継承なし。** 深い入れ子は「泣き を付けたら メロディック も付いたことになるのか」という継承の問題を生む。多階層が要るのは集合層だけで足りる。
- **Rekordbox の My Tag には寄せない。** 軸の設計は Catra の都合だけで決める。相互運用も将来の含みも持たせない。
- **同じ曲を同じ静的プレイリストに何度でも入れられる。** 静的プレイリストは集合ではなく列である。DJ Mix でアカペラを2回落とす、ツール曲を使い回す、終盤でリプライズする、はすべて正当な列である。スマートプレイリストとフォルダは集合なので重複は起きない（フォルダの合併は畳む）。
- **階層の深さに上限は設けない。** 禁止するのは循環だけである。
- **同一親の中で同名を許す。**

## データモデル

`library.db` にテーブルを足す。既存の `tracks` は列の追加のみで、意味は変えない。

### 集合層

```sql
CREATE TABLE IF NOT EXISTS playlists (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_id  INTEGER REFERENCES playlists(id) ON DELETE CASCADE,
    kind       TEXT    NOT NULL,   -- 'folder' | 'static' | 'smart'
    name       TEXT    NOT NULL,
    position   INTEGER NOT NULL,   -- 同一 parent 内の 0 起点連番
    rule       TEXT,               -- smart のみ。述語の JSON
    sort_rule  TEXT,               -- smart のみ。並び規則の JSON
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_playlists_parent ON playlists(parent_id, position);

CREATE TABLE IF NOT EXISTS playlist_entries (   -- static のみ
    id          INTEGER PRIMARY KEY AUTOINCREMENT,   -- 要素の同一性。API に出す
    playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    track_id    INTEGER NOT NULL REFERENCES tracks(id)    ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    added_at    INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_playlist_entries_playlist ON playlist_entries(playlist_id, position);
CREATE INDEX IF NOT EXISTS idx_playlist_entries_track    ON playlist_entries(track_id);
```

一意制約は置かない。列なので同じ `track_id` の行が同じ `playlist_id` に何行あってもよい。テーブル名を `playlist_entries` としたのは、中身が「曲」ではなく「列の要素」だからである。`id` は内部の連番ではなく要素の同一性であり、削除と並べ替えの API はこれで要素を指す。

`idx_playlist_entries_track` は残す。`in_playlist` 述語の評価と、重複解決での付け替えで引く。

### 属性層（ユーザー由来）

```sql
CREATE TABLE IF NOT EXISTS tag_axes (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL UNIQUE,
    selection  TEXT    NOT NULL,   -- 'single' | 'multi'
    position   INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS tags (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    axis_id    INTEGER NOT NULL REFERENCES tag_axes(id) ON DELETE CASCADE,
    name       TEXT    NOT NULL,
    position   INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(axis_id, name)
);
CREATE INDEX IF NOT EXISTS idx_tags_axis ON tags(axis_id, position);

CREATE TABLE IF NOT EXISTS track_tags (
    track_id    INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    tag_id      INTEGER NOT NULL REFERENCES tags(id)   ON DELETE CASCADE,
    assigned_at INTEGER NOT NULL,
    PRIMARY KEY (track_id, tag_id)
);
CREATE INDEX IF NOT EXISTS idx_track_tags_tag ON track_tags(tag_id);
```

単一選択の軸は SQLite の宣言では表せない。書き込み経路で守る。単一選択の軸のタグを付けるとき、同じ軸の他のタグを同一トランザクション内で外す。

### 位置の扱い

`position` は 0 起点の連番とし、隙間を作らない。挿入・移動・削除のたびに同一トランザクション内で該当範囲を振り直す。3000曲規模では十分速く、順序キー方式は要らない。

### 外部キーの注意

現在の `LibraryState::new` は `PRAGMA journal_mode=WAL` しか設定しておらず、SQLite の既定では外部キー制約がオフである（`library/db.rs`）。このままだと `ON DELETE CASCADE` が発火せず、永久削除したトラックの行が `playlist_entries` と `track_tags` に残る。接続ごとに `PRAGMA foreign_keys = ON` を立てることを、この機能の必須条件とする。

### 前提: 足りない列

述語を書くための材料が `tracks` に足りない。現在あるのは title / artist / album / genre / bpm / key_name / rating / bitrate_kbps / duration_ms / added_at / source / converted である。**リリース年・レーベル・コメント・リミキサーの列がない。** 年やレーベルで絞る述語を出すなら、取り込み時にタグから読んで `tracks` に列を足す作業が先に要る。この文書では列の追加を前提として扱い、実際に足すまでは該当フィールドを述語の選択肢に出さない。

## 述語

スマートプレイリストの `rule` は JSON で持つ。SQL 文字列を保存しない。

```json
{ "all": [
    { "field": "bpm",   "op": "between",  "value": [128, 132] },
    { "tag":   7 },
    { "any": [
        { "field": "genre", "op": "contains", "value": "house" },
        { "field": "genre", "op": "contains", "value": "techno" }
    ]},
    { "not": { "in_playlist": 12 } }
]}
```

項は4種類とする。

| 項 | 意味 |
|---|---|
| `field` | ファイル由来属性への述語。`op` は `eq` `contains` `between` `gt` `lt` `is_null` など |
| `tag` | ユーザー由来タグの参照。タグ ID を指す |
| `in_playlist` | 他の集合の参照。集合 ID を指す |
| `all` / `any` / `not` | 論理結合 |

決定事項:

- 評価は都度クエリとする。結果を materialize しない。3000曲規模では問題にならない。
- **述語の結果は常に集合である。** 静的プレイリストが列であっても、`in_playlist` は「その曲が列に現れるか」の真偽だけを見る。同じ曲が3回現れても述語の結果は変わらず、スマートプレイリストに同じ曲が3回出ることはない。
- `in_playlist` は循環しうる（スマート A が B を参照し、B が A を参照する）。保存時に循環を検出して拒否する。
- 述語は常にゴミ箱の曲を除く。ゴミ箱を対象にする述語は書けない。
- タグ ID とプレイリスト ID を JSON に埋めるため、参照先を消したときに `rule` が壊れた参照を持つ。削除時に参照している集合を洗い出して警告し、ユーザーが了承したら該当の項だけ落とす。

`sort_rule` は並べる列と昇降だけを持つ。

```json
{ "column": "bpm", "direction": "asc" }
```

## 操作

### 集合

| 操作 | 規則 |
|---|---|
| 作成 | 名前は trim して空を拒否する。上限255文字。親はフォルダのみ。`position` は親の末尾。 |
| 読み出し | フラットな配列を `parent_id, position` 順で返す。木の組み立ては呼び出し側で行う。 |
| 改名 | 名前だけ変える。`updated_at` を更新する。 |
| 移動 | 親と位置を同時に指定する。親がフォルダでない、または自分の子孫であるときは拒否する。移動元・移動先の `position` を振り直す。 |
| 削除 | フォルダは子孫ごと再帰削除する。実行前に子孫の件数を示して確認する。この集合を `in_playlist` で参照しているスマートプレイリストがあれば併せて示す。 |
| 規則の編集 | smart のみ。保存時に循環と壊れた参照を検査する。 |

集合にゴミ箱は作らない。実体ファイルを持たないので二段階削除の理由がない。取り消しは持たない（未決）。

### 静的プレイリストの要素

| 操作 | 規則 |
|---|---|
| 追加 | 複数トラックをまとめて受ける。**既に入っている曲でも黙って挿入する。** 作った要素 ID を順に返す。位置を省略すると末尾。 |
| 読み出し | `position` 順に要素 ID 付きで返す。ゴミ箱の曲も含めて返し、除外は呼び出し側で行う。 |
| 並べ替え | プレイリスト内の**要素 ID** の完全な配列を受け取り、その順に振り直す。差分でなく全体を渡すので冪等になる。集合が現在と一致しなければ拒否する。 |
| 削除（要素） | **要素 ID** を受け、行を消して `position` を詰める。同じ曲の他の要素は残る。 |
| 削除（曲すべて） | トラック ID を受け、その曲の要素をすべて消す。「この曲をこの列から追い出す」用。 |

どちらの削除もトラック本体とファイルには触らない。

**要素 ID は API に出す。** 列なのでトラック ID では要素を指せない。同じ曲が3番目と9番目にあるとき、「9番目だけ消す」「9番目だけ前に動かす」がトラック ID では表現できない。

これは既存の Rekordbox 側と同じ形である。あちらも要素に独自の同一性を与え、削除と移動は `songPlaylistId` で指している（`apps/desktop/src/lib/api.ts:156,166`、`apps/desktop/src-tauri/src/rekordbox/playlist.rs:431,500`）。

### タグ

| 操作 | 規則 |
|---|---|
| 軸の作成・改名・削除 | 名前は全体で一意。削除は配下のタグと割り当てごと消す。件数を示して確認する。 |
| 軸の選択方式の変更 | 複数選択 → 単一選択にするとき、既に複数付いている曲がある。件数を示し、どれを残すか（最後に付けたもの／中止）を選ばせる。 |
| タグの作成・改名・削除 | 名前は軸の中で一意。改名は1箇所の更新で、参照している曲はすべて追従する。 |
| 曲への割り当て | 複数トラック × 複数タグをまとめて受ける。単一選択の軸なら、同じ軸の既存タグを同一トランザクションで外す。 |
| 曲からの解除 | 複数トラック × 複数タグをまとめて受ける。 |

## ゴミ箱・重複・変換との関係

| 出来事 | プレイリスト | タグ |
|---|---|---|
| トラックをゴミ箱へ | 要素は残す。表示では既定で隠し、「ゴミ箱 n 件」を示す | 割り当ては残す |
| ゴミ箱から復元 | 元の位置のまま戻る | そのまま |
| 永久削除・ゴミ箱を空に | 要素がカスケードで消える | カスケードで消える |
| 重複解決「新しい方を残す」 | **負け側の要素の参照先を勝ち側へ付け替える。位置も出現回数もそのまま保つ。** 畳まない | **負け側の割り当てを勝ち側へ引き継ぐ。** 単一選択の軸で衝突したら勝ち側の既存を優先 |
| 重複解決「別フォーマットとして取り込む」 | 何もしない | 何もしない |
| 変換で派生トラックができた | 何もしない。自動で入れない | 何もしない。自動で継がない |
| タグ再読み込み・再取り込み | 影響なし | **影響なし。不変条件3。** |

引き継ぎをやらないと、ユーザーから見て分類が黙って消える。

ライブラリ健全性チェックの対象外とする。分類は実体を指さないのでパス切れが起きない。

## Tauri コマンド

接頭辞は `playlist_` と `tag_` とする。既存の `library_` / `rekordbox_` に並ぶ。

```
playlist_list_tree()                                  -> Vec<PlaylistNode>
playlist_create(name, parentId?, kind)                -> PlaylistNode
playlist_rename(id, name)                             -> PlaylistNode
playlist_move(id, parentId?, position)                -> ()
playlist_delete(id)                                   -> u32       // 消したノード数
playlist_set_rule(id, rule, sortRule)                 -> PlaylistNode
playlist_entries(id)                                  -> Vec<PlaylistEntry>  // kind を問わず解決済み
playlist_add_tracks(playlistId, trackIds, position?)  -> Vec<i64>  // 作った要素 ID
playlist_remove_entries(playlistId, entryIds)         -> u32       // この要素だけ
playlist_remove_tracks(playlistId, trackIds)          -> u32       // その曲の要素をすべて
playlist_reorder_entries(playlistId, entryIds)        -> ()

tag_list_axes()                                       -> Vec<TagAxis>   // タグを含む
tag_create_axis(name, selection)                      -> TagAxis
tag_update_axis(id, name?, selection?)                -> TagAxis
tag_delete_axis(id)                                   -> u32
tag_create(axisId, name)                              -> Tag
tag_rename(id, name)                                  -> Tag
tag_delete(id)                                        -> u32
tag_assign(trackIds, tagIds)                          -> u32
tag_unassign(trackIds, tagIds)                        -> u32
tag_of_tracks(trackIds)                               -> Map<trackId, tagIds>

browse_facet(field)                                   -> Vec<FacetValue>  // 値と件数。保存しない
browse_facet_tracks(field, value)                     -> Vec<Track>       // 束の中身。保存しない
browse_tag_tracks(tagIds)                             -> Vec<Track>       // AND で絞る。保存しない
```

`playlist_entries` は kind を隠す。static なら列の順、smart なら述語を解いて `sort_rule` 順、folder なら子孫の合併を既定順で返す。呼び出し側は種類を意識しなくてよい。

`PlaylistEntry.entryId` は static のときだけ値が入り、smart と folder では null になる。null は「手動の同一性を持たない＝並べ替えできない」を自然に表すので、呼び出し側は kind を見なくても並べ替えの可否を判断できる。

すべて同期コマンドとし、進捗イベントは出さない。失敗は既存どおり `Result<_, String>` で文字列を返す。

## 型（フロント）

```ts
export type PlaylistKind = "folder" | "static" | "smart";

export interface PlaylistNode {
  id: number;
  parentId: number | null;
  kind: PlaylistKind;
  name: string;
  position: number;
  /** ゴミ箱の曲を除いた件数。static は列の長さ（重複を数える）、smart と folder は集合の大きさ。 */
  trackCount: number;
  rule: Rule | null;
  sortRule: SortRule | null;
  createdAt: number;
  updatedAt: number;
}

export type TagSelection = "single" | "multi";

export interface TagAxis {
  id: number;
  name: string;
  selection: TagSelection;
  position: number;
  tags: Tag[];
}

export interface Tag {
  id: number;
  axisId: number;
  name: string;
  position: number;
  trackCount: number;
}

export interface PlaylistEntry {
  /** 要素の同一性。static のみ。smart / folder は null（並べ替えできない）。 */
  entryId: number | null;
  position: number;
  track: Track;
}
```

曲そのものは既存の `Track` をそのまま使う。`trashedAt` があるのでゴミ箱判定はフロントでできる。ただし **`Track[]` だけでは列を表せない。** 同じ `Track.id` が2回現れたとき、どちらの要素かを区別できないため、必ず `PlaylistEntry[]` で受け渡す。

合計時間を出すときも同じ数え方に従う。static では重複を重複のまま足す（2回鳴らすなら2回分）。smart と folder は集合なので1曲を1回だけ足す。

## UI

ここから下は見せ方の話であり、上のドメインとは分けて扱う。

### 置き場所

Rekordbox タブが持つ `browseMode: "all" | "playlist"` と同じ構造を library タブに足す。第4のタブは作らない。分類はライブラリの見せ方であり、別のライブラリではない。

`appSession` を広げる。

```ts
library: {
  browseMode: BrowseMode;          // "all" | "playlist"
  selectedPlaylistId: number | null;
  expandedPlaylistIds: number[];
  list: TrackListSession;
}
```

既存の `library: TrackListSession` からの形の変更になるため、`parseSession` は旧形を読んだら `library.list` へ移す。保存キー `catra.app-session.v1` は据え置く。

### 集合

- 左に木を出す。行はインデント＋開閉＋種類アイコン（フォルダ／プレイリスト／スマート）＋名前＋件数。
- 右クリック: 新規プレイリスト／新規スマートプレイリスト／新規フォルダ／名前を変更／削除。
- 名前の入力は `RekordboxList.svelte` のテキストプロンプトモーダルを使い回す。
- スマートプレイリストは規則エディタを別モーダルで出す。条件行の追加・削除と `all` / `any` の切り替えができれば足りる。入れ子は1段まで出せればよい。

### タグ

- 曲を選んだときの詳細に、軸ごとのタグを出す。単一選択の軸はラジオ、複数選択の軸はトグル。
- 複数曲を選んで一括で付け外しできる。3曲中2曲に付いている状態は中間表示にする。
- タグでの絞り込みは、軸を並べて値をクリックする形にする。複数軸を選んだら AND で効く。
- よく使う絞り込みは「スマートプレイリストにする」ボタンで集合層へ昇格させる。

### 同値類のブラウズ

アルバム・ジャンル・アーティスト・年は、木に置かず絞り込み用の別枠に出す。`browse_facet` の結果をその場で表示し、どこにも保存しない。ここも「スマートプレイリストにする」で昇格させられる。

助手くんのライブラリの実測では、ジャンルは178種類あり、うち105種類は1曲しか持たない。アルバムは183種類で、うち73種類は1曲だけである。権威が配信元にあるため値が揺れる。この枠は「揺れたままの値を眺める場所」であり、整えたい分類はタグに移す。

### 表示ソート

静的プレイリストの既定ソートは「プレイリスト順」とする。`SortColumn` に `"position"` を足し、これを既定にする。列ヘッダで別のソートを選んだら表示だけ並べ替え、`position` は書き換えない。この状態では曲順のドラッグを無効にし、「プレイリスト順に戻す」を出す。

スマートとフォルダは手動順を持たないので、この切り替えを出さない。

### 行のキーと選択の単位

静的プレイリスト表示では、行のキーと選択の単位を**要素**にする。トラック ID をキーにすると同じ曲の2つの要素が1行に潰れる。

現在の `TrackList.svelte` / `TrackRow.svelte` / `SelectionCheckbox.svelte` は `Track.id` が一意である前提で書かれている。要素キーを外から渡せる形にする必要がある。smart と folder は `entryId` が null なので、そのときだけトラック ID をキーにしてよい。

### ドラッグ&ドロップ

- 曲リストの選択行 → 静的プレイリストにドロップで追加。フォルダとスマートはドロップ不可。
- **既に入っている曲を落としても、警告も確認も出さない。** 列なので正当な操作である。黙って末尾（またはドロップ位置）に足す。
- 曲リストの選択行 → タグにドロップで割り当て。
- 木の中でノードをドラッグして親と順序を変える。行の上下境界で「間に挟む」、中央で「フォルダの中へ」を出し分ける。
- 静的プレイリスト表示中は、曲リスト内のドラッグで**要素**を動かす。同じ曲が2つあるとき、片方だけを動かせる。
- OS からのファイルの D&D は既存どおりライブラリ取り込みへ。プレイリスト行に落ちたら、取り込み後にそこへ追加する。

### ゴミ箱の曲

既定では隠す。「ゴミ箱 n 件」を出し、クリックで表示に切り替える。表示したらグレーアウトし、プレビュー再生は不可とする。

## 対象外（この段階では作らない）

- Rekordbox の My Tag との相互運用、および将来の含みを持たせた設計
- Rekordbox へのプレイリストのエクスポート、Rekordbox からのインポート
- M3U / XML の読み書き
- 分類の共有・書き出し
- タグの3層以上の入れ子と継承
- 同値類（アルバム・ジャンル等）を実体として登録すること
- Cue（`local-library.md` のとおり Rekordbox 側に残す）
- 分類の履歴と取り消し

## 決めたこと（実装で確定した分）

- 同じ曲の要素を2つ選んだ状態でトラック単位の操作（タグ付け、変換、Rekordbox、ゴミ箱）を実行したら、**曲に畳んで1回だけ効かせる**。曲への操作は曲に効くもので、列の中の出現回数とは関係がない。
- 列から外す操作だけは**要素単位**のまま。同じ曲の片方だけを外せる。一覧の一括バーに「プレイリストから外す」として出し、「同じ曲を全部消す」は当面出さない（`playlist_remove_tracks` は API としては持つ）。

## 未決

- 削除の取り消し（Ctrl+Z）を持つか。持つなら集合行の soft delete が要る。
- 述語の項に「タグが未設定」を出すか。「テンション未設定の曲」は整理に効くが、単一選択の軸でしか意味が安定しない。
- ファイル由来のジャンルを、初回にタグへ取り込む導線を出すか。178種類をそのまま持ち込むと汚れが移る。
- `tracks` へのリリース年・レーベル・コメント・リミキサー列の追加時期。述語の表現力がこれに縛られる。
- 曲数が数万件になったときの `position` 振り直しと述語評価の速度。

## 実装の当たり

| 場所 | 内容 |
|---|---|
| `src-tauri/src/library/db.rs` | テーブル定義を追加。`PRAGMA foreign_keys = ON` を追加。 |
| `src-tauri/src/library/playlist.rs` | 新規。木の操作、列への要素の挿入・削除・並べ替え、`position` 振り直し、循環判定、合併の解決。 |
| `src-tauri/src/library/rule.rs` | 新規。述語 JSON → SQL の組み立て、循環と壊れた参照の検査。 |
| `src-tauri/src/library/tag.rs` | 新規。軸とタグ、単一選択の強制、一括割り当て。 |
| `src-tauri/src/library/duplicate.rs` 周辺 | 「新しい方を残す」でのメンバーシップとタグの引き継ぎ。 |
| `src-tauri/src/library/mod.rs` | `pub use` を追加。 |
| `src-tauri/src/commands/playlist.rs`, `tag.rs` | 新規。Tauri コマンド。 |
| `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs` | コマンド登録。 |
| `src/lib/types.ts`, `src/lib/api.ts` | 型と invoke ラッパ。 |
| `src/lib/playlistTree.ts` | 新規。フラット配列 → 表示行。 |
| `src/lib/components/PlaylistTree.svelte` | 新規。木と右クリックメニュー。 |
| `src/lib/components/RuleEditor.svelte` | 新規。述語エディタ。 |
| `src/lib/components/TagPanel.svelte` | 新規。軸ごとのタグ表示と一括付け外し。 |
| `src/lib/appSession.svelte.ts` | `library` の形を拡張し、旧形を読み替える。 |
| `src/lib/trackListView.ts` | `SortColumn` に `"position"` を追加。 |
| `src/lib/components/TrackList.svelte`, `TrackRow.svelte`, `SelectionCheckbox.svelte` | 現在は `Track.id` が一意である前提。行のキーと選択の単位を要素にできる形に開く。 |
| `src/routes/+page.svelte` | library タブに browseMode を足し、木とリストをつなぐ。 |

テストは `library/trash.rs` と同じく Rust 側のユニットテストで置く。最低限、循環移動の拒否、フォルダ再帰削除、`position` の連番維持、永久削除でのカスケード、重複解決での引き継ぎ、単一選択軸の排他、述語の循環拒否、再取り込みでタグが残ることを押さえる。

列であることに由来する分は別に押さえる。同じ曲を2回入れて要素が2つできること、片方だけを消しても残る方が動かないこと、要素の挿入・削除のあとも `position` が0から連番であること、同じ曲の2つの要素が独立に動くこと、重複を含む列を並べ替えても出現回数が変わらないこと、重複解決の引き継ぎで出現回数と位置が保たれること。
