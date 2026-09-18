import Foundation

/// What reading the snapshot produced.
public enum SnapshotLoad: Sendable, Equatable {
    /// The app has not written one yet — installed, but never opened.
    case missing
    /// A file is there but this widget cannot trust it: written by another
    /// version of nankiv, or damaged. Opening the app rewrites it.
    case stale
    case loaded(WidgetSnapshot)
}

/// Reads the snapshot the app writes.
///
/// The widget is sandboxed, with read access to exactly one directory — the
/// one this file lives in — granted by its entitlements. It cannot see the
/// database beside it, so this file is everything the widget knows.
public struct SnapshotSource: Sendable {
    /// Relative to the home directory. Must match the sandbox exception in
    /// NankivWidget.entitlements and the directory `lib.rs` publishes into.
    public static let relativePath =
        "Library/Application Support/app.nankiv.desktop/widget/snapshot.json"

    public let url: URL

    public init(url: URL) {
        self.url = url
    }

    /// The real snapshot. Inside the sandbox, `NSHomeDirectory()` points into
    /// the extension's container, so the account's home comes from the user
    /// database instead — the path the sandbox exception is written against.
    public static var standard: SnapshotSource {
        SnapshotSource(url: realHome().appendingPathComponent(relativePath))
    }

    public func load() -> SnapshotLoad {
        guard let data = try? Data(contentsOf: url) else {
            return FileManager.default.fileExists(atPath: url.path) ? .stale : .missing
        }
        return Self.decode(data)
    }

    public static func decode(_ data: Data) -> SnapshotLoad {
        struct Header: Decodable { let schema: Int }
        guard
            let header = try? JSONDecoder().decode(Header.self, from: data),
            header.schema == WidgetSnapshot.supportedSchema
        else { return .stale }

        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        decoder.dateDecodingStrategy = .iso8601
        guard let snapshot = try? decoder.decode(WidgetSnapshot.self, from: data) else {
            return .stale
        }
        return .loaded(snapshot)
    }

    static func realHome() -> URL {
        if let entry = getpwuid(getuid()), let dir = entry.pointee.pw_dir {
            return URL(fileURLWithPath: String(cString: dir), isDirectory: true)
        }
        return FileManager.default.homeDirectoryForCurrentUser
    }
}
