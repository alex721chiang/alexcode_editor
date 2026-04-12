from PIL import Image

try:
    # Open the verified V2 image
    img = Image.open('/Users/alextw/.openclaw/workspace/2026-03-25-alexcode-icon-v2.png')
    
    # Convert to RGBA to ensure proper transparency/alpha channel handling for Windows icons
    img = img.convert("RGBA")
    
    # Save as a standard 256x256 PNG for Qt internal window icon
    img_256 = img.resize((256, 256), Image.Resampling.LANCZOS)
    img_256.save('/Users/alextw/.openclaw/workspace/alexcode_editor/src/icon.png', format="PNG")
    
    # Save as ICO with multiple sizes
    img.save('/Users/alextw/.openclaw/workspace/alexcode_editor/src/alexcode.ico', format='ICO', sizes=[(256, 256), (128, 128), (64, 64), (48, 48), (32, 32), (16, 16)])
    
    print("Icons successfully regenerated in RGBA format!")
except Exception as e:
    print(f"Error: {e}")
