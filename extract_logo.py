import glob
import os
from PIL import Image

def extract():
    # 查找匹配的截图文件
    pattern = r'C:\Users\Admin\.gemini\antigravity\brain\28c43195-75a0-432a-91b2-38e02651be47\logo_variants_preview_*.png'
    files = glob.glob(pattern)
    if not files:
        print("未找到默认路径下的截图")
        return
    
    file_path = files[0]
    print(f"找到截图: {file_path}")
    
    img = Image.open(file_path).convert("RGBA")
    width, height = img.size
    print(f"图像尺寸: {width}x{height}")
    
    target_rgb = (15, 23, 42) # #0F172A
    tolerance = 2
    
    matching_pixels = []
    for y in range(0, height):
        for x in range(0, width):
            r, g, b, a = img.getpixel((x, y))
            if abs(r - target_rgb[0]) <= tolerance and abs(g - target_rgb[1]) <= tolerance and abs(b - target_rgb[2]) <= tolerance:
                matching_pixels.append((x, y))
                
    if not matching_pixels:
        print("未匹配到特定颜色块")
        return
        
    # 第一个 Logo 放在最左侧，过滤出左侧三分之一区域内的像素
    left_third_pixels = [p for p in matching_pixels if p[0] < width / 3]
    if not left_third_pixels:
        print("左侧区域未找到匹配像素")
        return
        
    xs = [p[0] for p in left_third_pixels]
    ys = [p[1] for p in left_third_pixels]
    
    min_x, max_x = min(xs), max(xs)
    min_y, max_y = min(ys), max(ys)
    
    # 递归向内缩进/向外扩展，确定左右精确边缘
    # 在 min_x 附近探测，直到彻底退出背景色
    left_boundary = min_x
    while left_boundary > 0:
        has_match = False
        for y_test in range(min_y, max_y):
            r, g, b, _ = img.getpixel((left_boundary, y_test))
            if abs(r - target_rgb[0]) <= tolerance and abs(g - target_rgb[1]) <= tolerance and abs(b - target_rgb[2]) <= tolerance:
                has_match = True
                break
        if not has_match:
            break
        left_boundary -= 1
        
    right_boundary = max_x
    while right_boundary < width:
        has_match = False
        for y_test in range(min_y, max_y):
            r, g, b, _ = img.getpixel((right_boundary, y_test))
            if abs(r - target_rgb[0]) <= tolerance and abs(g - target_rgb[1]) <= tolerance and abs(b - target_rgb[2]) <= tolerance:
                has_match = True
                break
        if not has_match:
            break
        right_boundary += 1
        
    box_w = right_boundary - left_boundary
    print(f"检测到的横向精准宽度: {box_w}px (从 {left_boundary} 到 {right_boundary})")
    
    # 采用滑动窗口寻找包含最多匹配像素的 Y 轴区间 [y, y + box_w]
    # 我们将把 matching_pixels 中落在 [left_boundary, right_boundary] 区间的点整理出来
    col_pixels = [p for p in left_third_pixels if left_boundary <= p[0] <= right_boundary]
    
    best_y = min_y
    max_count = 0
    
    # 沿着 Y 轴上下扫描滑动窗口
    for y_start in range(min_y, max_y - box_w + 1):
        # 统计在这个窗口内的像素个数
        count = sum(1 for p in col_pixels if y_start <= p[1] <= y_start + box_w)
        if count > max_count:
            max_count = count
            best_y = y_start
            
    top_boundary = best_y
    bottom_boundary = best_y + box_w
    
    print(f"滑动窗口检测到的最佳纵向高度: {box_w}px (从 {top_boundary} 到 {bottom_boundary})")
    
    # 裁剪并保存
    cropped = img.crop((left_boundary, top_boundary, right_boundary, bottom_boundary))
    
    # 保存至项目根目录与资产目录
    out_path1 = r'd:\Windsurf-Project\EgoSync\探索\egosync_logo_v1.png'
    cropped.save(out_path1, "PNG")
    print(f"已导出项目 PNG: {out_path1}")
    
    assets_dir = r'd:\Windsurf-Project\EgoSync\探索\egosync-app\src\assets'
    if os.path.exists(assets_dir):
        out_path2 = os.path.join(assets_dir, 'logo.png')
        cropped.save(out_path2, "PNG")
        print(f"已拷贝 PNG 到前端 assets: {out_path2}")
        
if __name__ == '__main__':
    extract()
