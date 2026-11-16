from PIL import Image, ImageDraw, ImageFilter
import os

S = 2048
# --- background ---
bg = Image.new("RGBA", (S, S), (0,0,0,0))
grad = Image.new("RGBA", (1, S))
for y in range(S):
    t = y / (S - 1)
    r = int(9 + 11*t); g = int(13 + 14*t); b = int(23 + 21*t)
    grad.putpixel((0, y), (r, g, b, 255))
grad = grad.resize((S, S))
mask = Image.new("L", (S, S), 0)
ImageDraw.Draw(mask).rounded_rectangle([0,0,S-1,S-1], radius=440, fill=255)
bg.paste(grad, (0,0), mask)

glow = Image.new("RGBA", (S, S), (0,0,0,0))
ImageDraw.Draw(glow).ellipse([-300,-500,1300,900], fill=(70,110,255,40))
glow = glow.filter(ImageFilter.GaussianBlur(180))
bg.alpha_composite(Image.composite(glow, Image.new("RGBA",(S,S),(0,0,0,0)), mask))
ImageDraw.Draw(bg).rounded_rectangle([8,8,S-9,S-9], radius=432, outline=(120,150,220,42), width=6)

blocks = [
    (390, 430, 1070, 1620, (86,124,255), (56,86,210), 78),
    (1116, 430, 1570, 900, (146,94,250), (110,64,210), 62),
    (1116, 946, 1570, 1232, (36,202,235), (22,150,190), 54),
    (1116, 1278, 1570, 1620, (244,74,128), (202,45,98), 62),
]

def make_block(b, with_highlight=True):
    x0,y0,x1,y1,c1,c2,rad = b
    w=x1-x0; h=y1-y0
    g = Image.new("RGBA", (w,h))
    gd = ImageDraw.Draw(g)
    for yy in range(h):
        t=yy/max(1,h-1)
        col=tuple(int(c1[i]*(1-t)+c2[i]*t) for i in range(3))+(255,)
        gd.line([(0,yy),(w,yy)], fill=col)
    m = Image.new("L",(w,h),0)
    ImageDraw.Draw(m).rounded_rectangle([0,0,w-1,h-1], radius=rad, fill=255)
    layer=Image.new("RGBA",(S,S),(0,0,0,0))
    layer.paste(g,(x0,y0),m)
    if with_highlight:
        hl=Image.new("RGBA",(S,S),(0,0,0,0))
        ImageDraw.Draw(hl).rounded_rectangle([x0+10,y0+10,x1-10,y0+int(h*0.40)], radius=rad, fill=(255,255,255,22))
        hl=hl.filter(ImageFilter.GaussianBlur(26))
        fullm = Image.new("L",(S,S),0); fullm.paste(m,(x0,y0)); layer.alpha_composite(Image.composite(hl, Image.new("RGBA",(S,S),(0,0,0,0)), fullm))
    return layer

shadow = Image.new("RGBA",(S,S),(0,0,0,0))
sd = ImageDraw.Draw(shadow)
for b in blocks:
    sd.rounded_rectangle([b[0]+14,b[1]+34,b[2]+14,b[3]+34], radius=b[6], fill=(0,0,0,120))
shadow = shadow.filter(ImageFilter.GaussianBlur(50))

final = Image.new("RGBA",(S,S),(0,0,0,0))
final.alpha_composite(bg)
final.alpha_composite(shadow)
for b in blocks:
    final.alpha_composite(make_block(b))

# thin top rim on large block
rd = ImageDraw.Draw(final)
rd.rounded_rectangle([390,430,1070,1620], radius=78, outline=(255,255,255,26), width=4)
rd.rounded_rectangle([1116,430,1570,900], radius=62, outline=(255,255,255,24), width=3)
rd.rounded_rectangle([1116,1278,1570,1620], radius=62, outline=(255,255,255,22), width=3)

# clip final to rounded square
out = Image.new("RGBA",(S,S),(0,0,0,0))
out.paste(final, (0,0), mask)

out = out.resize((1024,1024), Image.LANCZOS)
os.makedirs("assets/icons", exist_ok=True)
out.save("assets/icons/icon_1024.png")
out.resize((512,512), Image.LANCZOS).save("assets/icons/icon_512.png")
out.resize((256,256), Image.LANCZOS).save("assets/icons/icon_256.png")
out.resize((128,128), Image.LANCZOS).save("assets/icons/icon_128.png")
out.resize((64,64), Image.LANCZOS).save("assets/icons/icon_64.png")
out.resize((32,32), Image.LANCZOS).save("assets/icons/icon_32.png")
out.save("assets/icons/icon.ico", sizes=[(16,16),(24,24),(32,32),(48,48),(64,64),(128,128),(256,256)])
print("icon ok")

