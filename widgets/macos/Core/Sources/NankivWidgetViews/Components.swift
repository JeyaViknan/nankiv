import NankivWidgetModel
import SwiftUI
import WidgetKit

// The pieces every size is assembled from.
//
// Two rules hold throughout, because the widget is drawn three different ways:
// in full colour; vibrant, where macOS desaturates it against the wallpaper; and
// accented, where the system tints it and removes the background.
//
//   - Nothing relies on colour. Every answer has its own glyph shape and its own
//     words, so a desaturated or tinted widget reads exactly as a coloured one.
//   - Colour is used only where it means something: green for the answer people
//     hope for, orange for "can't tell". A rejection is deliberately neutral,
//     as it is in the app.

// MARK: - Verdict glyph

extension Verdict {
    var symbol: String {
        switch self {
        case .shortlisted: "checkmark.circle.fill"
        case .notShortlisted: "minus.circle.fill"
        case .undetermined: "questionmark.circle.fill"
        }
    }
}

struct VerdictGlyph: View {
    let verdict: Verdict
    let size: CGFloat

    @Environment(\.widgetRenderingMode) private var renderingMode

    var body: some View {
        glyph
            .font(.system(size: size, weight: .semibold))
            .imageScale(.medium)
            .widgetAccentable()
            .accessibilityHidden(true)
    }

    @ViewBuilder private var glyph: some View {
        let image = Image(systemName: verdict.symbol)
        if renderingMode == .fullColor {
            switch verdict {
            case .shortlisted:
                image.symbolRenderingMode(.palette).foregroundStyle(.white, .green)
            case .notShortlisted:
                image.symbolRenderingMode(.palette).foregroundStyle(.secondary, .quaternary)
            case .undetermined:
                image.symbolRenderingMode(.palette).foregroundStyle(.white, .orange)
            }
        } else {
            // Without colour, hierarchy does the work: a soft disc, a solid mark.
            image.symbolRenderingMode(.hierarchical).foregroundStyle(.primary)
        }
    }
}

/// A small glyph for rows, tinted only in full colour.
struct StatusDot: View {
    let verdict: Verdict

    @Environment(\.widgetRenderingMode) private var renderingMode

    var body: some View {
        Image(systemName: verdict.symbol)
            .font(.subheadline.weight(.semibold))
            .symbolRenderingMode(.hierarchical)
            .foregroundStyle(tint)
            .accessibilityHidden(true)
    }

    private var tint: AnyShapeStyle {
        guard renderingMode == .fullColor else { return AnyShapeStyle(.secondary) }
        switch verdict {
        case .shortlisted: return AnyShapeStyle(.green)
        case .notShortlisted: return AnyShapeStyle(.tertiary)
        case .undetermined: return AnyShapeStyle(.orange)
        }
    }
}

// MARK: - Answer

/// The answer, at the size of a headline. Its accessibility label is a full
/// sentence, because "You're in" alone does not say in what.
struct AnswerStack: View {
    let drive: DrivePreview
    let glyphSize: CGFloat
    let titleFont: Font
    let detail: String?

    @Environment(\.locale) private var locale

