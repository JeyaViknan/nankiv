import Foundation
import Testing
@testable import NankivWidgetModel

// The fixtures are written by src-tauri/tests/widget_fixtures.rs from the same
// code the app publishes with. Decoding them here is what keeps the Swift
// mirror of the contract honest.

enum Fixture {
    static let directory = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()  // SnapshotTests.swift
        .deletingLastPathComponent()  // NankivWidgetModelTests
        .deletingLastPathComponent()  // Tests
        .deletingLastPathComponent()  // Core
        .deletingLastPathComponent()  // macos
        .appendingPathComponent("fixtures")

    static let names = [
        "ready", "importing", "failed", "not_shortlisted", "insufficient",
        "soft_preference", "undetermined", "not_set_up", "no_identity", "empty",
    ]

    static func data(_ name: String) throws -> Data {
        try Data(contentsOf: directory.appendingPathComponent("\(name).json"))
    }

    static func snapshot(_ name: String) throws -> WidgetSnapshot {
        guard case .loaded(let s) = SnapshotSource.decode(try data(name)) else {
            Issue.record("\(name).json did not decode")
            throw CocoaError(.coderReadCorrupt)
        }
        return s
    }

    /// The moment the fixtures were generated.
    static let now = ISO8601DateFormatter().date(from: "2026-09-17T15:00:00Z")!
}

@Suite("Decoding the snapshot the app publishes")
struct SnapshotTests {
    @Test("Every fixture decodes", arguments: Fixture.names)
    func decodes(name: String) throws {
        let snapshot = try Fixture.snapshot(name)
        #expect(snapshot.schema == WidgetSnapshot.supportedSchema)
        #expect(snapshot.generatedAt == Fixture.now)
    }

    @Test func theLatestDriveAndItsAnalysis() throws {
        let s = try Fixture.snapshot("ready")
        let latest = try #require(s.drives.first)
        #expect(latest.company == "Aurora Systems")
        #expect(latest.verdict == .shortlisted)
        #expect(latest.totalStudents == 96)
        #expect(latest.analysis.status == .ready)
        #expect(latest.analysis.cutoff == .around(threshold: 8.5, observedFloor: 8.51))
        #expect(!latest.analysis.histogram.isEmpty)
        #expect(latest.circle.total == 5)
        #expect(latest.circle.members.map(\.label).contains("Priya"))
        #expect(s.season == Season(drives: 5, shortlisted: 3, notShortlisted: 2, undetermined: 0))
        #expect(s.drives.map(\.company) == [
            "Aurora Systems", "Bluefin Analytics", "Cedar Labs", "Driftwood Energy", "Elmstead Foods",
        ])
    }

    @Test func transientActivityCarriesItsExpiry() throws {
        guard case .importing(let file, let since, let until) = try Fixture.snapshot("importing").activity else {
            Issue.record("expected importing"); return
        }
        #expect(file == "Fjord Robotics shortlist.xlsx")
        #expect(until.timeIntervalSince(since) == 180)

        guard case .failed(let failed, let at, let expiry) = try Fixture.snapshot("failed").activity else {
            Issue.record("expected failed"); return
        }
        #expect(failed == "Round 2 - final.xlsx")
        #expect(expiry.timeIntervalSince(at) == 86_400)
    }

    @Test func undeterminedKeepsItsReason() throws {
        let latest = try #require(try Fixture.snapshot("undetermined").drives.first)
        #expect(latest.verdict == .undetermined(.needsRegNo))
        let noIdentity = try #require(try Fixture.snapshot("no_identity").drives.first)
        #expect(noIdentity.verdict == .undetermined(.noIdentityConfigured))
    }

    @Test func analysisStatesStayDistinct() throws {
        #expect(try Fixture.snapshot("insufficient").drives.first?.analysis.status == .insufficient)
        #expect(try Fixture.snapshot("not_set_up").drives.first?.analysis.status == .notSetUp)
        #expect(try Fixture.snapshot("soft_preference").drives.first?.analysis.cutoff == .skewsHigh(observedFloor: 8.09))
    }

    // MARK: Surviving what this version does not know

    @Test func anotherSchemaIsStaleNotMisread() throws {
        var json = try JSONSerialization.jsonObject(with: try Fixture.data("ready")) as! [String: Any]
        json["schema"] = 2
        #expect(SnapshotSource.decode(try JSONSerialization.data(withJSONObject: json)) == .stale)
    }

    @Test func damagedFilesAreStale() {
        #expect(SnapshotSource.decode(Data("{\"schema\":1,".utf8)) == .stale)
        #expect(SnapshotSource.decode(Data()) == .stale)
        #expect(SnapshotSource.decode(Data("[]".utf8)) == .stale)
    }

    @Test func anUnknownVerdictIsNeverARejection() throws {
        let json = #"{"status":"waitlisted"}"#
        let v = try JSONDecoder().decode(Verdict.self, from: Data(json.utf8))
        #expect(v == .undetermined(.other))
    }

    @Test func anUnreadableCutoffIsDroppedNotFatal() throws {
        var json = try JSONSerialization.jsonObject(with: try Fixture.data("ready")) as! [String: Any]
        var drives = json["drives"] as! [[String: Any]]
        var analysis = drives[0]["analysis"] as! [String: Any]
        analysis["cutoff"] = ["kind": "hard_cutoff"]  // no threshold
        drives[0]["analysis"] = analysis
        json["drives"] = drives
        guard case .loaded(let s) = SnapshotSource.decode(try JSONSerialization.data(withJSONObject: json)) else {
            Issue.record("snapshot should still load"); return
        }
        #expect(s.drives[0].analysis.cutoff == nil)
        #expect(s.drives[0].verdict == .shortlisted)
    }

    @Test func anUnknownActivityShowsTheDataAnyway() throws {
        var json = try JSONSerialization.jsonObject(with: try Fixture.data("ready")) as! [String: Any]
        json["activity"] = ["state": "syncing"]
        guard case .loaded(let s) = SnapshotSource.decode(try JSONSerialization.data(withJSONObject: json)) else {
            Issue.record("snapshot should still load"); return
        }
        #expect(s.activity == .idle)
    }

    @Test func aMissingFileIsMissingNotStale() {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString + ".json")
        #expect(SnapshotSource(url: url).load() == .missing)
    }

    @Test func theStandardLocationIsTheRealHome() {
        let path = SnapshotSource.standard.url.path
        #expect(path.hasSuffix("/Library/Application Support/app.nankiv.desktop/widget/snapshot.json"))
        #expect(!path.contains("/Library/Containers/"))
    }

    // MARK: Links

    @Test func linksMatchWhatTheAppAccepts() throws {
        let links = try JSONDecoder().decode([String: String].self, from: try Fixture.data("links"))
        #expect(DeepLink.drive(42).absoluteString == links["drive_42"])
        #expect(DeepLink.latest.absoluteString == links["latest"])
        #expect(DeepLink.shortlists.absoluteString == links["shortlists"])
    }
}
