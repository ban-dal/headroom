#!/usr/bin/env python3
"""Generate a monochrome macOS template tray icon (an "H" mark) with stdlib only.

Template icons are black pixels + alpha; macOS recolors them for light/dark
menu bars. Outputs 32px and 64px@2x PNGs under src-tauri/icons/tray/.
"""
import os
import struct
import zlib

OUT = os.path.join(os.path.dirname(__file__), "..", "src-tauri", "icons", "tray")


def render(size: int) -> bytes:
    s = size
    px = bytearray(s * s * 4)  # RGBA, zero-init = transparent

    # "H" geometry scaled to canvas (proportions tuned at 32px).
    pad = round(s * 0.19)          # top/bottom margin
    bar_w = max(1, round(s * 0.14))
    left_x0 = round(s * 0.22)
    right_x1 = s - left_x0
    cross_y0 = round(s * 0.43)
    cross_y1 = round(s * 0.57)

    def fill(x0, y0, x1, y1):
        for y in range(y0, y1):
            for x in range(x0, x1):
                i = (y * s + x) * 4
                px[i] = 0          # black
                px[i + 1] = 0
                px[i + 2] = 0
                px[i + 3] = 255    # opaque

    fill(left_x0, pad, left_x0 + bar_w, s - pad)            # left stem
    fill(right_x1 - bar_w, pad, right_x1, s - pad)          # right stem
    fill(left_x0, cross_y0, right_x1, cross_y1)             # crossbar

    return encode_png(s, s, bytes(px))


def encode_png(w: int, h: int, rgba: bytes) -> bytes:
    def chunk(tag: bytes, data: bytes) -> bytes:
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    raw = bytearray()
    for y in range(h):
        raw.append(0)  # no filter
        raw.extend(rgba[y * w * 4 : (y + 1) * w * 4])

    ihdr = struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)  # 8-bit RGBA
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + chunk(b"IEND", b"")
    )


def main():
    os.makedirs(OUT, exist_ok=True)
    for size, name in [(32, "tray.png"), (64, "tray@2x.png")]:
        path = os.path.join(OUT, name)
        with open(path, "wb") as f:
            f.write(render(size))
        print("wrote", os.path.normpath(path))


if __name__ == "__main__":
    main()
