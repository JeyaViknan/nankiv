import Foundation

/// When a shortlist arrived, in the words a glance wants.
///
/// Absolute rather than relative: "Today, 14:05" is true for the rest of the
/// day, while "3 hours ago" is wrong a minute later unless the widget redraws
/// constantly. The labels change only at midnight, which is exactly what
/// `Schedule` plans entries for.
public struct Freshness: Sendable {
    public let calendar: Calendar
    public let locale: Locale

    public init(calendar: Calendar = .autoupdatingCurrent, locale: Locale = .autoupdatingCurrent) {
        var calendar = calendar
        calendar.locale = locale
        self.calendar = calendar
        self.locale = locale
    }

    /// "Today, 14:05", "Yesterday", "Monday", "12 Sep".
    public func label(for date: Date, now: Date) -> String {
        switch age(of: date, now: now) {
        case .today: "Today, \(time(date))"
        case .yesterday: "Yesterday"
        case .thisWeek: date.formatted(style.weekday(.wide))
        case .earlier: date.formatted(dayMonth(date, now: now))
        }
    }

    /// "14:05", "Yesterday", "Mon", "12 Sep" — for rows in a list.
    public func compactLabel(for date: Date, now: Date) -> String {
        switch age(of: date, now: now) {
        case .today: time(date)
        case .yesterday: "Yesterday"
        case .thisWeek: date.formatted(style.weekday(.abbreviated))
        case .earlier: date.formatted(dayMonth(date, now: now))
        }
    }

    enum Age { case today, yesterday, thisWeek, earlier }

    func age(of date: Date, now: Date) -> Age {
        // A date slightly in the future is clock skew between the app writing
        // and the widget reading, not a shortlist from tomorrow.
        let days = calendar.dateComponents(
            [.day], from: calendar.startOfDay(for: date), to: calendar.startOfDay(for: now)
        ).day ?? 0
        switch days {
        case ...0: return .today
        case 1: return .yesterday
        case 2...6: return .thisWeek
        default: return .earlier
        }
    }

    private var style: Date.FormatStyle {
        Date.FormatStyle(locale: locale, calendar: calendar, timeZone: calendar.timeZone)
    }

    private func time(_ date: Date) -> String {
        date.formatted(style.hour().minute())
    }

    private func dayMonth(_ date: Date, now: Date) -> Date.FormatStyle {
        let sameYear = calendar.component(.year, from: date) == calendar.component(.year, from: now)
        return sameYear ? style.day().month(.abbreviated) : style.day().month(.abbreviated).year()
    }
}
