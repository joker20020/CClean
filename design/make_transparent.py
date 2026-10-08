# -*- coding: utf-8 -*-
"""将 app-icon-master.png 的蓝色底版抠除，保留白色容量环与绿色扫帚，输出透明背景 PNG。"""
import colorsys
from PIL import Image, ImageFilter

SRC = r"E:\GitHub\CClean\design\app-icon-master.png"
DST = r"E:\GitHub\CClean\design\app-icon-master-transparent.png"
PREVIEW = r"E:\GitHub\CClean\design\app-icon-transparent-preview.png"

img = Image.open(SRC).convert("RGBA")
w, h = img.size
px = img.load()

# 步骤 1：按 HSV 判定每个像素的保留度
#   白色/浅色图形（容量环）：低饱和度、高明度 -> 保留
#   绿色图形（扫帚与进度弧）：色相约 130-185 -> 保留
#   蓝色底版（含深蓝边角与阴影）：色相约 190-260 且有一定饱和度 -> 透明
alpha = Image.new("L", (w, h), 0)
ap = alpha.load()

for y in range(h):
    for x in range(w):
        r, g, b, _ = px[x, y]
        rf, gf, bf = r / 255.0, g / 255.0, b / 255.0
        hue, sat, val = colorsys.rgb_to_hsv(rf, gf, bf)
        deg = hue * 360.0

        if val < 0.18:
            keep = 0.0  # 近黑杂边
        elif sat < 0.20 and val > 0.55:
            keep = 1.0  # 白色图形
        elif 125.0 <= deg <= 185.0 and sat > 0.22 and val > 0.30:
            keep = 1.0  # 绿色图形
        elif 185.0 < deg < 200.0 and sat > 0.20:
            # 青绿过渡带：按色相线性收边，避免绿弧边缘锯齿
            keep = max(0.0, (200.0 - deg) / 15.0)
        elif 200.0 <= deg <= 265.0 and sat > 0.16:
            keep = 0.0  # 蓝色底版
        elif sat < 0.28 and val > 0.45:
            # 蓝白抗锯齿过渡像素：按饱和度半保留
            keep = max(0.0, 1.0 - (sat - 0.20) / 0.08) if sat > 0.20 else 1.0
        else:
            keep = 0.0
        ap[x, y] = max(0, min(255, int(keep * 255)))

# 步骤 2：轻微羽化 alpha 边缘（约 1px），消除抠图锯齿
alpha = alpha.filter(ImageFilter.GaussianBlur(radius=1.2))

out = Image.new("RGBA", (w, h), (0, 0, 0, 0))
out.putalpha(alpha)
# 用原 RGB + 新 alpha 合成
rgb = img.convert("RGB")
out = Image.merge("RGBA", (*rgb.split(), alpha))
out.save(DST)

# 步骤 3：生成棋盘格预览图，便于肉眼检查透明效果
checker = Image.new("RGBA", (w, h))
cp = checker.load()
cell = 64
for y in range(h):
    for x in range(w):
        if ((x // cell) + (y // cell)) % 2 == 0:
            cp[x, y] = (204, 204, 204, 255)
        else:
            cp[x, y] = (255, 255, 255, 255)
checker.alpha_composite(out)
# 预览缩小一半便于查看
checker = checker.resize((w // 2, h // 2), Image.LANCZOS)
checker.convert("RGB").save(PREVIEW)

print("saved:", DST)
print("preview:", PREVIEW)
