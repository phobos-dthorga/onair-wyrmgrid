#!/usr/bin/env python3
"""Build deterministic WyrmGrid promotional artwork from immutable originals."""

from __future__ import annotations

import hashlib
import json
import sys
from dataclasses import dataclass
from pathlib import Path

try:
    from PIL import Image, ImageChops, ImageDraw, ImageFont
except ImportError as error:  # pragma: no cover - environment guidance
    raise SystemExit(
        "Pillow is required. Install assets/brand/key-art/requirements.txt."
    ) from error


ROOT = Path(__file__).resolve().parent
ORIGINALS = ROOT / "originals"
DERIVATIVES = ROOT / "derivatives"

CANVAS = (7, 17, 15)
TEXT = (233, 241, 239)
MUTED = (167, 184, 178)
ACCENT = (115, 214, 173)
HIGHLIGHT = (213, 174, 95)


@dataclass(frozen=True)
class Theme:
    name: str
    source: str
    crop_bias_y: float
    veil_alpha: int


THEMES = (
    Theme("dark", "candidate-01-atlas-nocturne.png", 0.42, 188),
    Theme("light", "candidate-03-world-in-motion.png", 0.48, 174),
)

FORMATS = {
    "hero": (1600, 900),
    "social": (1280, 640),
}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest().upper()


def font_path(*names: str) -> Path:
    directories = (
        Path("C:/Windows/Fonts"),
        Path("/usr/share/fonts/truetype/msttcorefonts"),
        Path("/Library/Fonts"),
    )
    for directory in directories:
        for name in names:
            candidate = directory / name
            if candidate.exists():
                return candidate
    raise SystemExit(f"Required brand font was not found: {', '.join(names)}")


SERIF = font_path("georgia.ttf", "Georgia.ttf")
SERIF_BOLD = font_path("georgiab.ttf", "Georgia Bold.ttf")
SANS = font_path("segoeui.ttf", "Arial.ttf")
SANS_BOLD = font_path("seguisb.ttf", "Arial Bold.ttf")


def cover(source: Image.Image, size: tuple[int, int], bias_y: float) -> Image.Image:
    target_width, target_height = size
    scale = max(target_width / source.width, target_height / source.height)
    resized = source.resize(
        (round(source.width * scale), round(source.height * scale)),
        Image.Resampling.LANCZOS,
    )
    left = max(0, (resized.width - target_width) // 2)
    spare_y = max(0, resized.height - target_height)
    top = round(spare_y * bias_y)
    return resized.crop((left, top, left + target_width, top + target_height))


def add_readability_veil(image: Image.Image, alpha: int) -> None:
    width, height = image.size
    horizontal = Image.new("L", (width, 1))
    horizontal.putdata(
        [round(alpha * max(0.0, 1.0 - x / (width * 0.62))) for x in range(width)]
    )
    horizontal = horizontal.resize(image.size)

    vertical = Image.new("L", (1, height))
    vertical.putdata(
        [
            round(alpha * 0.34 * max(0.0, 1.0 - y / (height * 0.72)))
            for y in range(height)
        ]
    )
    vertical = vertical.resize(image.size)

    overlay = Image.new("RGBA", image.size, (*CANVAS, 0))
    overlay.putalpha(ImageChops.lighter(horizontal, vertical))
    image.alpha_composite(overlay)


def draw_tracking_text(
    draw: ImageDraw.ImageDraw,
    origin: tuple[int, int],
    text: str,
    font: ImageFont.FreeTypeFont,
    fill: tuple[int, int, int],
    tracking: int,
) -> None:
    x, y = origin
    for character in text:
        draw.text((x, y), character, font=font, fill=fill)
        bounds = draw.textbbox((x, y), character, font=font)
        x = bounds[2] + tracking


def draw_brand(image: Image.Image, format_name: str) -> None:
    width, height = image.size
    scale = width / 1600
    x = round(74 * scale)
    y = round((66 if format_name == "hero" else 50) * scale)
    mark = round(58 * scale)
    line_width = max(2, round(2 * scale))

    draw = ImageDraw.Draw(image)
    centre_x = x + mark // 2
    centre_y = y + mark // 2
    draw.polygon(
        (
            (centre_x, y),
            (x + mark, centre_y),
            (centre_x, y + mark),
            (x, centre_y),
        ),
        outline=HIGHLIGHT,
        width=line_width,
    )

    mark_font = ImageFont.truetype(SERIF, max(13, round(18 * scale)))
    mark_bounds = draw.textbbox((0, 0), "WG", font=mark_font)
    mark_width = mark_bounds[2] - mark_bounds[0]
    mark_height = mark_bounds[3] - mark_bounds[1]
    draw.text(
        (
            centre_x - mark_width / 2,
            centre_y - mark_height / 2 - mark_bounds[1],
        ),
        "WG",
        font=mark_font,
        fill=HIGHLIGHT,
    )

    copy_x = x + mark + round(25 * scale)
    eyebrow_font = ImageFont.truetype(SANS_BOLD, max(12, round(15 * scale)))
    title_font = ImageFont.truetype(SERIF, max(36, round(57 * scale)))
    tagline_font = ImageFont.truetype(SANS, max(17, round(23 * scale)))

    draw_tracking_text(
        draw,
        (copy_x, y - round(3 * scale)),
        "ONAIR",
        eyebrow_font,
        HIGHLIGHT,
        max(2, round(4 * scale)),
    )
    title_y = y + round(13 * scale)
    draw.text((copy_x, title_y), "WyrmGrid", font=title_font, fill=TEXT)

    tagline_y = y + mark + round(22 * scale)
    draw.line(
        (x, tagline_y - round(9 * scale), x + round(48 * scale), tagline_y - round(9 * scale)),
        fill=ACCENT,
        width=max(2, round(3 * scale)),
    )
    draw.text(
        (x, tagline_y),
        "See the network. Command the skies.",
        font=tagline_font,
        fill=MUTED,
    )

    rule_y = height - max(6, round(8 * scale))
    draw.rectangle((0, rule_y, width * 0.68, height), fill=ACCENT)
    draw.rectangle((round(width * 0.68), rule_y, width, height), fill=HIGHLIGHT)


def build() -> dict[str, object]:
    DERIVATIVES.mkdir(exist_ok=True)
    manifest: dict[str, object] = {
        "schema": 1,
        "generator": "assets/brand/key-art/build_derivatives.py",
        "originals": {},
        "derivatives": {},
    }

    for theme in THEMES:
        source_path = ORIGINALS / theme.source
        manifest["originals"][theme.name] = {
            "file": source_path.relative_to(ROOT).as_posix(),
            "sha256": sha256(source_path),
        }
        with Image.open(source_path) as source:
            source = source.convert("RGBA")
            for format_name, size in FORMATS.items():
                output = cover(source, size, theme.crop_bias_y)
                add_readability_veil(output, theme.veil_alpha)
                draw_brand(output, format_name)
                output_path = DERIVATIVES / f"{format_name}-{theme.name}.png"
                output.convert("RGB").save(
                    output_path,
                    format="PNG",
                    compress_level=9,
                    optimize=False,
                )
                manifest["derivatives"][f"{format_name}-{theme.name}"] = {
                    "file": output_path.relative_to(ROOT).as_posix(),
                    "dimensions": list(size),
                    "sha256": sha256(output_path),
                    "source_theme": theme.name,
                }

    manifest_path = DERIVATIVES / "manifest.json"
    manifest_path.write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    return manifest


if __name__ == "__main__":
    result = build()
    print(f"Built {len(result['derivatives'])} key-art derivatives in {DERIVATIVES}")
