from __future__ import annotations

from collections import deque
from functools import lru_cache
from glob import glob
from io import BytesIO

from PIL import Image, ImageDraw, ImageFont

from stock_db.sources.stooq.exceptions import StooqCaptchaError

_ALPHABET = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"
_CANVAS_SIZE = 24
_COMPONENT_MIN_PIXELS = 100
_FONT_PATTERNS = (
    "/nix/store/*/share/fonts/truetype/DejaVuSansCondensed-Bold.ttf",
    "/nix/store/*/share/fonts/truetype/DejaVuSans-BoldOblique.ttf",
    "/nix/store/*/share/fonts/truetype/FreeSansBoldOblique.ttf",
    "/nix/store/*/share/fonts/truetype/LiberationSans-BoldItalic.ttf",
)
_FONT_SIZES = (34, 38, 42)
_ROTATION_ANGLES = tuple(range(-35, 36, 5))
_SCORE_THRESHOLD = 0.45


def _red_mask(image: Image.Image) -> list[list[int]]:
    rgb_image = image.convert("RGB")
    width, height = rgb_image.size
    mask = [[0] * width for _ in range(height)]
    for y in range(height):
        for x in range(width):
            red, green, blue = rgb_image.getpixel((x, y))
            if red > 180 and green < 170 and blue < 170 and red - max(green, blue) > 30:
                mask[y][x] = 1
    return mask


def _remove_grid(mask: list[list[int]]) -> list[list[int]]:
    height = len(mask)
    width = len(mask[0])
    grid_rows = {y for y in range(height) if sum(mask[y]) >= int(width * 0.8)}
    grid_cols = {
        x for x in range(width)
        if sum(mask[y][x] for y in range(height)) >= int(height * 0.8)
    }

    cleaned = [[0] * width for _ in range(height)]
    for y in range(height):
        for x in range(width):
            if mask[y][x] == 0:
                continue
            if y in grid_rows or x in grid_cols:
                neighbors = 0
                for yy in range(max(0, y - 1), min(height, y + 2)):
                    for xx in range(max(0, x - 1), min(width, x + 2)):
                        neighbors += mask[yy][xx]
                if neighbors <= 4:
                    continue
            cleaned[y][x] = 1

    for y in grid_rows:
        if y == 0 or y == height - 1:
            continue
        for x in range(width):
            if cleaned[y][x] == 1:
                continue
            if cleaned[y - 1][x] and cleaned[y + 1][x]:
                cleaned[y][x] = 1
            elif (
                x > 0
                and x < width - 1
                and (
                    (cleaned[y - 1][x - 1] and cleaned[y + 1][x + 1])
                    or (cleaned[y - 1][x + 1] and cleaned[y + 1][x - 1])
                )
            ):
                cleaned[y][x] = 1

    for x in grid_cols:
        if x == 0 or x == width - 1:
            continue
        for y in range(height):
            if cleaned[y][x] == 1:
                continue
            if cleaned[y][x - 1] and cleaned[y][x + 1]:
                cleaned[y][x] = 1
            elif (
                y > 0
                and y < height - 1
                and (
                    (cleaned[y - 1][x - 1] and cleaned[y + 1][x + 1])
                    or (cleaned[y - 1][x + 1] and cleaned[y + 1][x - 1])
                )
            ):
                cleaned[y][x] = 1

    return cleaned


def _connected_components(mask: list[list[int]]) -> list[tuple[int, int, int, int]]:
    height = len(mask)
    width = len(mask[0])
    seen = [[False] * width for _ in range(height)]
    components: list[tuple[int, int, int, int, int]] = []
    for y in range(height):
        for x in range(width):
            if mask[y][x] == 0 or seen[y][x]:
                continue
            queue: deque[tuple[int, int]] = deque([(x, y)])
            seen[y][x] = True
            pixels = 0
            min_x = max_x = x
            min_y = max_y = y
            while queue:
                current_x, current_y = queue.popleft()
                pixels += 1
                min_x = min(min_x, current_x)
                max_x = max(max_x, current_x)
                min_y = min(min_y, current_y)
                max_y = max(max_y, current_y)
                for next_y in range(max(0, current_y - 1), min(height, current_y + 2)):
                    for next_x in range(max(0, current_x - 1), min(width, current_x + 2)):
                        if mask[next_y][next_x] == 0 or seen[next_y][next_x]:
                            continue
                        seen[next_y][next_x] = True
                        queue.append((next_x, next_y))
            if pixels >= _COMPONENT_MIN_PIXELS:
                components.append((pixels, min_x, min_y, max_x, max_y))
    components.sort(key=lambda component: component[1])
    return [(min_x, min_y, max_x, max_y) for _, min_x, min_y, max_x, max_y in components]


