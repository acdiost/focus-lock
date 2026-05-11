#!/usr/bin/env bash
# gen_icons.sh — 从 SVG 路径生成所有应用图标
# 用法：cd src-tauri/icons && bash gen_icons.sh
# 依赖：macOS 自带的 qlmanage / sips / iconutil / python3（stdlib）
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# ─────────────────────────────────────────────
# 可调参数
# ─────────────────────────────────────────────

# 留白比例（相对内容尺寸，0.10 = 10%）
PADDING_RATIO=0.10

# 输出目录（默认脚本所在目录）
OUT_DIR="$SCRIPT_DIR"

# ─────────────────────────────────────────────
# 生成 SVG
# ─────────────────────────────────────────────

SVG_FILE="/tmp/focus_lock_icon.svg"
cat > "$SVG_FILE" << SVGEOF
<svg version="1.0" xmlns="http://www.w3.org/2000/svg" width="800.000000" height="800.000000" viewBox="0 0 800.000000 800.000000" preserveAspectRatio="xMidYMid meet">
    <g transform="translate(0.000000,800.000000) scale(0.100000,-0.100000)" fill="#000000" stroke="none">
        <path fill="#3CB371" d="M4814 6249 c-183 -19 -429 -102 -587 -196 -198 -119 -319 -310 -342 -543 -6 -58 6 -230 21 -317 6 -29 3 -33 -31 -47 -50 -21 -52 -21 -65 19 -18 56 -108 185 -170 246 -135 132 -366 235 -605 269 -38 5 -123 10 -188 10 l-118 0 6 -37 c45 -264 161 -478 335 -620 138 -112 259 -157 425 -157 126 -1 167 9 369 84 l137 51 102 -36 c252 -90 448 -120 587 -91 353 75 660 473 764 990 28 138 52 304 45 310 -2 2 -58 16 -124 31 -182 41 -382 53 -561 34z m444 -179 l62 -12 -6 -37 c-61 -359 -178 -615 -371 -807 -179 -179 -362 -213 -663 -124 l-73 22 84 85 c98 98 229 207 348 288 85 58 336 185 364 185 9 0 18 4 22 9 5 9 -36 136 -49 148 -17 17 -280 -106 -415 -194 -126 -81 -210 -151 -353 -292 -70 -68 -129 -123 -131 -120 -2 2 -9 33 -15 69 -16 96 -9 274 13 335 29 81 93 173 157 226 109 91 336 187 513 218 33 6 71 13 85 15 46 8 364 -3 428 -14z m-2108 -584 c183 -51 300 -115 399 -221 61 -65 117 -164 108 -189 -7 -19 -92 -36 -175 -36 -184 1 -392 152 -497 362 -60 121 -60 122 18 114 34 -3 101 -17 147 -30z"/>
        <path fill="#FF4D4D" d="M2586 4963 c-205 -201 -341 -404 -447 -668 -104 -262 -154 -591 -129 -870 34 -393 158 -734 374 -1023 248 -332 594 -561 962 -637 183 -38 361 -28 562 29 l114 33 56 -23 c137 -55 339 -75 502 -51 317 48 597 189 839 421 170 165 281 319 386 539 326 683 240 1495 -221 2070 -88 109 -238 261 -253 255 -9 -3 -101 -113 -101 -121 0 -2 41 -44 91 -93 225 -220 375 -483 459 -804 107 -413 56 -879 -139 -1268 -244 -487 -702 -817 -1166 -839 -134 -6 -236 9 -352 53 -85 31 -136 30 -248 -6 -116 -37 -196 -50 -306 -50 -461 0 -953 342 -1203 837 -295 585 -254 1296 106 1826 62 91 180 229 246 287 l60 53 -46 63 c-26 35 -52 63 -57 64 -6 0 -46 -34 -89 -77z"/>
        <path fill="#0F1A20" d="M3950 4522 l-30 -30 0 -446 0 -446 -514 0 -515 0 -20 -26 c-28 -36 -27 -85 2 -112 l23 -22 576 0 577 0 20 26 c21 26 21 36 21 525 l0 498 -22 25 c-13 15 -37 29 -55 32 -27 5 -37 1 -63 -24z"/>
        <path fill="#F8F4E9" d="M4906 2755 c-67 -90 -128 -152 -190 -195 -25 -17 -46 -34 -46 -37 0 -9 71 -122 81 -128 14 -9 97 52 174 130 64 63 175 207 175 225 0 8 -117 80 -130 80 -4 0 -33 -34 -64 -75z"/>
    </g>
</svg>
SVGEOF

echo "✓ SVG 已生成：$SVG_FILE"

# ─────────────────────────────────────────────
# 渲染 SVG → 1024px PNG（qlmanage 会加白底）
# ─────────────────────────────────────────────

qlmanage -t -s 1024 -o /tmp/ "$SVG_FILE" 2>/dev/null
RAW_PNG="/tmp/focus_lock_icon.svg.png"
echo "✓ 渲染完成：$RAW_PNG"

# ─────────────────────────────────────────────
# Python：去白底 + 裁内容边界 + 加留白 → 方形 PNG
# ─────────────────────────────────────────────

