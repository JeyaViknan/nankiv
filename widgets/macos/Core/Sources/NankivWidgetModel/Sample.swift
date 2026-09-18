import Foundation

extension Glance {
    /// What the widget looks like, without anyone's data in it.
    ///
    /// Used where WidgetKit needs something to draw before a real timeline
    /// exists: the redacted placeholder while loading, and the widget gallery
    /// when nankiv has not yet published a snapshot. Invented throughout.
    public static func sample(now: Date = Date()) -> Glance {
        let buckets: [Bucket] = [
            (8.5, 8.75, 14), (8.75, 9.0, 22), (9.0, 9.25, 27), (9.25, 9.5, 19), (9.5, 9.75, 10), (9.75, 10.0, 4),
        ].map { Bucket(lower: $0.0, upper: $0.1, count: $0.2) }

        func preview(_ id: Int64, _ company: String, _ verdict: Verdict, daysAgo: Double) -> DrivePreview {
            DrivePreview(
                id: id,
                company: company,
                importedAt: now.addingTimeInterval(-daysAgo * 86_400 - 3_600),
                totalStudents: 96,
                verdict: verdict,
                circle: .none,
                analysis: AnalysisSummary(
                    status: .insufficient, matched: 0, coverage: 0, cutoff: nil, median: nil,
                    histogram: [], overRepresented: [])
            )
        }

        let latest = DrivePreview(
            id: 1,
            company: "Your latest shortlist",
            importedAt: now.addingTimeInterval(-3_600),
            totalStudents: 96,
            verdict: .shortlisted,
            circle: CircleSummary(
                total: 3, shortlisted: 2, notShortlisted: 1, undetermined: 0,
                members: [
                    .init(label: "Friend", verdict: .shortlisted),
                    .init(label: "Friend", verdict: .shortlisted),
                    .init(label: "Friend", verdict: .notShortlisted),
                ]),
            analysis: AnalysisSummary(
                status: .ready, matched: 96, coverage: 1,
                cutoff: .around(threshold: 8.5, observedFloor: 8.51), median: 9.05,
                histogram: buckets, overRepresented: [])
        )

        return .shortlist(
            ShortlistGlance(
                drive: latest,
                isPinned: false,
                notice: nil,
                season: Season(drives: 4, shortlisted: 2, notShortlisted: 1, undetermined: 1),
                others: [
                    preview(2, "Another company", .notShortlisted, daysAgo: 1),
                    preview(3, "Another company", .shortlisted, daysAgo: 2),
                    preview(4, "Another company", .undetermined(.needsRegNo), daysAgo: 4),
                ]
            )
        )
    }
}
