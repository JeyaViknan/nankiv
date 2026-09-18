import Foundation

// The snapshot the app publishes for its widgets.
//
// This mirrors `WidgetSnapshot` in src-tauri/src/widget.rs and is pinned by the
// files in widgets/fixtures, which the Rust tests generate and the tests here
// decode. Nothing in this file interprets data — every verdict, count and
// cutoff was decided by the app. The only judgement made on this side is how
// to survive a value this version does not recognise, and that judgement always
// errs toward "unknown", never toward an answer.

public struct WidgetSnapshot: Decodable, Sendable, Equatable {
    /// The schema this widget understands. A snapshot with any other schema was
    /// written by a different version of nankiv and is not trusted.
    public static let supportedSchema = 1

    public let schema: Int
    public let generatedAt: Date
    public let identityConfigured: Bool
    public let activity: Activity
    public let season: Season
    /// When the newest result stops leading the widget. `nil` when there is
    /// nothing to lead with.
    public let leadsUntil: Date?
    /// Newest first.
    public let drives: [DrivePreview]

    public init(
        schema: Int = WidgetSnapshot.supportedSchema,
        generatedAt: Date,
        identityConfigured: Bool,
        activity: Activity,
        season: Season,
        leadsUntil: Date? = nil,
        drives: [DrivePreview]
    ) {
        self.schema = schema
        self.generatedAt = generatedAt
        self.identityConfigured = identityConfigured
        self.activity = activity
        self.season = season
        self.leadsUntil = leadsUntil
        self.drives = drives
    }
}

public enum Activity: Decodable, Sendable, Equatable {
    case idle
    /// Shown until `until`, after which the app is assumed to have stopped.
    case importing(filename: String, since: Date, until: Date)
    /// Shown until `until`, or until an import succeeds.
    case failed(filename: String, at: Date, until: Date)

    private enum CodingKeys: String, CodingKey {
        case state, filename, since, until, at
    }

    public init(from decoder: any Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        switch try c.decode(String.self, forKey: .state) {
        case "importing":
            self = .importing(
                filename: try c.decode(String.self, forKey: .filename),
                since: try c.decode(Date.self, forKey: .since),
                until: try c.decode(Date.self, forKey: .until)
            )
        case "failed":
            self = .failed(
                filename: try c.decode(String.self, forKey: .filename),
                at: try c.decode(Date.self, forKey: .at),
                until: try c.decode(Date.self, forKey: .until)
            )
        default:
            // Something newer than this widget knows about. Showing the data
            // without a status line is better than showing nothing.
            self = .idle
        }
    }
}

public struct Season: Decodable, Sendable, Equatable {
    public let drives: Int
    public let shortlisted: Int
    public let notShortlisted: Int
    public let undetermined: Int

    public init(drives: Int, shortlisted: Int, notShortlisted: Int, undetermined: Int) {
        self.drives = drives
        self.shortlisted = shortlisted
        self.notShortlisted = notShortlisted
        self.undetermined = undetermined
    }
}

public struct DrivePreview: Decodable, Sendable, Equatable, Identifiable {
    public let id: Int64
    public let company: String
    public let importedAt: Date
    public let totalStudents: Int
    public let verdict: Verdict
    public let circle: CircleSummary
    public let analysis: AnalysisSummary

    public init(
        id: Int64,
        company: String,
        importedAt: Date,
        totalStudents: Int,
        verdict: Verdict,
        circle: CircleSummary,
        analysis: AnalysisSummary
    ) {
        self.id = id
        self.company = company
        self.importedAt = importedAt
        self.totalStudents = totalStudents
        self.verdict = verdict
        self.circle = circle
        self.analysis = analysis
    }
}

/// Three answers, never two. There is deliberately no `Bool` view of this.
public enum Verdict: Decodable, Sendable, Equatable {
    case shortlisted
    case notShortlisted
    case undetermined(UndeterminedReason)

    private enum CodingKeys: String, CodingKey { case status, reason }

    public init(from decoder: any Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        let reason = try c.decodeIfPresent(String.self, forKey: .reason)
        self.init(status: try c.decode(String.self, forKey: .status), reason: reason)
    }

    init(status: String, reason: String?) {
        switch status {
        case "shortlisted": self = .shortlisted
        case "not_shortlisted": self = .notShortlisted
        default:
            // Includes any status this version does not recognise: an unknown
            // word is never read as a rejection.
            self = .undetermined(reason.flatMap(UndeterminedReason.init(rawValue:)) ?? .other)
        }
    }
}

public enum UndeterminedReason: String, Sendable, Equatable {
    case noIdentityConfigured = "no_identity_configured"
    case needsNeoId = "needs_neo_id"
    case needsRegNo = "needs_reg_no"
    case fileNotUnderstood = "file_not_understood"
    case other
}

