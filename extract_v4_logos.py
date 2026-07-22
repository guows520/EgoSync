import glob
import os
from PIL import Image

def process_v4_logos():
    # 找到最新的 V4 绿幕截图
    pattern = r'C:\Users\Admin\.gemini\antigravity\brain\28c43195-75a0-432a-91b2-38e02651be47\logo_render_*.png'
    files = glob.glob(pattern)
    if not files:
        print("未找到 V4 版本绿幕截图", flush=True)
        return
        
    file_path = files[-1]
    print(f"载入 V4 截图: {file_path}", flush=True)
    
    img = Image.open(file_path).convert("RGBA")
    width, height = img.size
    print(f"图像尺寸: {width}x{height}", flush=True)
    
    # 极速像素数组指针
    img_pixels = img.load()
    
    left_non_greens = []
    right_non_greens = []
    
    scan_min_x = int(width * 0.05)
    scan_max_x = int(width * 0.95)
    scan_min_y = int(height * 0.15)
    scan_max_y = int(height * 0.85)
    
    mid_x = width // 2
    
    # 检测非绿背景色点以精确定位 Logo 外沿
    for y in range(scan_min_y, scan_max_y, 2):
        for x in range(scan_min_x, scan_max_x, 2):
            r, g, b, _ = img_pixels[x, y]
            is_green_screen = (g - r > 100) and (g - b > 80)
            
            if not is_green_screen:
                if x < mid_x:
                    left_non_greens.append((x, y))
                else:
                    right_non_greens.append((x, y))
                    
    # 处理左半边：V4 晶体透明版
    if left_non_greens:
        xs = [p[0] for p in left_non_greens]
        ys = [p[1] for p in left_non_greens]
        min_x, max_x = min(xs), max(xs)
        min_y, max_y = min(ys), max(ys)
        
        # padding
        padding = 8
        left_b = max(0, min_x - padding)
        right_b = min(mid_x, max_x + padding)
        top_b = max(scan_min_y, min_y - padding)
        bottom_b = min(scan_max_y, max_y + padding)
        
        side = max(right_b - left_b, bottom_b - top_b)
        cx = (left_b + right_b) // 2
        cy = (top_b + bottom_b) // 2
        left_b, right_b = cx - side // 2, cx + side // 2
        top_b, bottom_b = cy - side // 2, cy + side // 2
        
        print(f"检测到左侧 V4 透明版大小: {side}x{side}", flush=True)
        crop_and_key_fast(img_pixels, left_b, top_b, right_b, bottom_b, "logo_v4_transparent")
    else:
        print("未在左半边检测到 V4 Logo，绿幕截图可能为纯绿白板", flush=True)
        
    # 处理右半边：V4 晶体有底色版
    if right_non_greens:
        xs = [p[0] for p in right_non_greens]
        ys = [p[1] for p in right_non_greens]
        min_x, max_x = min(xs), max(xs)
        min_y, max_y = min(ys), max(ys)
        
        # padding
        padding = 8
        left_b = max(mid_x, min_x - padding)
        right_b = min(width, max_x + padding)
        top_b = max(scan_min_y, min_y - padding)
        bottom_b = min(scan_max_y, max_y + padding)
        
        side = max(right_b - left_b, bottom_b - top_b)
        cx = (left_b + right_b) // 2
        cy = (top_b + bottom_b) // 2
        left_b, right_b = cx - side // 2, cx + side // 2
        top_b, bottom_b = cy - side // 2, cy + side // 2
        
        print(f"检测到右侧 V4 有底色版大小: {side}x{side}", flush=True)
        crop_and_key_fast(img_pixels, left_b, top_b, right_b, bottom_b, "logo_v4")
    else:
        print("未在右半边检测到 V4 Logo，绿幕截图可能解析出了残缺块", flush=True)


def crop_and_key_fast(src_pixels, l, t, r, b, name):
    cw = r - l
    ch = b - t
    transparent_img = Image.new("RGBA", (cw, ch))
    out_pixels = transparent_img.load()
    
    # 极速色差抠图与多阶羽化淡消溢绿
    for y in range(ch):
        for x in range(cw):
            rgba = src_pixels[l + x, t + y]
            pr, pg, pb, pa = rgba
            
            dist = abs(pr - 43) + abs(pg - 217) + abs(pb - 93)
            is_bg = (pg - pr > 100) and (pg - pb > 80)
            
            if is_bg or dist < 32:
                out_pixels[x, y] = (0, 0, 0, 0)
            elif dist < 160:
                # 渐变过渡削弱绿反射，保留边缘质感（如圆弧光线等）
                alpha_factor = (dist - 32) / 128.0
                alpha_factor = max(0.0, min(1.0, alpha_factor))
                
                max_oth = max(pr, pb)
                new_g = max_oth if pg > max_oth else pg
                new_a = int(pa * alpha_factor)
                
                out_pixels[x, y] = (pr, new_g, pb, new_a)
            else:
                out_pixels[x, y] = rgba
                
    # 导出至项目工作区根目录
    out_path1 = f'd:\\Windsurf-Project\\EgoSync\\探索\\egosync_{name}.png'
    transparent_img.save(out_path1, "PNG")
    print(f"已导出 PNG 成果至: {out_path1}", flush=True)
    
    # 同步拷贝到前端 assets 目录
    assets_dir = r'd:\Windsurf-Project\EgoSync\探索\egosync-app\src\assets'
    if os.path.exists(assets_dir):
        out_path2 = os.path.join(assets_dir, f'{name}.png')
        transparent_img.save(out_path2, "PNG")
        print(f"已拷贝 PNG 到前端 Assets: {out_path2}", flush=True)

if __name__ == '__main__':
    process_v4_logos()
