import NankivWidgetModel
import SwiftUI
import WidgetKit

/// Draws a glance at the size it was given.
///
/// Each size answers a different question, not the same one bigger:
///
/// - **Small** — Did I make it? The answer, the company, and when it arrived.
/// - **Medium** — …and what does it tell me? The answer beside the one insight
///   that matters most: the estimated cutoff, or failing that, your circle.
/// - **Large** — The whole story of one shortlist: answer, analysis, circle.
/// - **Extra large** — The season: this shortlist in full, beside your circle
///   and every recent shortlist, each a link to its own analysis.
public struct GlanceView: View {
    let glance: Glance
    let family: WidgetFamily
    let now: Date

    public init(glance: Glance, family: WidgetFamily, now: Date) {
        self.glance = glance
        self.family = family
        self.now = now
    }

    public var body: some View {
        switch glance {
        case .shortlist(let shortlist):
            switch family {
            case .systemMedium: MediumShortlist(glance: shortlist, now: now)
            case .systemLarge: LargeShortlist(glance: shortlist, now: now)
            case .systemExtraLarge: ExtraLargeShortlist(glance: shortlist, now: now)
            default: SmallShortlist(glance: shortlist, now: now)
            }
        default:
            MessageView(glance: glance, family: family)
        }
    }
}

/// The widget's background, shared by the extension and the render harness so
/// both draw the same thing. WidgetKit removes it where the system supplies its
/// own — vibrant on the desktop, and accented appearances.
public struct GlanceBackground: View {
    public init() {}

    public var body: some View {
        Rectangle().fill(.background)
    }
}

// MARK: - Small

struct SmallShortlist: View {
    let glance: ShortlistGlance
    let now: Date

    @Environment(\.locale) private var locale
    @Environment(\.calendar) private var calendar

    var body: some View {
        let copy = Copy(locale: locale)
        let freshness = Freshness(calendar: calendar, locale: locale)
        let drive = glance.drive

        VStack(alignment: .leading, spacing: 0) {
            VerdictGlyph(verdict: drive.verdict, size: 30)

            Spacer(minLength: 6)

            Text(copy.title(drive.verdict))
                .font(.title.weight(.bold))
                .lineLimit(1)
                .minimumScaleFactor(0.7)
                .sensitive()
                .accessibilityLabel(copy.spoken(drive.verdict, company: drive.company))

            HStack(spacing: 5) {
                Text(drive.company)
                    .font(.headline)
                    .lineLimit(1)
                // Beside the name it belongs to, rather than floating in a corner.
                if glance.isPinned { PinMark() }
            }
            .accessibilityHidden(true)

            Group {
                if let notice = glance.notice {
                    NoticeLine(notice: notice, compact: true)
                } else if case .undetermined = drive.verdict {
                    Text(copy.shortDetail(drive.verdict, totalStudents: drive.totalStudents))
                        .foregroundStyle(.secondary)
                } else {
                    Text(freshness.label(for: drive.importedAt, now: now))
                        .foregroundStyle(.secondary)
                }
            }
            .font(.subheadline)
            .lineLimit(1)
            .padding(.top, 2)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
    }
}

// MARK: - Medium

struct MediumShortlist: View {
    let glance: ShortlistGlance
    let now: Date

    var body: some View {
        HStack(alignment: .top, spacing: 16) {
            SmallShortlist(glance: glance, now: now)
                .frame(maxWidth: .infinity)
            Insight(glance: glance)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        }
    }

    /// The one thing worth saying beside the answer.
    struct Insight: View {
        let glance: ShortlistGlance

        var body: some View {
            let drive = glance.drive
            if drive.analysis.status == .ready {
                AnalysisBlock(drive: drive, minChartHeight: 36, showsAxis: false)
                    .frame(maxHeight: .infinity, alignment: .top)
            } else if drive.circle.total > 0 {
                CircleBlock(circle: drive.circle, rows: 3, columns: 1)
            } else {
                VStack(alignment: .leading, spacing: 12) {
                    AnalysisBlock(drive: drive, minChartHeight: 0, showsAxis: false)
                    Spacer(minLength: 0)
                    SeasonTally(season: glance.season)
                }
            }
        }
    }
}

// MARK: - Large

extension DrivePreview {
    var hasChart: Bool { analysis.status == .ready && !analysis.histogram.isEmpty }
}

struct LargeShortlist: View {
    let glance: ShortlistGlance
    let now: Date

