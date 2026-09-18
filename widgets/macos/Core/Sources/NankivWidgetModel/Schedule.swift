import Foundation

/// When a widget's timeline needs entries, and when WidgetKit should ask again.
///
/// The widget never polls. The app reloads its timelines whenever the snapshot
/// changes; the timeline only has to cover changes that happen with the
/// passage of time and nothing else:
///
/// - a transient state expiring ("Importing…" after the app stopped),
/// - the newest result finishing its turn at the top, and
/// - date labels rolling over at midnight ("Today" becomes "Yesterday").
///
/// Labels settle into plain dates after a week, so a week of midnights covers
/// every change, and the one reload at the end is a safety net rather than a
/// schedule.
public struct TimelinePlan: Sendable, Equatable {
    /// Entry dates, ascending, starting with now.
    public let moments: [Date]
    /// When WidgetKit should build a new timeline if the app has not asked first.
    public let reload: Date
}

public enum Schedule {
    public static let midnights = 8

    /// How long to wait before looking again when there is no snapshot to plan
    /// around. The app reloads the widget the moment it writes one; this only
    /// matters if that request is lost.
    public static let retryWithoutSnapshot: TimeInterval = 60 * 60

    public static func plan(for load: SnapshotLoad, from now: Date, calendar: Calendar = .current) -> TimelinePlan {
        guard case .loaded(let snapshot) = load else {
            return TimelinePlan(moments: [now], reload: now.addingTimeInterval(retryWithoutSnapshot))
        }

        var moments: Set<Date> = [now]
        // The moment the newest result stops leading.
        if let leadsUntil = snapshot.leadsUntil, leadsUntil > now { moments.insert(leadsUntil) }
        switch snapshot.activity {
        case .importing(_, _, let until), .failed(_, _, let until):
            if until > now { moments.insert(until) }
        case .idle:
            break
        }

        var day = calendar.startOfDay(for: now)
        for _ in 0..<midnights {
            guard let next = calendar.date(byAdding: .day, value: 1, to: day) else { break }
            moments.insert(next)
            day = next
        }

        let sorted = moments.sorted()
        return TimelinePlan(moments: sorted, reload: sorted.last ?? now)
    }
}
