#!/usr/bin/env python3
"""Render a command's real terminal output as an SVG in a macOS Terminal-style window.

The README's CLI picture is generated, not screenshotted, so it can be rebuilt whenever the
CLI's output changes and always shows what the binary actually prints:

    cargo build -p synthpass-cli
    python tools/render_terminal_svg.py --prompt "synthpass" \\
        --out knowledge/img/cli-help.svg -- target/debug/synthpass

Everything after `--` is run as the command; its stdout and stderr become the window body, in
the order written. The output is deterministic: fixed character metrics, no timestamps, no
fonts embedded. Viewers fall back to their own monospace font, so the frame is sized from a
fixed character cell rather than measured text.
"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path
from xml.sax.saxutils import escape

FONT_SIZE = 14
CELL_W = 8.43  # width of one monospace character at FONT_SIZE, in px
LINE_H = 19
PAD_X = 18
PAD_TOP = 14
PAD_BOTTOM = 18
TITLE_H = 30

BG = "#1e1e1e"
TITLE_BG = "#323232"
TITLE_FG = "#b0b0b0"
TEXT_FG = "#d4d4d4"
PROMPT_FG = "#5fd75f"
LIGHTS = ("#ff5f57", "#febc2e", "#28c840")
FONT = "'SF Mono', Menlo, Monaco, Consolas, 'DejaVu Sans Mono', monospace"


def render(prompt: str, lines: list[str], title: str) -> str:
    cols = max([len(prompt) + 2] + [len(line) for line in lines])
    width = round(PAD_X * 2 + cols * CELL_W)
    body_lines = len(lines) + 1  # the prompt line
    height = TITLE_H + PAD_TOP + body_lines * LINE_H + PAD_BOTTOM

    out = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
        f'viewBox="0 0 {width} {height}" role="img" aria-label="{escape(title)}">',
        f'<rect width="{width}" height="{height}" rx="10" fill="{BG}"/>',
        f'<path d="M0 10a10 10 0 0 1 10-10h{width - 20}a10 10 0 0 1 10 10v{TITLE_H - 10}'
        f'h-{width}z" fill="{TITLE_BG}"/>',
    ]
    for i, colour in enumerate(LIGHTS):
        out.append(f'<circle cx="{20 + i * 20}" cy="{TITLE_H / 2}" r="6" fill="{colour}"/>')
    out.append(
        f'<text x="{width / 2}" y="{TITLE_H / 2 + 4.5}" text-anchor="middle" '
        f'font-family="-apple-system, \'Segoe UI\', sans-serif" font-size="13" '
        f'fill="{TITLE_FG}">{escape(title)}</text>'
    )
    out.append(
        f'<g font-family="{escape(FONT)}" font-size="{FONT_SIZE}" fill="{TEXT_FG}" '
        f'xml:space="preserve" style="white-space:pre">'
    )
    y = TITLE_H + PAD_TOP + FONT_SIZE
    out.append(
        f'<text x="{PAD_X}" y="{y}" xml:space="preserve"><tspan fill="{PROMPT_FG}">$</tspan> {escape(prompt)}</text>'
    )
    for line in lines:
        y += LINE_H
        if line.strip():
            out.append(f'<text x="{PAD_X}" y="{y}" xml:space="preserve">{escape(line)}</text>')
    out.append("</g>")
    out.append("</svg>")
    return "\n".join(out) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--prompt", required=True, help="the command line shown after the $")
    parser.add_argument("--title", default="synthpass — zsh", help="window title")
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("command", nargs=argparse.REMAINDER, help="-- then the command to run")
    args = parser.parse_args()

    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command:
        parser.error("give the command to run after --")
    # Windows' CreateProcess won't resolve a relative path like target/debug/synthpass;
    # give it an absolute one when the file exists (with or without .exe).
    for candidate in (Path(command[0]), Path(command[0] + ".exe")):
        if candidate.is_file():
            command[0] = str(candidate.resolve())
            break
    result = subprocess.run(command, capture_output=True, text=True, encoding="utf-8")
    text = result.stdout + result.stderr
    lines = text.rstrip("\n").split("\n")

    args.out.write_text(render(args.prompt, lines, args.title), encoding="utf-8", newline="\n")
    print(f"wrote {args.out} ({len(lines)} lines, exit {result.returncode})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
