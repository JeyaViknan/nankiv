import Foundation

/// Every sentence the widget shows.
///
/// Kept apart from the views so the wording can be tested, and so it can stay
/// word-for-word consistent with the app: "You're in" means the same thing on
/// the desktop as it does on the drive screen, and an estimate is called an
/// estimate in both places.
public struct Copy: Sendable {
    public let locale: Locale

    public init(locale: Locale = .autoupdatingCurrent) {
        self.locale = locale
    }

    // MARK: Verdicts

    public func title(_ verdict: Verdict) -> String {
        switch verdict {
        case .shortlisted: "You're in"
        case .notShortlisted: "Not this time"
        case .undetermined: "Can't tell"
        }
    }

    /// One word, for lists where the glyph carries most of the meaning.
    public func status(_ verdict: Verdict) -> String {
        switch verdict {
        case .shortlisted: "In"
        case .notShortlisted: "Not in"
        case .undetermined: "Unknown"
        }
    }

    public func detail(_ verdict: Verdict, totalStudents: Int) -> String {
        switch verdict {
        case .shortlisted:
            "\(count(totalStudents)) students shortlisted"
        case .notShortlisted:
            "Not among the \(count(totalStudents)) shortlisted"
        case .undetermined(let reason):
            switch reason {
            case .needsRegNo: "This list uses registration numbers. Add yours in nankiv."
            case .needsNeoId: "This list uses Neo IDs. Add yours in nankiv."
            case .fileNotUnderstood: "This file has no IDs to match you against."
            case .noIdentityConfigured: "Add your Neo ID in nankiv."
            case .other: "Open nankiv for details."
            }
        }
    }

    /// The same, in the space a small widget has.
    public func shortDetail(_ verdict: Verdict, totalStudents: Int) -> String {
        switch verdict {
        case .shortlisted, .notShortlisted:
            "\(count(totalStudents)) shortlisted"
        case .undetermined(let reason):
            switch reason {
            case .needsRegNo: "Needs your reg. number"
            case .needsNeoId: "Needs your Neo ID"
            case .fileNotUnderstood: "No IDs in this file"
            case .noIdentityConfigured: "Needs your Neo ID"
            case .other: "Open nankiv for details"
            }
        }
    }

    /// Spoken by VoiceOver in place of the title, which leans on the glyph.
    public func spoken(_ verdict: Verdict, company: String) -> String {
        switch verdict {
        case .shortlisted: "You're on the \(company) shortlist"
        case .notShortlisted: "You're not on the \(company) shortlist"
        case .undetermined: "Can't tell whether you're on the \(company) shortlist"
        }
    }

    // MARK: Analysis

    public func headline(_ analysis: AnalysisSummary) -> String {
        switch analysis.status {
        case .insufficient: return "Not enough data to analyse"
        case .notSetUp: return "Analysis isn't set up"
        case .ready:
            switch analysis.cutoff {
            case .around(let threshold, _): return "CGPA cutoff around \(cgpa(threshold, digits: 1))"
            case .skewsHigh: return "Skews high, no hard cutoff"
            case .noFilter, nil: return "No CGPA filter detected"
            }
        }
    }

    /// The qualifier that always travels with a cutoff.
    public func basis(_ analysis: AnalysisSummary, totalStudents: Int) -> String {
        switch analysis.status {
        case .ready:
            analysis.matched >= totalStudents
                ? "Estimate from all \(count(totalStudents)) students"
                : "Estimate from \(count(analysis.matched)) of \(count(totalStudents)) students"
        case .insufficient:
            "Matched \(count(analysis.matched)) of \(count(totalStudents)) students"
        case .notSetUp:
            "Open nankiv to add CGPA data."
        }
    }

    public func median(_ value: Double) -> String {
        "Median \(cgpa(value, digits: 2))"
    }

    public func cgpa(_ value: Double, digits: Int) -> String {
        value.formatted(.number.precision(.fractionLength(digits)).locale(locale))
    }

    // MARK: Circle and season

    public let circleTitle = "Your circle"
    /// The resting widget: no fresh result, and an invitation to drop the next.
    public let restingTitle = "No new shortlist"
    public let restingInvitation = "Drop the next one into nankiv"
    /// The same invitation where a small widget has room for four words.
    public let restingInvitationShort = "Drop the next one"

    /// "Last: Aurora Systems · You're in"
    public func lastResult(_ drive: DrivePreview) -> String {
        "Last: \(drive.company) · \(status(drive.verdict))"
    }
    public let recentTitle = "Recent shortlists"
    public let seasonTitle = "This season"

    public func circleSummary(_ circle: CircleSummary) -> String {
        "\(count(circle.shortlisted)) of \(count(circle.total)) in"
    }

    public func moreMembers(_ n: Int) -> String {
        "+\(count(n)) more"
    }

    public func seasonSummary(_ season: Season) -> String {
        season.drives == 1 ? "1 shortlist" : "\(count(season.drives)) shortlists"
    }

    // MARK: Notices

    public func notice(_ notice: Notice) -> String {
        switch notice {
        case .importing(let filename): "Importing \(filename)…"
        case .failed(let filename): "Couldn't import \(filename)"
        }
    }

    public func shortNotice(_ notice: Notice) -> String {
        switch notice {
        case .importing: "Importing…"
        case .failed: "Last import failed"
        }
    }

    // MARK: Whole-widget states

    public struct Message: Sendable, Equatable {
        public let title: String
        public let body: String
    }

    public func message(for glance: Glance) -> Message? {
        switch glance {
        case .shortlist, .resting:
            nil
        case .notStarted:
            Message(title: "Open nankiv", body: "Your latest shortlist result will appear here.")
        case .needsRefresh:
            Message(title: "Open nankiv", body: "Open the app to bring this widget up to date.")
        case .needsIdentity:
            Message(title: "Finish setting up", body: "Add your Neo ID in nankiv to see your results here.")
        case .noShortlists(nil):
            Message(title: "No shortlists yet", body: "Drop a shortlist into nankiv and your result appears here.")
        case .noShortlists(.importing(let filename)?):
            Message(title: "Importing…", body: filename)
        case .noShortlists(.failed(let filename)?):
            Message(title: "Couldn't import", body: "\(filename) — open nankiv to see why.")
        case .removed:
            Message(title: "Shortlist removed", body: "Edit this widget to choose another.")
        }
    }

    // MARK: Numbers

    public func count(_ n: Int) -> String {
        n.formatted(.number.locale(locale))
    }
}
