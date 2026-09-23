// The few things only AppKit can do, reached from Rust.
//
// Both are one call each, compiled into a static library by build.rs. Nothing
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
