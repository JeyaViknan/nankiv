import NankivWidgetModel
import NankivWidgetViews
import SwiftUI
import WidgetKit

@main
struct NankivWidgets: WidgetBundle {
    var body: some Widget {
        ShortlistWidget()
    }
}

/// The desktop widget: your latest shortlist result, at a glance.
struct ShortlistWidget: Widget {
    static let kind = "app.nankiv.widget.shortlist"

    var body: some WidgetConfiguration {
        // Static, with nothing to configure.
        //
        // An AppIntentConfiguration would let someone pin the widget to one
        // shortlist, and it is written and tested in the model. It cannot ship
        // yet: the system has to hand the widget an intent configuration, and
        // it will not for an app built outside Xcode and signed ad hoc —
        // chronod refuses every timeline with "Intent configuration is required
        // but was not provided", which leaves the widget showing its loading
        // placeholder forever. Following the latest shortlist needs no
        // configuration, which is the behaviour almost everyone wants anyway.
        StaticConfiguration(kind: Self.kind, provider: GlanceProvider()) { entry in
            GlanceEntryView(entry: entry)
        }
        .configurationDisplayName("Shortlist")
        .description("See whether you made your latest shortlist, and what it suggests.")
        .supportedFamilies([.systemSmall, .systemMedium, .systemLarge, .systemExtraLarge])
    }
}

struct GlanceEntry: TimelineEntry {
    let date: Date
    let glance: Glance
}

struct GlanceEntryView: View {
    let entry: GlanceEntry

    @Environment(\.widgetFamily) private var family

    var body: some View {
        GlanceView(glance: entry.glance, family: family, now: entry.date)
            .containerBackground(for: .widget) { GlanceBackground() }
            // One destination for the whole widget: the shortlist it shows.
            // Rows in the extra-large recent list carry their own links.
            .widgetURL(entry.glance.destination)
    }
}

/// Supplies entries from the snapshot the app writes.
///
/// It reads one small file and does no analysis — the app has already done it.
/// Timelines are rebuilt when the app asks (it reloads them whenever the
/// snapshot changes); the entries here only cover what changes with time alone.
struct GlanceProvider: TimelineProvider {
    /// No configuration yet, so the widget always follows the latest shortlist.
    private let pinned: Int64? = nil

    func placeholder(in context: Context) -> GlanceEntry {
        // Drawn redacted by WidgetKit; invented content only.
        GlanceEntry(date: Date(), glance: .sample())
    }

    func getSnapshot(in context: Context, completion: @escaping (GlanceEntry) -> Void) {
        let now = Date()
        let glance = Glance.make(from: SnapshotSource.standard.load(), pinned: pinned, at: now)
        // In the widget gallery, show what the widget does, not a setup
        // message: someone deciding whether to add it needs to see its value.
        if context.isPreview, case .shortlist = glance {
            completion(GlanceEntry(date: now, glance: glance))
        } else if context.isPreview {
            completion(GlanceEntry(date: now, glance: .sample(now: now)))
        } else {
            completion(GlanceEntry(date: now, glance: glance))
        }
    }

    func getTimeline(in context: Context, completion: @escaping (Timeline<GlanceEntry>) -> Void) {
        let now = Date()
        let load = SnapshotSource.standard.load()
        let plan = Schedule.plan(for: load, from: now)
        let entries = plan.moments.map { moment in
            GlanceEntry(date: moment, glance: Glance.make(from: load, pinned: pinned, at: moment))
        }
        completion(Timeline(entries: entries, policy: .after(plan.reload)))
    }
}