public struct CircleSummary: Decodable, Sendable, Equatable {
    public let total: Int
    public let shortlisted: Int
    public let notShortlisted: Int
    public let undetermined: Int
    /// In the app's order: shortlisted, then undetermined, then not shortlisted.
    public let members: [Member]

    public init(total: Int, shortlisted: Int, notShortlisted: Int, undetermined: Int, members: [Member]) {
        self.total = total
        self.shortlisted = shortlisted
        self.notShortlisted = notShortlisted
        self.undetermined = undetermined
        self.members = members
    }

    public static let none = CircleSummary(total: 0, shortlisted: 0, notShortlisted: 0, undetermined: 0, members: [])

    public struct Member: Decodable, Sendable, Equatable {
        public let label: String
        public let verdict: Verdict

        public init(label: String, verdict: Verdict) {
            self.label = label
            self.verdict = verdict
        }

        private enum CodingKeys: String, CodingKey { case label, status }

        public init(from decoder: any Decoder) throws {
            let c = try decoder.container(keyedBy: CodingKeys.self)
            label = try c.decode(String.self, forKey: .label)
            verdict = Verdict(status: try c.decode(String.self, forKey: .status), reason: nil)
        }
    }
}

public struct AnalysisSummary: Decodable, Sendable, Equatable {
    public enum Status: String, Sendable, Equatable {
        case ready
        case insufficient
        case notSetUp = "not_set_up"
    }

    public let status: Status
    public let matched: Int
    public let coverage: Double
    /// Present only when `status` is `ready`.
    public let cutoff: Cutoff?
    public let median: Double?
    /// Empty buckets at either end already trimmed.
    public let histogram: [Bucket]
    public let overRepresented: [String]

    public init(
        status: Status,
        matched: Int,
        coverage: Double,
        cutoff: Cutoff?,
        median: Double?,
        histogram: [Bucket],
        overRepresented: [String]
    ) {
        self.status = status
        self.matched = matched
        self.coverage = coverage
        self.cutoff = status == .ready ? cutoff : nil
        self.median = status == .ready ? median : nil
        self.histogram = status == .ready ? histogram : []
        self.overRepresented = status == .ready ? overRepresented : []
    }

    private enum CodingKeys: String, CodingKey {
        case status, matched, coverage, cutoff, median, histogram, overRepresented
    }

    public init(from decoder: any Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        let raw = try c.decode(String.self, forKey: .status)
        self.init(
            // An unfamiliar status shows no statistics rather than guessed ones.
            status: Status(rawValue: raw) ?? .insufficient,
            matched: try c.decode(Int.self, forKey: .matched),
            coverage: try c.decode(Double.self, forKey: .coverage),
            // A cutoff this version cannot read is left out, not allowed to
            // fail the whole snapshot.
            cutoff: (try? c.decodeIfPresent(Cutoff.self, forKey: .cutoff)) ?? nil,
            median: try c.decodeIfPresent(Double.self, forKey: .median),
            histogram: try c.decodeIfPresent([Bucket].self, forKey: .histogram) ?? [],
            overRepresented: try c.decodeIfPresent([String].self, forKey: .overRepresented) ?? []
        )
    }
}

/// Every case is an estimate. The widget labels it as one wherever it appears,
/// exactly as the app does: a pattern in who was shortlisted is not a rule the
/// company published.
public enum Cutoff: Sendable, Equatable {
    case around(threshold: Double, observedFloor: Double)
    case skewsHigh(observedFloor: Double)
    case noFilter
}

extension Cutoff: Decodable {
    private enum CodingKeys: String, CodingKey { case kind, threshold, observedFloor }

    public init(from decoder: any Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        let threshold = try c.decodeIfPresent(Double.self, forKey: .threshold)
        let floor = try c.decodeIfPresent(Double.self, forKey: .observedFloor)
        switch (try c.decode(String.self, forKey: .kind), threshold, floor) {
        case ("hard_cutoff", let t?, let f?): self = .around(threshold: t, observedFloor: f)
        case ("soft_preference", _, let f?): self = .skewsHigh(observedFloor: f)
        case ("no_cgpa_filter", _, _): self = .noFilter
        default:
            throw DecodingError.dataCorruptedError(
                forKey: .kind, in: c, debugDescription: "unrecognised cutoff")
        }
    }
}

public struct Bucket: Decodable, Sendable, Equatable {
    public let lower: Double
    public let upper: Double
    public let count: Int

    public init(lower: Double, upper: Double, count: Int) {
        self.lower = lower
        self.upper = upper
        self.count = count
    }
}
