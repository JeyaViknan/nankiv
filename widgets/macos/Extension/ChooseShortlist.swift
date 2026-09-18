import AppIntents
import NankivWidgetModel
import WidgetKit

/// The widget's one setting: which shortlist to show.
///
/// Left empty — the default — the widget follows the latest shortlist, which
/// is what almost everyone wants, so nobody has to configure anything. Choosing
/// one keeps it in view while newer imports arrive: the company someone is
/// waiting on, or a drive with several rounds.
struct ChooseShortlist: WidgetConfigurationIntent {
    static let title: LocalizedStringResource = "Choose Shortlist"
    static let description = IntentDescription("Show your latest shortlist, or keep a particular one in view.")

    @Parameter(title: "Shortlist", description: "Leave empty to always show the latest.")
    var shortlist: ShortlistEntity?

    init() {}

    var pinnedID: Int64? {
        shortlist.flatMap { Int64($0.id) }
    }
}

struct ShortlistEntity: AppEntity {
    static let typeDisplayRepresentation = TypeDisplayRepresentation(name: "Shortlist")
    static let defaultQuery = ShortlistQuery()

    /// The drive's id in nankiv, as text.
    let id: String
    let company: String
    let detail: String

    var displayRepresentation: DisplayRepresentation {
        DisplayRepresentation(title: "\(company)", subtitle: "\(detail)")
    }
}

struct ShortlistQuery: EntityQuery {
    func entities(for identifiers: [ShortlistEntity.ID]) async throws -> [ShortlistEntity] {
        let drives = Self.drives()
        return identifiers.map { id in
            if let drive = drives.first(where: { String($0.id) == id }) {
                return Self.entity(drive)
            }
            // Returned rather than dropped. A dropped entity would quietly turn
            // the widget back into "latest"; this way it says the shortlist was
            // removed, and the settings name it for what it is.
            return ShortlistEntity(id: id, company: "Removed shortlist", detail: "No longer in nankiv")
        }
    }

    func suggestedEntities() async throws -> [ShortlistEntity] {
        Self.drives().map(Self.entity)
    }

    private static func drives() -> [DrivePreview] {
        guard case .loaded(let snapshot) = SnapshotSource.standard.load() else { return [] }
        return snapshot.drives
    }

    private static func entity(_ drive: DrivePreview) -> ShortlistEntity {
        let copy = Copy()
        let when = Freshness().label(for: drive.importedAt, now: Date())
        return ShortlistEntity(
            id: String(drive.id),
            company: drive.company,
            detail: "\(copy.status(drive.verdict)) · \(when)"
        )
    }
}