def _normalize_image(image: Image.Image) -> tuple[int, ...]:
    grayscale = image.convert("L").point(lambda value: 255 if value > 0 else 0)
    bbox = grayscale.getbbox()
    if bbox is None:
        return (0,) * (_CANVAS_SIZE * _CANVAS_SIZE)

    cropped = grayscale.crop(bbox)
    width, height = cropped.size
    scale = min((_CANVAS_SIZE - 4) / width, (_CANVAS_SIZE - 4) / height)
    resized = cropped.resize(
        (max(1, round(width * scale)), max(1, round(height * scale))),
        Image.Resampling.NEAREST,
    )
    canvas = Image.new("1", (_CANVAS_SIZE, _CANVAS_SIZE), 0)
    offset_x = (_CANVAS_SIZE - resized.width) // 2
    offset_y = (_CANVAS_SIZE - resized.height) // 2
    canvas.paste(resized.convert("1"), (offset_x, offset_y))
    return tuple(1 if pixel else 0 for pixel in canvas.getdata())


@lru_cache(maxsize=1)
def _font_paths() -> tuple[str, ...]:
    paths: list[str] = []
    for pattern in _FONT_PATTERNS:
        for path in sorted(glob(pattern)):
            if path not in paths:
                paths.append(path)
    if not paths:
        raise StooqCaptchaError("No OCR fonts found for Stooq CAPTCHA solver")
    return tuple(paths)


@lru_cache(maxsize=None)
def _templates_for_char(char: str) -> tuple[tuple[int, ...], ...]:
    templates: list[tuple[int, ...]] = []
    for font_path in _font_paths():
        for font_size in _FONT_SIZES:
            font = ImageFont.truetype(font_path, font_size)
            for angle in _ROTATION_ANGLES:
                image = Image.new("L", (80, 80), 0)
                draw = ImageDraw.Draw(image)
                bbox = draw.textbbox((0, 0), char, font=font)
                offset_x = (80 - (bbox[2] - bbox[0])) // 2 - bbox[0]
                offset_y = (80 - (bbox[3] - bbox[1])) // 2 - bbox[1]
                draw.text((offset_x, offset_y), char, fill=255, font=font)
                rotated = image.rotate(angle, expand=True, fillcolor=0)
                templates.append(_normalize_image(rotated))
    return tuple(templates)


def _score(component: tuple[int, ...], template: tuple[int, ...]) -> float:
    intersection = 0
    union = 0
    for left, right in zip(component, template, strict=True):
        intersection += left & right
        union += left | right
    if union == 0:
        return 0.0
    return intersection / union


def _classify_component(component: tuple[int, ...]) -> str:
    best_char = ""
    best_score = -1.0
    for char in _ALPHABET:
        char_score = max(_score(component, template) for template in _templates_for_char(char))
        if char_score > best_score:
            best_char = char
            best_score = char_score
    if best_score < _SCORE_THRESHOLD:
        raise StooqCaptchaError(f"Stooq CAPTCHA OCR score too low: {best_score:.3f}")
    return best_char


def solve_stooq_captcha(image_bytes: bytes) -> str:
    image = Image.open(BytesIO(image_bytes))
    cleaned_mask = _remove_grid(_red_mask(image))
    components = _connected_components(cleaned_mask)
    if len(components) != 4:
        raise StooqCaptchaError(
            f"Expected 4 Stooq CAPTCHA components, got {len(components)}"
        )

    mask_image = Image.new("1", image.size, 0)
    for y, row in enumerate(cleaned_mask):
        for x, value in enumerate(row):
            if value:
                mask_image.putpixel((x, y), 1)

    characters: list[str] = []
    for min_x, min_y, max_x, max_y in components:
        crop = mask_image.crop((min_x, min_y, max_x + 1, max_y + 1))
        characters.append(_classify_component(_normalize_image(crop)))
    return "".join(characters)
