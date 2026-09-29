// The few things only AppKit can do, reached from Rust.
//
// Each is one call, compiled into a static library by build.rs. Nothing
// here holds state or makes decisions: the core decides, this performs.

import AppKit
import WidgetKit

/// Tells WidgetKit the snapshot changed.
@_cdecl("nankiv_reload_widget_timelines")
public func nankivReloadWidgetTimelines() {
    // WidgetKit is weakly linked, so the app still opens on macOS 10.15.
    guard #available(macOS 11.0, *) else { return }
    // Must match ShortlistWidget.kind in widgets/macos/Extension.
    WidgetCenter.shared.reloadTimelines(ofKind: "app.nankiv.widget.shortlist")
}

/// A single tap on the trackpad, for the moment a result lands.
///
/// The system decides whether it happens at all: it does nothing on a mouse,
/// and nothing when the person has turned trackpad feedback off. That is the
/// point of using the platform's own haptics rather than inventing something.
@_cdecl("nankiv_tap")
public func nankivTap() {
    DispatchQueue.main.async {
        NSHapticFeedbackManager.defaultPerformer.perform(
            .generic, performanceTime: .now)
    }
}

/// An SF Symbol, drawn by the system at the size it will be shown.
///
/// The symbols come from macOS itself, by name, the way a native app gets
/// them: nankiv ships none of Apple's artwork, so nothing of it is in the
/// repository or in the Windows build, where it is not licensed for use.
///
/// Returns "width height base64png" — the size in points and a PNG at `scale`
/// pixels per point — or nothing if this Mac does not have the symbol. The
/// caller frees the string with `nankiv_free_string`.
@_cdecl("nankiv_symbol")
public func nankivSymbol(
    _ name: UnsafePointer<CChar>, _ pointSize: Double, _ weight: Int32, _ scale: Double
) -> UnsafeMutablePointer<CChar>? {
    // Symbols by name arrived in macOS 11. Before that, the interface keeps
    // its own drawings.
    guard #available(macOS 11.0, *) else { return nil }
    let weights: [NSFont.Weight] = [
        .ultraLight, .thin, .light, .regular, .medium, .semibold, .bold, .heavy, .black,
    ]
    let config = NSImage.SymbolConfiguration(
        pointSize: pointSize, weight: weights[Int(max(0, min(weight, 8)))])
    guard
        let symbol = NSImage(
            systemSymbolName: String(cString: name), accessibilityDescription: nil)?
            .withSymbolConfiguration(config)
    else { return nil }

    let size = symbol.size
    guard
        let bitmap = NSBitmapImageRep(
            bitmapDataPlanes: nil,
            pixelsWide: Int((size.width * scale).rounded(.up)),
            pixelsHigh: Int((size.height * scale).rounded(.up)),
            bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
            colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)
    else { return nil }
    bitmap.size = size

    // Only the shape matters: the interface uses it as a mask and fills it
    // with whatever colour the text around it has.
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
    symbol.draw(in: NSRect(origin: .zero, size: size))
    NSGraphicsContext.restoreGraphicsState()

    guard let png = bitmap.representation(using: .png, properties: [:]) else { return nil }
    return strdup("\(size.width) \(size.height) \(png.base64EncodedString())")
}

@_cdecl("nankiv_free_string")
public func nankivFreeString(_ string: UnsafeMutablePointer<CChar>?) {
    free(string)
}
