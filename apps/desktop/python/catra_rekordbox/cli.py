from __future__ import annotations

import argparse
import json
import sys


def cmd_check(_args: argparse.Namespace) -> int:
    from pyrekordbox import show_config

    show_config()
    return 0


def cmd_info(_args: argparse.Namespace) -> int:
    import pyrekordbox

    print(f"pyrekordbox {pyrekordbox.__version__}")
    print(f"python {sys.version.split()[0]}")
    return 0


def cmd_db_status(_args: argparse.Namespace) -> int:
    from pyrekordbox import Rekordbox6Database

    db = Rekordbox6Database()
    tracks = db.get_content()
    playlists = db.get_playlist()

    payload = {
        "trackCount": tracks.count(),
        "playlistCount": playlists.count(),
    }
    print(json.dumps(payload, ensure_ascii=False))
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="catra-rekordbox",
        description="Catra Rekordbox CLI (pyrekordbox wrapper)",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    check = subparsers.add_parser("check", help="Show pyrekordbox Rekordbox paths/config")
    check.set_defaults(func=cmd_check)

    info = subparsers.add_parser("info", help="Show installed versions")
    info.set_defaults(func=cmd_info)

    db_status = subparsers.add_parser(
        "db-status",
        help="Print Rekordbox master.db track and playlist counts as JSON",
    )
    db_status.set_defaults(func=cmd_db_status)

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
