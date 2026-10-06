// Gives a cut-out image (the share card, the widgets) the soft shadow a real
// object casts, following its own outline, on a transparent ground.
//
//   shadow <in.png> <out.png> [corner-radius-px]
import AppKit

let a = CommandLine.arguments
let src = NSBitmapImageRep(data: try! Data(contentsOf: URL(fileURLWithPath: a[1])))!
var image = src.cgImage!
let w = CGFloat(image.width), h = CGFloat(image.height)
let pad = (w / 12).rounded()
let out = NSBitmapImageRep(
  bitmapDataPlanes: nil, pixelsWide: Int(w + pad * 2), pixelsHigh: Int(h + pad * 2),
  bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
  colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: out)
let cg = NSGraphicsContext.current!.cgContext
let rect = CGRect(x: pad, y: pad, width: w, height: h)
// An opaque image gets its corners rounded first, so the shadow follows them.
if a.count > 3, let r = Double(a[3]) {
  cg.addPath(CGPath(roundedRect: rect, cornerWidth: r, cornerHeight: r, transform: nil))
  cg.clip()
}
cg.setShadow(offset: CGSize(width: 0, height: -pad * 0.35), blur: pad * 0.8, color: NSColor(white: 0, alpha: 0.32).cgColor)
cg.beginTransparencyLayer(auxiliaryInfo: nil)
cg.draw(image, in: rect)
cg.endTransparencyLayer()
NSGraphicsContext.restoreGraphicsState()
try! out.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: a[2]))
print(a[2])
