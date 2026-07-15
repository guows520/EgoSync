import glob
import os
from PIL import Image

def process_transparent():
    # 找寻最新的绿幕截图
    pattern = r'C:\Users\Admin\.gemini\antigravity\brain\28c43195-75a0-432a-91b2-38e02651be47\logo_green_screen_*.png'
    files = glob.glob(pattern)
    if not files:
        print("未找到绿幕截图")
        return
        
    file_path = files[-1]
    print(f"载入绿幕截图: {file_path}")
    
    img = Image.open(file_path).convert("RGBA")
    width, height = img.size
    print(f"图像尺寸: {width}x{height}")
    
    non_green_xs = []
    non_green_ys = []
    
    # 限制在中心区域检测 Logo 以过滤掉窗口外的杂色/边框
    scan_min_x = int(width * 0.15)
    scan_max_x = int(width * 0.85)
    scan_min_y = int(height * 0.15)
    scan_max_y = int(height * 0.85)
    
    # 绿底由于色彩空间转换，实际为 (43, 217, 93) 左右
    for y in range(scan_min_y, scan_max_y, 2):
        for x in range(scan_min_x, scan_max_x, 2):
            r, g, b, _ = img.getpixel((x, y))
            # 绿幕的特征是 G 显著压倒 R 与 B
            is_green_screen = (g - r > 100) and (g - b > 80)
            if not is_green_screen:
                non_green_xs.append(x)
                non_green_ys.append(y)
                
    if not non_green_xs:
        print("没有检测到非绿幕的 Logo 主体区域")
        return
        
    min_x, max_x = min(non_green_xs), max(non_green_xs)
    min_y, max_y = min(non_green_ys), max(non_green_ys)
    
    print(f"Logo 内部初步检测 X: [{min_x}, {max_x}], Y: [{min_y}, {max_y}]")
    
    # 适当 padding
    padding = 10
    left_boundary = max(scan_min_x, min_x - padding)
    right_boundary = min(scan_max_x, max_x + padding)
    top_boundary = max(scan_min_y, min_y - padding)
    bottom_boundary = min(scan_max_y, max_y + padding)
    
    box_w = right_boundary - left_boundary
    box_h = bottom_boundary - top_boundary
    side = max(box_w, box_h)
    
    # 转化为纯正方形
    center_x = (left_boundary + right_boundary) // 2
    center_y = (top_boundary + bottom_boundary) // 2
    
    left_boundary = max(0, center_x - side // 2)
    right_boundary = min(width, left_boundary + side)
    top_boundary = max(0, center_y - side // 2)
    bottom_boundary = min(height, top_boundary + side)
    
    print(f"Logo 正方形提取区域: X: [{left_boundary}, {right_boundary}], Y: [{top_boundary}, {bottom_boundary}] ({side}x{side})")
    
    cropped = img.crop((left_boundary, top_boundary, right_boundary, bottom_boundary))
    cw, ch = cropped.size
    
    transparent_img = Image.new("RGBA", (cw, ch))
    
    # 绿幕去背公式与偏色处理
    for y in range(ch):
        for x in range(cw):
            r, g, b, a = cropped.getpixel((x, y))
            
            # 计算到纯绿底色 (43, 217, 93) 的距离
            dist = abs(r - 43) + abs(g - 217) + abs(b - 93)
            
            # 判断是否是强烈的背景像素
            is_bg = (g - r > 100) and (g - b > 80)
            
            if is_bg or dist < 30:
                # 认定属于绿幕区，转为透明
                transparent_img.putpixel((x, y), (0, 0, 0, 0))
            elif dist < 160:
                # 渐变过渡与抗锯齿区：平滑淡化绿色溢光
                alpha_factor = (dist - 30) / 130.0
                alpha_factor = max(0.0, min(1.0, alpha_factor))
                
                # 绿色去溢
                max_other = max(r, b)
                new_g = max_other if g > max_other else g
                new_a = int(a * alpha_factor)
                transparent_img.putpixel((x, y), (r, new_g, b, new_a))
            else:
                # 完全的主体像素
                transparent_img.putpixel((x, y), (r, g, b, a))
                
    # 成果导出
    out_path1 = r'd:\Windsurf-Project\EgoSync\探索\egosync_logo_v1_transparent.png'
    transparent_img.save(out_path1, "PNG")
    print(f"已导出透明 PNG 到项目根目录: {out_path1}")
    
    assets_dir = r'd:\Windsurf-Project\EgoSync\探索\egosync-app\src\assets'
    if os.path.exists(assets_dir):
        out_path2 = os.path.join(assets_dir, 'logo_transparent.png')
        transparent_img.save(out_path2, "PNG")
        print(f"已拷贝透明 PNG 到前端 Assets: {out_path2}")
        
if __name__ == '__main__':
    process_transparent()
