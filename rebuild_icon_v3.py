from PIL import Image, ImageDraw

try:
    img = Image.open('/Users/alextw/.openclaw/workspace/2026-03-25-alexcode-icon-v2.png').convert("RGBA")
    
    # Add a rounded mask or a transparent border to force a valid alpha channel
    mask = Image.new("L", img.size, 0)
    draw = ImageDraw.Draw(mask)
    # Draw a rounded rectangle mask with 100px radius
    draw.rounded_rectangle((0, 0, img.width, img.height), radius=200, fill=255)
    
    # Apply mask
    img.putalpha(mask)
    
    img_256 = img.resize((256, 256), Image.Resampling.LANCZOS)
    img_256.save('/Users/alextw/.openclaw/workspace/alexcode_editor/src/icon.png', format="PNG")
    
    img.save('/Users/alextw/.openclaw/workspace/alexcode_editor/src/alexcode.ico', format='ICO', sizes=[(256, 256), (128, 128), (64, 64), (48, 48), (32, 32), (16, 16)])
    
    print("ICO with transparency mask generated.")
except Exception as e:
    print(f"Error: {e}")
