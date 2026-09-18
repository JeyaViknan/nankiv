#if DEBUG
import NankivWidgetModel
import SwiftUI
import WidgetKit

// Every state, at every size.
//
// Each preview is a timeline: step through its entries in the canvas to see
// each state in turn. Use the canvas's colour scheme and widget rendering mode
// controls to see each one light, dark, vibrant and accented.
//
// The entries come from widgets/fixtures — the same snapshots the Rust and
// Swift tests check — so a preview shows exactly what the app would publish.

private enum PreviewTimeline {
    static let fixtures = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()  // Previews.swift
        .deletingLastPathComponent()  // Extension
        .deletingLastPathComponent()  // macos
        .appendingPathComponent("fixtures")

    static let now = ISO8601DateFormatter().date(from: "2026-09-17T15:01:00Z")!

    static func load(_ name: String) -> SnapshotLoad {
        guard let data = try? Data(contentsOf: fixtures.appendingPathComponent("\(name).json")) else {
            return .missing
        }
        return SnapshotSource.decode(data)
    }

    static func entry(_ name: String, pinned: Int64? = nil, offset: TimeInterval) -> GlanceEntry {
        GlanceEntry(date: now.addingTimeInterval(offset), glance: Glance.make(from: load(name), pinned: pinned, at: now))
    }

    static var cedarID: Int64? {
        guard case .loaded(let s) = load("ready") else { return nil }
        return s.drives.first { $0.company == "Cedar Labs" }?.id
    }

    /// Distinct dates, so the canvas lists them as separate entries.
    static var all: [GlanceEntry] {
        let fixtures = [
            "ready", "importing", "failed", "not_shortlisted", "insufficient", "soft_preference",
            "undetermined", "not_set_up", "no_identity", "empty",
        ]
        var entries = fixtures.enumerated().map { entry($1, offset: Double($0)) }
        entries.append(entry("ready", pinned: cedarID, offset: 20))
        entries.append(entry("ready", pinned: 999_999, offset: 21))
        entries.append(GlanceEntry(date: now.addingTimeInterval(22), glance: .noShortlists(.importing(filename: "Fjord Robotics shortlist.xlsx"))))
        entries.append(GlanceEntry(date: now.addingTimeInterval(23), glance: .notStarted))
        entries.append(GlanceEntry(date: now.addingTimeInterval(24), glance: .needsRefresh))
        entries.append(GlanceEntry(date: now.addingTimeInterval(25), glance: .sample(now: now)))
        return entries
    }
}

#Preview("Small", as: .systemSmall) {
    ShortlistWidget()
} timeline: {
    for entry in PreviewTimeline.all { entry }
}

#Preview("Medium", as: .systemMedium) {
    ShortlistWidget()
} timeline: {
    for entry in PreviewTimeline.all { entry }
}

#Preview("Large", as: .systemLarge) {
    ShortlistWidget()
} timeline: {
    for entry in PreviewTimeline.all { entry }
}

#Preview("Extra large", as: .systemExtraLarge) {
    ShortlistWidget()
} timeline: {
    for entry in PreviewTimeline.all { entry }
}
#endif
