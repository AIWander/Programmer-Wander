#!/usr/bin/env python3
"""Generate Programmer-Wander icon set + installer wizard art.

Outputs (all into assets/):
  programmer.ico   multi-size Windows icon (16..256)
  icon_256.png     PNG for docs / onboarding page
  icon_512.png     large PNG master
  wizard_large.bmp / wizard_large_2x.bmp   Inno WizardImageFile (164x314 / 328x628)
  wizard_small.bmp / wizard_small_2x.bmp   Inno WizardSmallImageFile (55x58 / 110x116)

Design: dark navy-to-indigo rounded tile, cyan-to-violet terminal chevron,
amber cursor block (Rust nod), soft glow. No text on the icon itself.
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

HERE = Path(__file__).resolve().parent
S = 1024  # master canvas

NAVY = (11, 16, 32)
INDIGO = (30, 27, 75)
CYAN = (34, 211, 238)
VIOLET = (167, 139, 250)
AMBER = (245, 158, 11)


def vgrad(size, top, bottom):
    """Vertical gradient image."""
    strip = Image.new("RGB", (1, size[1]))
    for y in range(size[1]):
        t = y / max(size[1] - 1, 1)
        strip.putpixel((0, y), tuple(int(a + (b - a) * t) for a, b in zip(top, bottom)))
    return strip.resize(size)


def hgrad(size, left, right):
    """Horizontal gradient image."""
    strip = Image.new("RGB", (size[0], 1))
    for x in range(size[0]):
        t = x / max(size[0] - 1, 1)
        strip.putpixel((x, 0), tuple(int(a + (b - a) * t) for a, b in zip(left, right)))
    return strip.resize(size)


def rounded_mask(size, radius):
    m = Image.new("L", size, 0)
    ImageDraw.Draw(m).rounded_rectangle([0, 0, size[0] - 1, size[1] - 1], radius=radius, fill=255)
    return m


def make_tile():
    tile = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    bg = vgrad((S, S), NAVY, INDIGO).convert("RGBA")
    mask = rounded_mask((S, S), int(S * 0.22))
    tile.paste(bg, (0, 0), mask)

    # inner border glow
    ring = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(ring).rounded_rectangle(
        [int(S * 0.015), int(S * 0.015), S - int(S * 0.015), S - int(S * 0.015)],
        radius=int(S * 0.21), outline=CYAN + (90,), width=int(S * 0.008))
    tile.alpha_composite(ring.filter(ImageFilter.GaussianBlur(int(S * 0.004))))

    # chevron ">" as thick polyline polygon
    w = int(S * 0.13)          # stroke thickness
    x0, x1 = int(S * 0.24), int(S * 0.52)
    ytop, ymid, ybot = int(S * 0.26), int(S * 0.50), int(S * 0.74)
    chev = Image.new("L", (S, S), 0)
    d = ImageDraw.Draw(chev)
    d.line([(x0, ytop), (x1, ymid), (x0, ybot)], fill=255, width=w, joint="curve")
    # round the line caps
    r = w // 2
    for cx, cy in [(x0, ytop), (x0, ybot)]:
        d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=255)

    grad = hgrad((S, S), CYAN, VIOLET).convert("RGBA")
    glow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    glow.paste(grad, (0, 0), chev)
    tile.alpha_composite(glow.filter(ImageFilter.GaussianBlur(int(S * 0.03))))
    body = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    body.paste(grad, (0, 0), chev)
    tile.alpha_composite(body)

    # amber cursor block
    cw, ch = int(S * 0.22), int(S * 0.085)
    cx, cy = int(S * 0.56), int(S * 0.74) - ch
    cur = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(cur).rounded_rectangle([cx, cy, cx + cw, cy + ch], radius=ch // 3, fill=AMBER + (255,))
    tile.alpha_composite(cur.filter(ImageFilter.GaussianBlur(int(S * 0.02))))
    tile.alpha_composite(cur)
    return tile


def wizard_banner(size):
    img = vgrad(size, NAVY, INDIGO)
    tile = make_tile()
    glyph = tile.resize((int(size[0] * 0.72),) * 2, Image.LANCZOS)
    img = img.convert("RGBA")
    img.alpha_composite(glyph, (int(size[0] * 0.14), int(size[1] * 0.10)))
    try:
        font = ImageFont.truetype("segoeuib.ttf", max(10, int(size[0] * 0.075)))
        d = ImageDraw.Draw(img)
        text = "PROGRAMMER\nWANDER"
        d.multiline_text((size[0] // 2, int(size[1] * 0.80)), text, font=font,
                         fill=(226, 232, 240, 255), anchor="ma", align="center",
                         spacing=int(size[0] * 0.02))
    except OSError:
        pass
    return img.convert("RGB")


def wizard_small(size):
    tile = make_tile().resize((min(size),) * 2, Image.LANCZOS)
    img = vgrad(size, NAVY, INDIGO).convert("RGBA")
    img.alpha_composite(tile, ((size[0] - tile.width) // 2, (size[1] - tile.height) // 2))
    return img.convert("RGB")


def main():
    tile = make_tile()
    tile.resize((512, 512), Image.LANCZOS).save(HERE / "icon_512.png")
    icon256 = tile.resize((256, 256), Image.LANCZOS)
    icon256.save(HERE / "icon_256.png")
    icon256.save(HERE / "programmer.ico", format="ICO",
                 sizes=[(256, 256), (128, 128), (64, 64), (48, 48), (32, 32), (24, 24), (16, 16)])
    wizard_banner((164, 314)).save(HERE / "wizard_large.bmp")
    wizard_banner((328, 628)).save(HERE / "wizard_large_2x.bmp")
    wizard_small((55, 58)).save(HERE / "wizard_small.bmp")
    wizard_small((110, 116)).save(HERE / "wizard_small_2x.bmp")
    print("wrote", sorted(p.name for p in HERE.iterdir() if p.suffix in {".ico", ".png", ".bmp"}))


if __name__ == "__main__":
    main()
