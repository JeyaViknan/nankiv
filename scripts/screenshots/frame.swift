// Puts a capture in a macOS window: rounded corners, a hairline edge, the
// traffic lights where nankiv's window has them, and a soft shadow, on a
// transparent ground so it sits on light and dark pages alike.
//
//   frame <in.png> <out.png> <light|dark> [crop x y w h, in points]
//
// With a crop there are no traffic lights: it is a detail, not a window.

import AppKit

let a = CommandLine.arguments
let source = NSBitmapImageRep(data: try! Data(contentsOf: URL(fileURLWithPath: a[1])))!
let dark = a[3] == "dark"
let scale: CGFloat = 2
var crop = CGRect(x: 0, y: 0, width: CGFloat(source.pixelsWide) / scale, height: CGFloat(source.pixelsHigh) / scale)
let isCrop = a.count >= 8
if isCrop { crop = CGRect(x: Double(a[4])!, y: Double(a[5])!, width: Double(a[6])!, height: Double(a[7])!) }

let radius: CGFloat = isCrop ? 14 : 19
let margin: CGFloat = isCrop ? 36 : 64
let W = (crop.width + margin * 2) * scale, H = (crop.height + margin * 2) * scale
let out = NSBitmapImageRep(
  bitmapDataPlanes: nil, pixelsWide: Int(W), pixelsHigh: Int(H), bitsPerSample: 8,
  samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB,
  bytesPerRow: 0, bitsPerPixel: 0)!
NSGraphicsContext.saveGraphicsState()
let ctx = NSGraphicsContext(bitmapImageRep: out)!
NSGraphicsContext.current = ctx
let cg = ctx.cgContext
cg.scaleBy(x: scale, y: scale)

// Flipped once, so everything below is in the capture's own top-down points.
let total = CGSize(width: crop.width + margin * 2, height: crop.height + margin * 2)
cg.translateBy(x: 0, y: total.height)
cg.scaleBy(x: 1, y: -1)
let body = CGRect(x: margin, y: margin, width: crop.width, height: crop.height)
let shape = CGPath(roundedRect: body, cornerWidth: radius, cornerHeight: radius, transform: nil)

// The shadow: one wide and soft, one close and tighter, as a window casts.
for (blur, y, alpha) in [(48.0, 22.0, dark ? 0.55 : 0.22), (10.0, 4.0, dark ? 0.35 : 0.12)] {
  cg.saveGState()
  cg.setShadow(offset: CGSize(width: 0, height: -y), blur: blur, color: NSColor(white: 0, alpha: alpha).cgColor)
  cg.addPath(shape)
  cg.setFillColor(NSColor(white: dark ? 0.1 : 1, alpha: 1).cgColor)
  cg.fillPath()
  cg.restoreGState()
}

// The capture, cut to the window's shape.
cg.saveGState()
cg.addPath(shape)
cg.clip()
let px = CGRect(x: crop.minX * scale, y: crop.minY * scale, width: crop.width * scale, height: crop.height * scale)
let part = source.cgImage!.cropping(to: px)!
cg.translateBy(x: 0, y: body.maxY + body.minY)
cg.scaleBy(x: 1, y: -1)
cg.draw(part, in: body)
cg.restoreGState()

// The traffic lights, where tauri.conf.json puts them; the third is the
// disabled zoom button, since nankiv's window does not go full screen.
if !isCrop {
  let lights: [(NSColor, NSColor)] = [
    (NSColor(srgbRed: 1, green: 0.373, blue: 0.341, alpha: 1), NSColor(srgbRed: 0.878, green: 0.267, blue: 0.243, alpha: 1)),
    (NSColor(srgbRed: 0.996, green: 0.737, blue: 0.180, alpha: 1), NSColor(srgbRed: 0.871, green: 0.631, blue: 0.137, alpha: 1)),
    dark
      ? (NSColor(white: 0.33, alpha: 1), NSColor(white: 0.28, alpha: 1))
      : (NSColor(white: 0.86, alpha: 1), NSColor(white: 0.78, alpha: 1)),
  ]
  for (i, (fill, edge)) in lights.enumerated() {
    let c = CGPoint(x: body.minX + 26.5 + CGFloat(i) * 23, y: body.minY + 21.5)
    let r = CGRect(x: c.x - 7, y: c.y - 7, width: 14, height: 14)
    cg.setFillColor(fill.cgColor); cg.fillEllipse(in: r)
    cg.setStrokeColor(edge.cgColor); cg.setLineWidth(0.5); cg.strokeEllipse(in: r.insetBy(dx: 0.25, dy: 0.25))
  }
}

// The hairline edge a window has.
cg.addPath(shape)
cg.setStrokeColor(NSColor(white: dark ? 1 : 0, alpha: dark ? 0.16 : 0.10).cgColor)
cg.setLineWidth(1)
cg.strokePath()

NSGraphicsContext.restoreGraphicsState()
try! out.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: a[2]))
print("\(a[2]) \(Int(W))x\(Int(H))")