    var body: some View {
        let copy = Copy(locale: locale)
        HStack(alignment: .center, spacing: 10) {
            VerdictGlyph(verdict: drive.verdict, size: glyphSize)
            VStack(alignment: .leading, spacing: 1) {
                Text(copy.title(drive.verdict))
                    .font(titleFont)
                    .lineLimit(1)
                    .minimumScaleFactor(0.75)
                    .sensitive()
                if let detail {
                    Text(detail)
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                        .lineLimit(2)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(copy.spoken(drive.verdict, company: drive.company))
        .accessibilityValue(detail ?? "")
    }
}

// MARK: - Notice

struct NoticeLine: View {
    let notice: Notice
    let compact: Bool

    @Environment(\.locale) private var locale
    @Environment(\.widgetRenderingMode) private var renderingMode

    var body: some View {
        let copy = Copy(locale: locale)
        Label {
            Text(compact ? copy.shortNotice(notice) : copy.notice(notice))
                .lineLimit(1)
                .truncationMode(.middle)
        } icon: {
            Image(systemName: symbol)
        }
        .font(.subheadline.weight(.medium))
        .foregroundStyle(tint)
        .accessibilityLabel(copy.notice(notice))
    }

    private var symbol: String {
        switch notice {
        case .importing: "arrow.down.circle"
        case .failed: "exclamationmark.triangle.fill"
        }
    }

    private var tint: AnyShapeStyle {
        if case .failed = notice, renderingMode == .fullColor { return AnyShapeStyle(.orange) }
        return AnyShapeStyle(.secondary)
    }
}

// MARK: - Distribution

/// The CGPAs on the shortlist, as bars, with the estimated cutoff marked.
///
/// When there is a cutoff the axis starts one bucket below it, so the empty
/// space under the bar is visible — that gap is the whole finding.
struct Histogram: View {
    let analysis: AnalysisSummary
    let showsAxis: Bool

    @Environment(\.locale) private var locale
    @Environment(\.redactionReasons) private var redactionReasons

    private var buckets: [Bucket] { analysis.histogram }

    /// Loading placeholders draw neutral shapes, never a coloured result.
    private var barFill: AnyShapeStyle {
        redactionReasons.contains(.placeholder) ? AnyShapeStyle(.quaternary) : AnyShapeStyle(Color.accentColor.gradient)
    }

    private var domain: ClosedRange<Double> {
        guard let first = buckets.first, let last = buckets.last else { return 0...1 }
        var lower = first.lower
        if case .around(let threshold, _) = analysis.cutoff {
            lower = min(lower, threshold - (first.upper - first.lower))
        }
        return lower...max(last.upper, lower + 0.01)
    }

    var body: some View {
        let copy = Copy(locale: locale)
        VStack(spacing: 3) {
            GeometryReader { proxy in
                let size = proxy.size
                let span = domain.upperBound - domain.lowerBound
                let tallest = CGFloat(buckets.map(\.count).max() ?? 1)
                let x = { (v: Double) in CGFloat((v - domain.lowerBound) / span) * size.width }

                ZStack(alignment: .bottomLeading) {
                    // Baseline.
                    Rectangle()
                        .fill(.quaternary)
                        .frame(height: 1)

                    ForEach(Array(buckets.enumerated()), id: \.offset) { _, bucket in
                        let slot = x(bucket.upper) - x(bucket.lower)
                        // Bars narrower than their bucket, with room between,
                        // read as a distribution rather than a block.
                        let width = max(min(slot * 0.72, 30), 2)
                        let height = max(CGFloat(bucket.count) / tallest * size.height, bucket.count > 0 ? 2 : 0)
                        UnevenRoundedRectangle(topLeadingRadius: min(3, width / 2), topTrailingRadius: min(3, width / 2))
                            .fill(barFill)
                            .frame(width: width, height: height)
                            .offset(x: x(bucket.lower) + (slot - width) / 2)
                    }
                    .widgetAccentable()

                    if case .around(let threshold, _) = analysis.cutoff {
                        Path { p in
                            p.move(to: CGPoint(x: x(threshold), y: -2))
                            p.addLine(to: CGPoint(x: x(threshold), y: size.height))
                        }
                        .stroke(.primary.opacity(0.7), style: StrokeStyle(lineWidth: 1.25, lineCap: .round, dash: [2.5, 2.5]))
                    }
                }
                .frame(width: size.width, height: size.height, alignment: .bottomLeading)
            }

            if showsAxis {
                axis(copy)
                    .font(.subheadline)
                    .monospacedDigit()
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("CGPA spread on this shortlist")
        .accessibilityValue(accessibilityValue(copy))
    }

    /// Labels where they mean something: the cutoff under its marker, the
    /// highest CGPA at the end, and the median between. The padded start of the
    /// axis is not a real value, so it is not labelled.
    private func axis(_ copy: Copy) -> some View {
        GeometryReader { proxy in
            let span = domain.upperBound - domain.lowerBound
            let x = { (v: Double) in CGFloat((v - domain.lowerBound) / span) * proxy.size.width }
            ZStack(alignment: .topLeading) {
                if case .around(let threshold, _) = analysis.cutoff {
                    let marker = x(threshold)
                    Text(copy.cgpa(threshold, digits: 1))
                        .fixedSize()
                        .alignmentGuide(.leading) { d in d.width / 2 - marker }
                } else if let first = buckets.first {
                    Text(copy.cgpa(first.lower, digits: 1))
                        .fixedSize()
                }
                if let median = analysis.median {
                    Text(copy.median(median))
                        .fixedSize()
                        .frame(maxWidth: .infinity, alignment: .center)
                }
                Text(copy.cgpa(domain.upperBound, digits: 1))
                    .fixedSize()
                    .frame(maxWidth: .infinity, alignment: .trailing)
            }
        }
        .frame(height: 14)
    }

    private func accessibilityValue(_ copy: Copy) -> String {
        guard let first = buckets.first, let last = buckets.last else { return "" }
        var parts = ["From \(copy.cgpa(first.lower, digits: 1)) to \(copy.cgpa(last.upper, digits: 1))"]
        if let median = analysis.median { parts.append(copy.median(median)) }
        return parts.joined(separator: ". ")
    }
}

/// What the shortlist suggests, always labelled as an estimate.
struct AnalysisBlock: View {
    let drive: DrivePreview
    let minChartHeight: CGFloat
    let showsAxis: Bool

    @Environment(\.locale) private var locale

    var body: some View {
        let copy = Copy(locale: locale)
        let analysis = drive.analysis
        VStack(alignment: .leading, spacing: 6) {
            VStack(alignment: .leading, spacing: 1) {
                Text(copy.headline(analysis))
                    .font(.headline)
                    .lineLimit(2)
                    .fixedSize(horizontal: false, vertical: true)
                Text(copy.basis(analysis, totalStudents: drive.totalStudents))
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .minimumScaleFactor(0.85)
            }
            .accessibilityElement(children: .combine)

            if analysis.status == .ready, !analysis.histogram.isEmpty {
                // At least `minChartHeight`, and whatever else the layout can spare.
                Histogram(analysis: analysis, showsAxis: showsAxis)
                    .frame(minHeight: minChartHeight, maxHeight: .infinity)
            }
        }
    }
}

// MARK: - Circle

struct CircleBlock: View {
    let circle: CircleSummary
    let rows: Int
    let columns: Int

    @Environment(\.locale) private var locale

    var body: some View {
        let copy = Copy(locale: locale)
        let shown = Array(circle.members.prefix(rows * columns))
        VStack(alignment: .leading, spacing: 6) {
            SectionHeader(title: copy.circleTitle, trailing: copy.circleSummary(circle))

            Grid(alignment: .leading, horizontalSpacing: 12, verticalSpacing: 5) {
                ForEach(0..<Int((Double(shown.count) / Double(columns)).rounded(.up)), id: \.self) { row in
                    GridRow {
                        ForEach(0..<columns, id: \.self) { column in
                            let index = row * columns + column
                            if index < shown.count {
                                MemberRow(member: shown[index])
                            } else {
                                Color.clear.gridCellUnsizedAxes([.horizontal, .vertical])
                            }
                        }
                    }
                }
            }

            if circle.total > shown.count {
                Text(copy.moreMembers(circle.total - shown.count))
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
        }
    }
}

struct MemberRow: View {
    let member: CircleSummary.Member

    @Environment(\.locale) private var locale

    var body: some View {
        HStack(spacing: 6) {
            StatusDot(verdict: member.verdict)
            Text(member.label)
                .font(.body)
                .lineLimit(1)
                .sensitive()
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("\(member.label), \(Copy(locale: locale).status(member.verdict))")
    }
}

// MARK: - Season

struct SeasonTally: View {
    let season: Season

    @Environment(\.locale) private var locale

    var body: some View {
        let copy = Copy(locale: locale)
        VStack(alignment: .leading, spacing: 6) {
            SectionHeader(title: copy.seasonTitle, trailing: copy.seasonSummary(season))
            HStack(spacing: 14) {
                tally(.shortlisted, season.shortlisted, copy)
                tally(.notShortlisted, season.notShortlisted, copy)
                if season.undetermined > 0 {
                    tally(.undetermined(.other), season.undetermined, copy)
                }
            }
        }
    }

    private func tally(_ verdict: Verdict, _ n: Int, _ copy: Copy) -> some View {
        HStack(spacing: 4) {
            StatusDot(verdict: verdict)
            Text(copy.count(n))
                .font(.headline)
                .monospacedDigit()
            Text(copy.status(verdict))
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .lineLimit(1)
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("\(copy.count(n)) \(copy.status(verdict))")
    }
}

// MARK: - Recent

struct RecentList: View {
    let drives: [DrivePreview]
    let now: Date

    @Environment(\.locale) private var locale
    @Environment(\.calendar) private var calendar

    var body: some View {
        let copy = Copy(locale: locale)
        let freshness = Freshness(calendar: calendar, locale: locale)
        VStack(alignment: .leading, spacing: 6) {
            SectionHeader(title: copy.recentTitle, trailing: nil)
            ForEach(drives) { drive in
                // Each row opens its own shortlist; the rest of the widget
                // opens the one it features.
                GlanceLink(destination: DeepLink.drive(drive.id)) {
                    HStack(spacing: 6) {
                        StatusDot(verdict: drive.verdict)
                        Text(drive.company)
                            .font(.body)
                            .foregroundStyle(.primary)
                            .lineLimit(1)
                        Spacer(minLength: 6)
                        Text(freshness.compactLabel(for: drive.importedAt, now: now))
                            .font(.subheadline)
                            .monospacedDigit()
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                            .layoutPriority(1)
                    }
                    .accessibilityElement(children: .ignore)
                    .accessibilityLabel(
                        "\(drive.company), \(copy.status(drive.verdict)), \(freshness.label(for: drive.importedAt, now: now))")
                }
            }
        }
    }
}

/// A `Link` in the widget; its label alone anywhere a link cannot be drawn.
struct GlanceLink<Label: View>: View {
    let destination: URL
    @ViewBuilder let label: () -> Label

    @Environment(\.glanceDrawsLinksAsLabels) private var labelsOnly

    var body: some View {
        if labelsOnly {
            label()
        } else {
            Link(destination: destination, label: label)
        }
    }
}

extension EnvironmentValues {
    /// Set by the render harness, which draws outside a widget host.
    @Entry public var glanceDrawsLinksAsLabels = false
}

struct SectionHeader: View {
    let title: String
    let trailing: String?

    var body: some View {
        HStack(alignment: .firstTextBaseline) {
            Text(title)
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(.secondary)
            Spacer(minLength: 6)
            if let trailing {
                Text(trailing)
                    .font(.subheadline)
                    .monospacedDigit()
                    .foregroundStyle(.secondary)
            }
        }
        .lineLimit(1)
        .accessibilityElement(children: .combine)
        .accessibilityAddTraits(.isHeader)
    }
}

// MARK: - Privacy

extension View {
    /// Marks personal content — an answer, a friend's name — so the system can
    /// hide it where it hides private data.
    ///
    /// Privacy-sensitive views are exempt from placeholder redaction, so in a
    /// placeholder the mark is left off: a widget that is still loading must
    /// never show "You're in" in the clear.
    func sensitive() -> some View {
        modifier(Sensitive())
    }
}

private struct Sensitive: ViewModifier {
    @Environment(\.redactionReasons) private var reasons

    func body(content: Content) -> some View {
        if reasons.contains(.placeholder) {
            content
        } else {
            content.privacySensitive()
        }
    }
}

/// A one-line account of why there is no chart.
struct AnalysisNote: View {
    let drive: DrivePreview

    @Environment(\.locale) private var locale

    var body: some View {
        let copy = Copy(locale: locale)
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Image(systemName: "chart.bar.xaxis")
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 1) {
                Text(copy.headline(drive.analysis))
                    .font(.subheadline.weight(.semibold))
                Text(copy.basis(drive.analysis, totalStudents: drive.totalStudents))
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
            .lineLimit(1)
        }
        .accessibilityElement(children: .combine)
    }
}
