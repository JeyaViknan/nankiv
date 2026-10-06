// Lays the widget renders out as they sit on a desktop: large on the left,
// medium over small on the right, on a soft wallpaper.
//
//   widgets <ready.png from nankiv-widget-render> <out.png>
import AppKit

let a = CommandLine.arguments
let sheet = NSBitmapImageRep(data: try! Data(contentsOf: URL(fileURLWithPath: a[1])))!.cgImage!
// The light row of the contact sheet, in its pixels.
let small = CGRect(x: 48, y: 174, width: 340, height: 340)
let medium = CGRect(x: 425, y: 174, width: 728, height: 340)
let large = CGRect(x: 1190, y: 174, width: 728, height: 768)
let gap: CGFloat = 44, pad: CGFloat = 96, radius: CGFloat = 46
let W = pad * 2 + large.width + gap + medium.width, H = pad * 2 + large.height
let out = NSBitmapImageRep(
  bitmapDataPlanes: nil, pixelsWide: Int(W), pixelsHigh: Int(H), bitsPerSample: 8,
  samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB,
  bytesPerRow: 0, bitsPerPixel: 0)!
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: out)
let cg = NSGraphicsContext.current!.cgContext
let ground = CGPath(roundedRect: CGRect(x: 0, y: 0, width: W, height: H), cornerWidth: 40, cornerHeight: 40, transform: nil)
cg.addPath(ground); cg.clip()
NSGradient(colors: [
  NSColor(srgbRed: 0.47, green: 0.62, blue: 0.93, alpha: 1),
  NSColor(srgbRed: 0.62, green: 0.55, blue: 0.90, alpha: 1),
  NSColor(srgbRed: 0.93, green: 0.66, blue: 0.75, alpha: 1),
])!.draw(in: NSRect(x: 0, y: 0, width: W, height: H), angle: -35)

func place(_ crop: CGRect, at origin: CGPoint) {
  let r = CGRect(origin: origin, size: crop.size)
  cg.saveGState()
  cg.setShadow(offset: CGSize(width: 0, height: -14), blur: 40, color: NSColor(white: 0, alpha: 0.22).cgColor)
  cg.beginTransparencyLayer(auxiliaryInfo: nil)
  cg.addPath(CGPath(roundedRect: r, cornerWidth: radius, cornerHeight: radius, transform: nil))
  cg.clip()
  cg.draw(sheet.cropping(to: crop)!, in: r)
  cg.endTransparencyLayer()
  cg.restoreGState()
}
// Core Graphics counts up from the bottom.
place(large, at: CGPoint(x: pad, y: pad))
place(medium, at: CGPoint(x: pad + large.width + gap, y: pad + large.height - medium.height))
place(small, at: CGPoint(x: pad + large.width + gap, y: pad + large.height - medium.height - gap - small.height))
NSGraphicsContext.restoreGraphicsState()
try! out.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: a[2]))
print(a[2])
