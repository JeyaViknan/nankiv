// Takes the README's screenshots of the real interface, in WebKit, the engine
// the app itself runs in.
//
//   swiftc -O scripts/screenshots/capture.swift -o /tmp/capture
//   /tmp/capture "http://localhost:1425/screens.html?theme=light#result" out.png 1080 760
//   /tmp/capture "http://localhost:1425/screens.html#share" card.png 1080 760 .share-pass
//
// The page is screens.html, which runs the interface on an invented season.
// SF Symbols are drawn here by macOS, the way src-tauri/macos/bridge.swift
// draws them for the app, and handed to the page as it asks for them.

import AppKit
import WebKit

/// The symbols the page asks for, drawn exactly as the app's bridge draws them.
final class Symbols: NSObject, WKScriptMessageHandlerWithReply {
  let weights: [String: NSFont.Weight] = [
    "ultralight": .ultraLight, "thin": .thin, "light": .light, "regular": .regular,
    "medium": .medium, "semibold": .semibold, "bold": .bold, "heavy": .heavy, "black": .black,
  ]

  func userContentController(
    _ controller: WKUserContentController, didReceive message: WKScriptMessage,
    replyHandler: @escaping (Any?, String?) -> Void
  ) {
    let requests = message.body as? [[String: Any]] ?? []
    replyHandler(requests.map(draw), nil)
  }

  func draw(_ r: [String: Any]) -> Any {
    guard let name = r["name"] as? String,
      let point = (r["pointSize"] as? NSNumber)?.doubleValue,
      let scale = (r["scale"] as? NSNumber)?.doubleValue,
      let symbol = NSImage(systemSymbolName: name, accessibilityDescription: nil)?
        .withSymbolConfiguration(
          .init(pointSize: point, weight: weights[r["weight"] as? String ?? ""] ?? .regular))
    else { return NSNull() }
    let size = symbol.size
    guard
      let bitmap = NSBitmapImageRep(
        bitmapDataPlanes: nil, pixelsWide: Int((size.width * scale).rounded(.up)),
        pixelsHigh: Int((size.height * scale).rounded(.up)), bitsPerSample: 8,
        samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB,
        bytesPerRow: 0, bitsPerPixel: 0)
    else { return NSNull() }
    bitmap.size = size
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
    symbol.draw(in: NSRect(origin: .zero, size: size))
    NSGraphicsContext.restoreGraphicsState()
    guard let png = bitmap.representation(using: .png, properties: [:]) else { return NSNull() }
    return [
      "width": size.width, "height": size.height,
      "url": "data:image/png;base64,\(png.base64EncodedString())",
    ]
  }
}

final class Capture: NSObject {
  let web: WKWebView
  let window: NSWindow
  let out: String

  init(url: URL, out: String, width: CGFloat, height: CGFloat) {
    self.out = out
    let config = WKWebViewConfiguration()
    config.userContentController.addScriptMessageHandler(
      Symbols(), contentWorld: .page, name: "symbols")
    // A capture window is never in front of anything, and WebKit throttles
    // a page it thinks nobody can see: timers crawl and entrances never end.
    // These private switches, for a local tool, tell it to render regardless.
    config.preferences.setValue(false, forKey: "hiddenPageDOMTimerThrottlingEnabled")
    config.preferences.setValue(false, forKey: "pageVisibilityBasedProcessSuppressionEnabled")
    web = WKWebView(frame: NSRect(x: 0, y: 0, width: width, height: height), configuration: config)
    web.setValue(false, forKey: "windowOcclusionDetectionEnabled")
    window = NSWindow(
      contentRect: web.frame, styleMask: [.borderless], backing: .buffered, defer: false)
    super.init()
    window.contentView = web
    // On screen but behind everything and fully transparent: a web view that
    // is off screen stops animating, and the scenes wait for their entrances.
    window.alphaValue = 0.01
    window.ignoresMouseEvents = true
    window.level = .init(rawValue: Int(CGWindowLevelForKey(.desktopWindow)))
    window.orderFrontRegardless()
    web.load(URLRequest(url: url))
    poll()
  }

  var polls = 0

  func poll() {
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) {
      self.polls += 1
      self.web.evaluateJavaScript("document.documentElement.dataset.ready || ''") { value, _ in
        if (value as? String) == "1" { self.shoot(); return }
        if let v = value as? String, v.hasPrefix("error") {
          FileHandle.standardError.write("scene failed: \(v)\n".data(using: .utf8)!)
          exit(1)
        }
        // Every few seconds, say where the page has got to.
        if self.polls % 16 == 0 {
          self.web.evaluateJavaScript(
            "[location.href, document.readyState, (document.body && document.body.innerText || '').slice(0, 80)].join(' | ')"
          ) { state, error in
            FileHandle.standardError.write("waiting: \(state ?? "-") \(error.map { "\($0)" } ?? "")\n".data(using: .utf8)!)
          }
        }
        self.poll()
      }
    }
  }

  /// A canvas on the page, saved as the PNG it holds rather than as a
  /// picture of the screen: the share card exactly as it is shared.
  var canvas: String? = nil

  func shoot() {
    if let selector = canvas {
      web.evaluateJavaScript("document.querySelector('\(selector)').toDataURL('image/png')") { value, error in
        guard let url = value as? String, let comma = url.firstIndex(of: ","),
          let data = Data(base64Encoded: String(url[url.index(after: comma)...]))
        else {
          FileHandle.standardError.write("canvas export failed: \(String(describing: error))\n".data(using: .utf8)!)
          exit(1)
        }
        try! data.write(to: URL(fileURLWithPath: self.out))
        print("\(self.out) canvas")
        exit(0)
      }
      return
    }
    // A close-up's bounds, as the page reported them, beside the picture.
    web.evaluateJavaScript("document.documentElement.dataset.crop || ''") { value, _ in
      if let crop = value as? String, !crop.isEmpty {
        try? crop.write(toFile: self.out + ".crop", atomically: true, encoding: .utf8)
      }
    }
    web.takeSnapshot(with: nil) { image, error in
      guard let image, let cg = image.cgImage(forProposedRect: nil, context: nil, hints: nil)
      else {
        FileHandle.standardError.write("snapshot failed: \(String(describing: error))\n".data(using: .utf8)!)
        exit(1)
      }
      let rep = NSBitmapImageRep(cgImage: cg)
      try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: self.out))
      print("\(self.out) \(cg.width)x\(cg.height)")
      exit(0)
    }
  }
}

let args = CommandLine.arguments
let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let capture = Capture(
  url: URL(string: args[1])!, out: args[2],
  width: CGFloat(Double(args[3]) ?? 1080), height: CGFloat(Double(args[4]) ?? 760))
if args.count > 5 { capture.canvas = args[5] }
DispatchQueue.main.asyncAfter(deadline: .now() + 30) {
  FileHandle.standardError.write("timed out waiting for the scene\n".data(using: .utf8)!)
  exit(1)
}
app.run()
