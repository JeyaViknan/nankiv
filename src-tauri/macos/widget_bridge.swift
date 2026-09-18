// Tells WidgetKit that the widget snapshot changed.
//
// WidgetKit's reload API exists only in Swift, so the Rust core reaches it
// through this one C-callable function, compiled into a static library by
// build.rs. It is the whole bridge: the widget reads its data from the snapshot
// file, never from here.

import WidgetKit

@_cdecl("nankiv_reload_widget_timelines")
public func nankivReloadWidgetTimelines() {
    // WidgetKit is weakly linked, so the app still opens on macOS 10.15.
    guard #available(macOS 11.0, *) else { return }
    // Must match ShortlistWidget.kind in widgets/macos/Extension.
    WidgetCenter.shared.reloadTimelines(ofKind: "app.nankiv.widget.shortlist")
}
