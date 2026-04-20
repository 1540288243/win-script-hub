# Icon generator for Win Script Hub
# Run: python generate_icon.py

from PIL import Image, ImageDraw, ImageFont
import os

icon_dir = r"d:\KaiFa\YuanMa\HouDuan\yx-launch-Platform\win-script-hub\icons"
os.makedirs(icon_dir, exist_ok=True)

sizes = [16, 32, 48, 64, 128, 256]
images = []

for size in sizes:
    img = Image.new('RGBA', (size, size), (37, 99, 235, 255))
    draw = ImageDraw.Draw(img)

    # Draw rounded rectangle background
    margin = max(1, size // 16)
    draw.rounded_rectangle(
        [margin, margin, size - margin - 1, size - margin - 1],
        radius=size // 6,
        fill=(37, 99, 235, 255)
    )

    # Draw lightning bolt symbol
    try:
        font_size = int(size * 0.55)
        font = ImageFont.truetype("seguiemj.ttf", font_size) if os.path.exists("seguiemj.ttf") else ImageFont.load_default()
    except:
        font = ImageFont.load_default()

    # Use text rendering for symbol
    text = chr(0x26A1)  # Lightning symbol
    bbox = draw.textbbox((0, 0), text, font=font)
    text_width = bbox[2] - bbox[0]
    text_height = bbox[3] - bbox[1]
    x = (size - text_width) // 2
    y = (size - text_height) // 2 - size // 20
    draw.text((x, y), text, fill=(255, 255, 255, 255), font=font)

    images.append(img)

# Save as PNG
png_path = os.path.join(icon_dir, "icon.png")
images[-1].save(png_path)
print(f"PNG saved: {png_path}")

# Save as ICO (multi-size)
ico_path = os.path.join(icon_dir, "icon.ico")
images[0].save(ico_path, format='ICO', sizes=[(s, s) for s in sizes])
print(f"ICO saved: {ico_path}")

print("Done!")
