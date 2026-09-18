import AppIntents
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
        AppIntentConfiguration(kind: Self.kind, intent: ChooseShortlist.self, provider: GlanceProvider()) { entry in
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
struct GlanceProvider: AppIntentTimelineProvider {
    func placeholder(in context: Context) -> GlanceEntry {
        // Drawn redacted by WidgetKit; invented content only.
        GlanceEntry(date: Date(), glance: .sample())
    }

    func snapshot(for configuration: ChooseShortlist, in context: Context) async -> GlanceEntry {
        let now = Date()
        let glance = Glance.make(from: SnapshotSource.standard.load(), pinned: configuration.pinnedID, at: now)
        // In the widget gallery, show what the widget does, not a setup
        // message: someone deciding whether to add it needs to see its value.
        if context.isPreview, case .shortlist = glance {
            return GlanceEntry(date: now, glance: glance)
        } else if context.isPreview {
            return GlanceEntry(date: now, glance: .sample(now: now))
        }
        return GlanceEntry(date: now, glance: glance)
    }

    func timeline(for configuration: ChooseShortlist, in context: Context) async -> Timeline<GlanceEntry> {
        let now = Date()
        let load = SnapshotSource.standard.load()
        let plan = Schedule.plan(for: load, from: now)
        let entries = plan.moments.map { moment in
            GlanceEntry(date: moment, glance: Glance.make(from: load, pinned: configuration.pinnedID, at: moment))
        }
        return Timeline(entries: entries, policy: .after(plan.reload))
    }
}
