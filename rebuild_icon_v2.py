from PIL import Image

try:
    img = Image.open('/Users/alextw/.openclaw/workspace/2026-03-25-alexcode-icon-v2.png').convert("RGBA")
    
    # Save as ICO with NO 256x256 layer to prevent PNG compression issues in rc.exe
    img.save('/Users/alextw/.openclaw/workspace/alexcode_editor/src/alexcode.ico', format='ICO', sizes=[(128, 128), (64, 64), (48, 48), (32, 32), (16, 16)])
    
    print("ICO generated without 256x256 layer.")
except Exception as e:
    print(f"Error: {e}")
