# -*- coding: utf-8 -*-
"""测量蓝色圆角方形底版在母版中的边界与圆角半径。"""
import colorsys
from PIL import Image

SRC = r"E:\GitHub\CClean\design\app-icon-master.png"
img = Image.open(SRC).convert("RGB")
w, h = img.size
px = img.load()

def is_blue(r, g, b):
    rf, gf, bf = r/255, g/255, b/255
    hue, sat, val = colorsys.rgb_to_hsv(rf, gf, bf)
    deg = hue*360
    return 185 <= deg <= 270 and sat > 0.18 and val > 0.25

# 顶边行：第一个蓝色像素 x；左边列：第一个蓝色像素 y
top = next(x for x in range(w) if is_blue(*px[x, 0]))
left = next(y for y in range(h) if is_blue(*px[0, y]))
# 底边/右边
bottom = next(y for y in range(h-1, -1, -1) if is_blue(*px[w//2, y]))
right = next(x for x in range(w-1, -1, -1) if is_blue(*px[x, h//2]))
# 顶边中点是否为蓝（判断是否满幅出血）
mid_top_blue = is_blue(*px[w//2, 2])
print("size:", w, h)
print("first blue on top row x =", top)
print("first blue on left col y =", left)
print("last blue on bottom (mid col) y =", bottom)
print("last blue on right (mid row) x =", right)
print("mid-top blue at y=2:", mid_top_blue)
# 角部曲线采样：对若干 y，求顶部区域首个蓝色 x，用于反推半径
for yy in [0, 10, 30, 60, 100, 150, 200, 300, 400]:
    xx = next((x for x in range(w) if is_blue(*px[x, yy])), None)
    print(f"y={yy}: first_blue_x={xx}")