PROCESSED_PNG="/tmp/focus_lock_processed.png"
python3 - "$RAW_PNG" "$PROCESSED_PNG" "$PADDING_RATIO" << 'PYEOF'
import sys, struct, zlib

src, dst, pad_ratio = sys.argv[1], sys.argv[2], float(sys.argv[3])

def read_png(path):
    with open(path, 'rb') as f: data = f.read()
    w = struct.unpack('>I', data[16:20])[0]
    h = struct.unpack('>I', data[20:24])[0]
    raw = b''
    i = 8
    while i < len(data):
        n = struct.unpack('>I', data[i:i+4])[0]
        if data[i+4:i+8] == b'IDAT': raw += data[i+8:i+8+n]
        i += 12 + n
    raw = zlib.decompress(raw)
    stride = w * 4
    rows, pos, prev = [], 0, bytes(stride)
    for _ in range(h):
        ft = raw[pos]; pos += 1
        row = bytearray(raw[pos:pos+stride]); pos += stride
        if ft == 1:
            for x in range(4, stride): row[x] = (row[x]+row[x-4])&0xff
        elif ft == 2:
            row = bytearray((a+b)&0xff for a,b in zip(row,prev))
        elif ft == 3:
            for x in range(stride):
                a = row[x-4] if x>=4 else 0
                row[x] = (row[x]+(a+prev[x])//2)&0xff
        elif ft == 4:
            for x in range(stride):
                a=row[x-4] if x>=4 else 0; b=prev[x]; c=prev[x-4] if x>=4 else 0
                pa,pb,pc=abs(b-c),abs(a-c),abs(a+b-2*c)
                pr=a if pa<=pb and pa<=pc else (b if pb<=pc else c)
                row[x]=(row[x]+pr)&0xff
        rows.append(bytes(row)); prev=bytes(row)
    return w, h, rows

def write_png(path, w, h, rows):
    def chunk(name, d):
        c=name+d; return struct.pack('>I',len(d))+c+struct.pack('>I',zlib.crc32(c)&0xffffffff)
    raw = b''.join(b'\x00'+r for r in rows)
    ihdr = struct.pack('>IIBBBBB', w, h, 8, 6, 0, 0, 0)
    with open(path, 'wb') as f:
        f.write(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',ihdr)+chunk(b'IDAT',zlib.compress(raw,9))+chunk(b'IEND',b''))

w, h, rows = read_png(src)

# 去白底
transparent = []

def is_keep_color(r, g, b, tol=6):
    return (
        abs(r - 248) <= tol and
        abs(g - 244) <= tol and
        abs(b - 233) <= tol
    )

for row in rows:
    r2 = bytearray(row)
    for x in range(w):
        rr, gg, bb, aa = (
            r2[x*4],
            r2[x*4+1],
            r2[x*4+2],
            r2[x*4+3]
        )
        if (
            aa > 200 and
            rr > 230 and gg > 230 and bb > 230 and
            not is_keep_color(rr, gg, bb)
        ):
            r2[x*4+3] = 0
    transparent.append(bytes(r2))

# 找内容边界
mx,my,Mx,My = w,h,0,0
for y,row in enumerate(transparent):
    for x in range(w):
        if row[x*4+3] > 20:
            mx=min(mx,x); my=min(my,y); Mx=max(Mx,x); My=max(My,y)

cw,ch = Mx-mx+1, My-my+1
pad = int(max(cw,ch)*pad_ratio)
sz = max(cw,ch)+pad*2
ox=(sz-cw)//2; oy=(sz-ch)//2
canvas=[bytearray(sz*4) for _ in range(sz)]
for y in range(ch):
    canvas[oy+y][ox*4:(ox+cw)*4]=transparent[my+y][mx*4:(mx+cw)*4]

write_png(dst, sz, sz, [bytes(r) for r in canvas])
print(f"  content {cw}×{ch} → canvas {sz}×{sz}")
PYEOF

echo "✓ 处理完成：$PROCESSED_PNG"

# ─────────────────────────────────────────────
# 生成各尺寸 PNG
# ─────────────────────────────────────────────

sips -z 1024 1024 "$PROCESSED_PNG" --out "$OUT_DIR/icon.png"      2>/dev/null
sips -z 256  256  "$PROCESSED_PNG" --out "$OUT_DIR/128x128@2x.png" 2>/dev/null
sips -z 128  128  "$PROCESSED_PNG" --out "$OUT_DIR/128x128.png"    2>/dev/null
sips -z 32   32   "$PROCESSED_PNG" --out "$OUT_DIR/32x32.png"      2>/dev/null
echo "✓ PNG 尺寸已生成"

# 托盘图标（从 128x128 裁剪，保持黑色 template 风格）
python3 - "$OUT_DIR/128x128.png" "$OUT_DIR/tray.png" << 'TRAYEOF'
import sys, struct, zlib

def read_png(path):
    with open(path, 'rb') as f: data = f.read()
    w = struct.unpack('>I', data[16:20])[0]
    h = struct.unpack('>I', data[20:24])[0]
    raw = b''
    i = 8
    while i < len(data):
        n = struct.unpack('>I', data[i:i+4])[0]
        if data[i+4:i+8] == b'IDAT': raw += data[i+8:i+8+n]
        i += 12 + n
    raw = zlib.decompress(raw)
    stride = w * 4
    rows, pos, prev = [], 0, bytes(stride)
    for _ in range(h):
        ft = raw[pos]; pos += 1
        row = bytearray(raw[pos:pos+stride]); pos += stride
        if ft == 1:
            for x in range(4, stride): row[x] = (row[x]+row[x-4])&0xff
        elif ft == 2:
            row = bytearray((a+b)&0xff for a,b in zip(row,prev))
        elif ft == 3:
            for x in range(stride):
                a = row[x-4] if x>=4 else 0
                row[x] = (row[x]+(a+prev[x])//2)&0xff
        elif ft == 4:
            for x in range(stride):
                a=row[x-4] if x>=4 else 0; b=prev[x]; c=prev[x-4] if x>=4 else 0
                pa,pb,pc=abs(b-c),abs(a-c),abs(a+b-2*c)
                pr=a if pa<=pb and pa<=pc else (b if pb<=pc else c)
                row[x]=(row[x]+pr)&0xff
        rows.append(bytes(row)); prev=bytes(row)
    return w, h, rows

def write_png(path, w, h, rows):
    def chunk(name, d):
        c=name+d; return struct.pack('>I',len(d))+c+struct.pack('>I',zlib.crc32(c)&0xffffffff)
    raw = b''.join(b'\x00'+r for r in rows)
    ihdr = struct.pack('>IIBBBBB', w, h, 8, 6, 0, 0, 0)
    with open(path, 'wb') as f:
        f.write(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',ihdr)+chunk(b'IDAT',zlib.compress(raw,9))+chunk(b'IEND',b''))

src, dst = sys.argv[1], sys.argv[2]
w, h, rows = read_png(src)

# 将彩色像素转为黑色（保留 alpha），用于 macOS template 模式
tray = []
for row in rows:
    r2 = bytearray(row)
    for x in range(w):
        a = r2[x*4+3]
        if a > 20:
            r2[x*4]=r2[x*4+1]=r2[x*4+2]=0
    tray.append(bytes(r2))

# 缩到 32×32
step = w // 32
canvas = []
for oy in range(32):
    row = bytearray(32*4)
    for ox in range(32):
        # 简单区域平均
        ra=ga=ba=aa=cnt=0
        for dy in range(step):
            for dx in range(step):
                sy=oy*step+dy; sx=ox*step+dx
                if sy<h and sx<w:
                    p=tray[sy]
                    ra+=p[sx*4]; ga+=p[sx*4+1]; ba+=p[sx*4+2]; aa+=p[sx*4+3]; cnt+=1
        if cnt:
            row[ox*4]=ra//cnt; row[ox*4+1]=ga//cnt; row[ox*4+2]=ba//cnt; row[ox*4+3]=aa//cnt
    canvas.append(bytes(row))

write_png(dst, 32, 32, canvas)
TRAYEOF
echo "✓ tray.png 已生成（黑色 template 模式）"

# ─────────────────────────────────────────────
# 重建 icon.icns
# ─────────────────────────────────────────────

ICONSET="/tmp/FocusLock_gen.iconset"
mkdir -p "$ICONSET"
sips -z 16   16   "$PROCESSED_PNG" --out "$ICONSET/icon_16x16.png"      2>/dev/null
sips -z 32   32   "$PROCESSED_PNG" --out "$ICONSET/icon_16x16@2x.png"   2>/dev/null
sips -z 32   32   "$PROCESSED_PNG" --out "$ICONSET/icon_32x32.png"      2>/dev/null
sips -z 64   64   "$PROCESSED_PNG" --out "$ICONSET/icon_32x32@2x.png"   2>/dev/null
sips -z 128  128  "$PROCESSED_PNG" --out "$ICONSET/icon_128x128.png"    2>/dev/null
sips -z 256  256  "$PROCESSED_PNG" --out "$ICONSET/icon_128x128@2x.png" 2>/dev/null
sips -z 256  256  "$PROCESSED_PNG" --out "$ICONSET/icon_256x256.png"    2>/dev/null
sips -z 512  512  "$PROCESSED_PNG" --out "$ICONSET/icon_256x256@2x.png" 2>/dev/null
sips -z 512  512  "$PROCESSED_PNG" --out "$ICONSET/icon_512x512.png"    2>/dev/null
sips -z 1024 1024 "$PROCESSED_PNG" --out "$ICONSET/icon_512x512@2x.png" 2>/dev/null
iconutil -c icns "$ICONSET" -o "$OUT_DIR/icon.icns"
echo "✓ icon.icns 已重建"

# ─────────────────────────────────────────────
# 重建 icon.ico
# ─────────────────────────────────────────────

sips -s format ico "$OUT_DIR/128x128@2x.png" --out "$OUT_DIR/icon.ico" 2>/dev/null
echo "✓ icon.ico 已重建"

echo ""
echo "全部完成。输出目录：$OUT_DIR"
