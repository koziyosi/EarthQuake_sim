from PIL import Image
import json

img = Image.open('assets/ui_assets.png')
# Convert to RGBA
img = img.convert('RGBA')
width, height = img.size

# Simple detection: find non-transparent/non-white regions
# In the screenshot, the background is white.
def is_bg(p):
    return p[0] > 250 and p[1] > 250 and p[2] > 250

regions = []
visited = set()

for y in range(0, height, 10):
    for x in range(0, width, 10):
        if (x, y) in visited: continue
        p = img.getpixel((x, y))
        if not is_bg(p):
            # Found an object, find its bounds
            min_x, min_y, max_x, max_y = x, y, x, y
            stack = [(x, y)]
            visited.add((x, y))
            while stack:
                cx, cy = stack.pop()
                min_x = min(min_x, cx)
                min_y = min(min_y, cy)
                max_x = max(max_x, cx)
                max_y = max(max_y, cy)
                for dx, dy in [(-10, 0), (10, 0), (0, -10), (0, 10)]:
                    nx, ny = cx + dx, cy + dy
                    if 0 <= nx < width and 0 <= ny < height and (nx, ny) not in visited:
                        if not is_bg(img.getpixel((nx, ny))):
                            visited.add((nx, ny))
                            stack.append((nx, ny))
            regions.append((min_x, min_y, max_x - min_x, max_y - min_y))

# Sort regions by y, then x
regions.sort(key=lambda r: (r[1] // 100, r[0]))

with open('assets/ui_rects.json', 'w') as f:
    json.dump(regions, f)

print(f"Detected {len(regions)} regions")
