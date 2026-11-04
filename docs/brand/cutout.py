"""Marchio senza sfondo per l'interfaccia (il riquadro scuro resta solo nelle icone)."""
import os

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
BASE = 18.0  # nero di fondo / riquadro


def cutout(path):
    img = Image.open(path).convert("RGB")
    data = np.asarray(img).astype(np.float32)
    peak = data.max(axis=2)
    alpha = np.clip((peak - BASE) / (255.0 - BASE), 0.0, 1.0)
    alpha[alpha < 0.04] = 0.0
    safe = np.maximum(alpha, 1e-3)[..., None]
    color = np.clip(data / safe, 0.0, 255.0)
    return Image.fromarray(
        np.concatenate([color, (alpha * 255.0)[..., None]], axis=2).astype(np.uint8),
        "RGBA",
    )


for name in ("manager", "launcher"):
    mark = cutout(os.path.join(HERE, "%s.png" % name))
    # toglie i margini trasparenti: il marchio riempie il riquadro
    box = mark.getbbox()
    if box:
        mark = mark.crop(box)
    mark.resize((512, int(round(512 * mark.height / mark.width))), Image.LANCZOS).save(
        os.path.join(REPO, "app", "public", "logo-%s.png" % name), optimize=True
    )
    print("marchio", name, mark.size)
