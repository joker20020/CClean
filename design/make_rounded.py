# -*- coding: utf-8 -*-
"""保留蓝色圆角方形底版，仅把圆角之外的浅色边缘抠为透明。"""
from PIL import Image, ImageDraw

SRC = r"E:\GitHub\CClean\design\app-icon-master.png"
DST = r"E:\GitHub\CClean\design\app-icon-master-rounded.png"
PREVIEW = r"E:\GitHub\CClean\design\app-icon-rounded-preview.png"

R = 276          # 实测圆角半径
SS = 4           # 超采样倍数，用于圆角抗锯齿

img = Image.open(SRC).convert("RGBA")
w, h = img.size

# 在 4x 画布上绘制圆角矩形蒙版
mask = Image.new("L", (w*SS, h*SS), 0)
d = ImageDraw.Draw(mask)
d.rounded_rectangle([0, 0, w*SS-1, h*SS-1], radius=R*SS, fill=255)
mask = mask.resize((w, h), Image.LANCZOS)

out = img.copy()
out.putalpha(mask)
out.save(DST)

# 棋盘格预览
checker = Image.new("RGBA", (w, h))
cp = checker.load()
cell = 64
for y in range(h):
    for x in range(w):
        cp[x, y] = (204, 204, 204, 255) if ((x//cell)+(y//cell)) % 2 == 0 else (255, 255, 255, 255)
checker.alpha_composite(out)
checker.resize((w//2, h//2), Image.LANCZOS).convert("RGB").save(PREVIEW)
print("saved:", DST)