    var body: some View {
        let drive = glance.drive
        VStack(alignment: .leading, spacing: 12) {
            Headline(glance: glance, now: now, glyphSize: 34, titleFont: .title.weight(.bold))

            if drive.hasChart {
                AnalysisBlock(drive: drive, minChartHeight: 56, showsAxis: true)
                    .frame(maxHeight: .infinity, alignment: .top)
                if drive.circle.total > 0 {
                    CircleBlock(circle: drive.circle, rows: 2, columns: 2)
                } else {
                    SeasonTally(season: glance.season)
                }
            } else {
                // No chart to give the space to, so the circle takes it and the
                // reason there is no analysis sits quietly below.
                if drive.circle.total > 0 {
                    CircleBlock(circle: drive.circle, rows: 3, columns: 2)
                }
                Spacer(minLength: 0)
                AnalysisNote(drive: drive)
                SeasonTally(season: glance.season)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }
}

// MARK: - Extra large

struct ExtraLargeShortlist: View {
    let glance: ShortlistGlance
    let now: Date

    var body: some View {
        let drive = glance.drive
        HStack(alignment: .top, spacing: 24) {
            VStack(alignment: .leading, spacing: 12) {
                Headline(glance: glance, now: now, glyphSize: 38, titleFont: .largeTitle.weight(.bold))
                if drive.hasChart {
                    AnalysisBlock(drive: drive, minChartHeight: 80, showsAxis: true)
                        .frame(maxHeight: .infinity, alignment: .top)
                } else {
                    if drive.circle.total > 0 {
                        CircleBlock(circle: drive.circle, rows: 3, columns: 2)
                            .padding(.top, 4)
                    }
                    Spacer(minLength: 0)
                    AnalysisNote(drive: drive)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)

            VStack(alignment: .leading, spacing: 16) {
                let circleOnRight = drive.hasChart && drive.circle.total > 0
                if circleOnRight {
                    CircleBlock(circle: drive.circle, rows: 3, columns: 2)
                }
                if !glance.others.isEmpty {
                    RecentList(drives: Array(glance.others.prefix(circleOnRight ? 4 : 8)), now: now)
                }
                Spacer(minLength: 0)
                SeasonTally(season: glance.season)
            }
            .frame(width: 280)
            .frame(maxHeight: .infinity, alignment: .topLeading)
        }
    }
}

/// Company and date, any notice, and the answer — the top of every large layout.
struct Headline: View {
    let glance: ShortlistGlance
    let now: Date
    let glyphSize: CGFloat
    let titleFont: Font

    @Environment(\.locale) private var locale

    var body: some View {
        let drive = glance.drive
        VStack(alignment: .leading, spacing: 10) {
            DriveHeader(glance: glance, now: now)
            if let notice = glance.notice {
                NoticeLine(notice: notice, compact: false)
            }
            AnswerStack(
                drive: drive, glyphSize: glyphSize, titleFont: titleFont,
                detail: Copy(locale: locale).detail(drive.verdict, totalStudents: drive.totalStudents))
        }
    }
}

// MARK: - Shared

struct DriveHeader: View {
    let glance: ShortlistGlance
    let now: Date

    @Environment(\.locale) private var locale
    @Environment(\.calendar) private var calendar

    var body: some View {
        let freshness = Freshness(calendar: calendar, locale: locale)
        HStack(alignment: .firstTextBaseline, spacing: 6) {
            Text(glance.drive.company)
                .font(.headline)
                .lineLimit(1)
            if glance.isPinned { PinMark() }
            Spacer(minLength: 8)
            Text(freshness.label(for: glance.drive.importedAt, now: now))
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .layoutPriority(1)
        }
        .accessibilityElement(children: .combine)
    }
}

/// Marks a widget showing a chosen shortlist rather than the latest, so a
/// student is never misled into thinking an old result is new.
struct PinMark: View {
    var body: some View {
        Image(systemName: "pin.fill")
            .font(.subheadline)
            .foregroundStyle(.tertiary)
            .accessibilityLabel("Pinned")
    }
}

// MARK: - Whole-widget messages

struct MessageView: View {
    let glance: Glance
    let family: WidgetFamily

    @Environment(\.locale) private var locale

    var body: some View {
        let message = Copy(locale: locale).message(for: glance)
        if family == .systemSmall {
            VStack(alignment: .leading, spacing: 2) {
                symbol.font(.system(size: 26, weight: .medium))
                Spacer(minLength: 6)
                title(message)
                detail(message, lines: 3)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
            .accessibilityElement(children: .combine)
        } else {
            VStack(spacing: 0) {
                Text("nankiv")
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .accessibilityHidden(true)
                Spacer(minLength: 0)
                VStack(spacing: 6) {
                    symbol.font(.system(size: family == .systemMedium ? 26 : 34, weight: .medium))
                    title(message)
                    detail(message, lines: 2)
                }
                .multilineTextAlignment(.center)
                .frame(maxWidth: 300)
                .accessibilityElement(children: .combine)
                Spacer(minLength: 0)
                // Balances the label above, so the message sits in the true centre.
                Text(" ").font(.subheadline).accessibilityHidden(true)
            }
        }
    }

    private var symbol: some View {
        Image(systemName: symbolName)
            .symbolRenderingMode(.hierarchical)
            .foregroundStyle(.secondary)
            .widgetAccentable()
            .accessibilityHidden(true)
    }

    private func title(_ message: Copy.Message?) -> some View {
        Text(message?.title ?? "")
            .font(.headline)
            .lineLimit(2)
    }

    private func detail(_ message: Copy.Message?, lines: Int) -> some View {
        Text(message?.body ?? "")
            .font(.subheadline)
            .foregroundStyle(.secondary)
            .lineLimit(lines)
            .truncationMode(.middle)
            .fixedSize(horizontal: false, vertical: true)
    }

    private var symbolName: String {
        switch glance {
        case .notStarted: "arrow.up.forward.app"
        case .needsRefresh: "arrow.clockwise"
        case .needsIdentity: "person.crop.circle.badge.questionmark"
        case .noShortlists(nil): "tray"
        case .noShortlists(.importing?): "arrow.down.circle"
        case .noShortlists(.failed?): "exclamationmark.triangle"
        case .removed: "pin.slash"
        case .shortlist: "checkmark.circle"
        }
    }
}
