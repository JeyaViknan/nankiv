import Foundation

/// What a widget shows at a given moment.
///
/// Every state the widget can be in is a case here, decided in one place from
/// the snapshot, the widget's configuration and the time. The views only draw
/// a `Glance`; they never look at the snapshot themselves, so a state cannot be
/// handled at one size and forgotten at another.
public enum Glance: Sendable, Equatable {
    /// nankiv has never run, so there is nothing to show yet.
    case notStarted
    /// The snapshot is from another version of nankiv, or unreadable.
    case needsRefresh
    /// Nobody has told nankiv who they are, so every answer would be "can't tell".
    case needsIdentity
    /// Set up, but no shortlist imported — possibly one on its way.
    case noShortlists(Notice?)
    /// The widget was pinned to a shortlist that has since been deleted.
    case removed
    case shortlist(ShortlistGlance)

    /// Where a click takes you: the shortlist on screen, or the app's front page.
    public var destination: URL {
        switch self {
        case .shortlist(let s): DeepLink.drive(s.drive.id)
        default: DeepLink.shortlists
        }
    }
}

public struct ShortlistGlance: Sendable, Equatable {
    public let drive: DrivePreview
    /// Whether this is a shortlist the student chose, rather than the latest.
    public let isPinned: Bool
    public let notice: Notice?
    public let season: Season
    /// Every other shortlist in the snapshot, newest first.
    public let others: [DrivePreview]

    public init(drive: DrivePreview, isPinned: Bool, notice: Notice?, season: Season, others: [DrivePreview]) {
        self.drive = drive
        self.isPinned = isPinned
        self.notice = notice
        self.season = season
        self.others = others
    }
}

/// Something happening in the app that is worth a line on the widget.
public enum Notice: Sendable, Equatable {
    case importing(filename: String)
    case failed(filename: String)
}

extension Glance {
    /// - Parameters:
    ///   - pinned: The shortlist chosen in the widget's settings, or `nil` for
    ///     the latest.
    ///   - date: The moment the entry is for. Transient states are judged
    ///     against this, not the current time, so a timeline built now shows
    ///     "Importing" only for the entries before it expires.
    public static func make(from load: SnapshotLoad, pinned: Int64?, at date: Date) -> Glance {
        let snapshot: WidgetSnapshot
        switch load {
        case .missing: return .notStarted
        case .stale: return .needsRefresh
        case .loaded(let s): snapshot = s
        }

        guard snapshot.identityConfigured else { return .needsIdentity }

        let notice = Notice(snapshot.activity, at: date)
        let shown: DrivePreview
        if let pinned {
            guard let match = snapshot.drives.first(where: { $0.id == pinned }) else {
                return .removed
            }
            shown = match
        } else {
            guard let latest = snapshot.drives.first else { return .noShortlists(notice) }
            shown = latest
        }

        return .shortlist(
            ShortlistGlance(
                drive: shown,
                isPinned: pinned != nil,
                notice: notice,
                season: snapshot.season,
                others: snapshot.drives.filter { $0.id != shown.id }
            )
        )
    }
}

extension Notice {
    init?(_ activity: Activity, at date: Date) {
        switch activity {
        case .idle:
            return nil
        case .importing(let filename, _, let until):
            guard date < until else { return nil }
            self = .importing(filename: filename)
        case .failed(let filename, _, let until):
            guard date < until else { return nil }
            self = .failed(filename: filename)
        }
    }
}

/// `nankiv://` links. The formats must match `parse_route` in
/// src-tauri/src/widget.rs; widgets/fixtures/links.json pins them.
public enum DeepLink {
    public static func drive(_ id: Int64) -> URL {
        URL(string: "nankiv://drive/\(id)")!
    }

    public static let latest = URL(string: "nankiv://latest")!
    public static let shortlists = URL(string: "nankiv://shortlists")!
}
