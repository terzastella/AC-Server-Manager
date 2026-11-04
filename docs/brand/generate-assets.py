"""Genera icone e badge dai due loghi master in docs/brand.

Uso: python docs/brand/generate-assets.py
Sorgenti: docs/brand/manager.png e docs/brand/launcher.png (1024x1024, RGBA).
"""
import os
import struct

from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
PUBLIC = os.path.join(REPO, "app", "public")
ICONS = os.path.join(REPO, "app", "src-tauri", "icons")


def rounded(size, src):
    return src.resize((size, size), Image.LANCZOS)


def save_ico(src, path, sizes=(16, 20, 24, 32, 40, 48, 64, 128, 256)):
    frames = []
    for size in sizes:
        img = rounded(size, src)
        tmp = os.path.join(HERE, "_tmp.png")
        img.save(tmp, format="PNG")
        with open(tmp, "rb") as handle:
            frames.append((size, handle.read()))
    os.remove(tmp)

    with open(path, "wb") as out:
        out.write(struct.pack("<HHH", 0, 1, len(frames)))
        offset = 6 + 16 * len(frames)
        for size, data in frames:
            dim = 0 if size >= 256 else size
            out.write(struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(data), offset))
            offset += len(data)
        for _, data in frames:
            out.write(data)


def main():
    manager = Image.open(os.path.join(HERE, "manager.png")).convert("RGBA")
    launcher = Image.open(os.path.join(HERE, "launcher.png")).convert("RGBA")

    # badge dell'interfaccia (256 px, mostrati a 34-36 px)
    rounded(256, manager).save(os.path.join(PUBLIC, "logo-manager.png"), optimize=True)
    rounded(256, launcher).save(os.path.join(PUBLIC, "logo-launcher.png"), optimize=True)

    # icone degli eseguibili e di Tauri
    save_ico(manager, os.path.join(HERE, "logo-manager.ico"))
    save_ico(launcher, os.path.join(HERE, "logo-launcher.ico"))

    for name, size in (("32x32.png", 32), ("64x64.png", 64), ("128x128.png", 128),
                       ("128x128@2x.png", 256), ("icon.png", 512)):
        rounded(size, manager).save(os.path.join(ICONS, name), optimize=True)
    rounded(256, launcher).save(os.path.join(ICONS, "launcher.png"), optimize=True)

    print("asset rigenerati")


if __name__ == "__main__":
    main()
