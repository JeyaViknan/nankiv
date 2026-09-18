import Foundation
import Testing
@testable import NankivWidgetModel

@Suite("Choosing what to show")
struct GlanceTests {
    let now = Fixture.now

    func glance(_ name: String, pinned: Int64? = nil, at date: Date? = nil) throws -> Glance {
        Glance.make(from: .loaded(try Fixture.snapshot(name)), pinned: pinned, at: date ?? now)
    }

    @Test func beforeTheAppHasRun() {
        #expect(Glance.make(from: .missing, pinned: nil, at: now) == .notStarted)
        #expect(Glance.make(from: .stale, pinned: nil, at: now) == .needsRefresh)
    }

    @Test func withoutAnIdentityThereIsNoAnswerToShow() throws {
        #expect(try glance("no_identity") == .needsIdentity)
    }

    @Test func setUpButEmpty() throws {
        #expect(try glance("empty") == .noShortlists(nil))
    }

    @Test func showsTheLatestShortlistByDefault() throws {
        guard case .shortlist(let s) = try glance("ready") else { Issue.record("expected a shortlist"); return }
        #expect(s.drive.company == "Aurora Systems")
        #expect(!s.isPinned)
        #expect(s.notice == nil)
        #expect(s.others.count == 4)
        #expect(!s.others.contains { $0.id == s.drive.id })
    }

    @Test func showsAPinnedShortlist() throws {
        let snapshot = try Fixture.snapshot("ready")
        let cedar = try #require(snapshot.drives.first { $0.company == "Cedar Labs" })
        guard case .shortlist(let s) = try glance("ready", pinned: cedar.id) else {
            Issue.record("expected a shortlist"); return
        }
        #expect(s.drive == cedar)
        #expect(s.isPinned)
        #expect(s.others.first?.company == "Aurora Systems")
    }

    @Test func aPinnedShortlistThatWasDeletedSaysSo() throws {
        #expect(try glance("ready", pinned: 999_999) == .removed)
    }

    @Test func importingShowsOnlyUntilItExpires() throws {
        guard case .shortlist(let during) = try glance("importing") else { Issue.record("expected a shortlist"); return }
        #expect(during.notice == .importing(filename: "Fjord Robotics shortlist.xlsx"))

        // The app quit mid-import: the widget stops believing it.
        guard case .shortlist(let after) = try glance("importing", at: now.addingTimeInterval(200)) else {
            Issue.record("expected a shortlist"); return
        }
        #expect(after.notice == nil)
    }

    @Test func aFailureShowsForADay() throws {
        guard case .shortlist(let soon) = try glance("failed", at: now) else { Issue.record("expected a shortlist"); return }
        #expect(soon.notice == .failed(filename: "Round 2 - final.xlsx"))
        guard case .shortlist(let later) = try glance("failed", at: now.addingTimeInterval(23 * 3600)) else {
            Issue.record("expected a shortlist"); return
        }
        #expect(later.notice == nil, "recorded two hours before the fixture, so it has expired")
    }

    @Test func linksGoStraightToTheShortlistOnScreen() throws {
        let snapshot = try Fixture.snapshot("ready")
        #expect(try glance("ready").destination == DeepLink.drive(snapshot.drives[0].id))
        #expect(Glance.notStarted.destination == DeepLink.shortlists)
        #expect(Glance.removed.destination == DeepLink.shortlists)
        #expect(try glance("no_identity").destination == DeepLink.shortlists)
    }

    @Test func theSampleIsEntirelyInvented() {
        guard case .shortlist(let s) = Glance.sample(now: now) else { Issue.record("expected a shortlist"); return }
        #expect(s.drive.company == "Your latest shortlist")
        #expect(s.drive.circle.members.allSatisfy { $0.label == "Friend" })
    }
}

@Suite("Planning the timeline")
struct ScheduleTests {
    var calendar: Calendar {
        var c = Calendar(identifier: .gregorian)
        c.timeZone = TimeZone(identifier: "Asia/Kolkata")!
        return c
    }

    @Test func withoutASnapshotItWaitsForTheApp() {
        let now = Fixture.now
        let plan = Schedule.plan(for: .missing, from: now, calendar: calendar)
        #expect(plan.moments == [now])
        #expect(plan.reload == now.addingTimeInterval(Schedule.retryWithoutSnapshot))
    }

    @Test func coversAWeekOfMidnights() throws {
        let now = Fixture.now
        let plan = Schedule.plan(for: .loaded(try Fixture.snapshot("ready")), from: now, calendar: calendar)
        #expect(plan.moments.first == now)
        #expect(plan.moments.count == 1 + Schedule.midnights)
        for moment in plan.moments.dropFirst() {
            #expect(calendar.startOfDay(for: moment) == moment)
        }
        #expect(plan.reload == plan.moments.last)
        #expect(plan.moments == plan.moments.sorted())
    }

    @Test func includesTheMomentAnImportExpires() throws {
        let now = Fixture.now
        let plan = Schedule.plan(for: .loaded(try Fixture.snapshot("importing")), from: now, calendar: calendar)
        #expect(plan.moments.contains(now.addingTimeInterval(160)))
        #expect(plan.moments.count == 2 + Schedule.midnights)
    }

    @Test func anExpiredStateAddsNothing() throws {
        let later = Fixture.now.addingTimeInterval(3600)
        let plan = Schedule.plan(for: .loaded(try Fixture.snapshot("importing")), from: later, calendar: calendar)
        #expect(plan.moments.count == 1 + Schedule.midnights)
    }
}

@Suite("Words")
struct CopyTests {
    let copy = Copy(locale: Locale(identifier: "en_IN"))

    @Test func threeAnswersNeverTwo() {
        #expect(copy.title(.shortlisted) == "You're in")
        #expect(copy.title(.notShortlisted) == "Not this time")
        #expect(copy.title(.undetermined(.needsRegNo)) == "Can't tell")
        #expect(copy.status(.undetermined(.other)) == "Unknown")
    }

    @Test func aCutoffIsAlwaysCalledAnEstimate() throws {
        let latest = try #require(try Fixture.snapshot("ready").drives.first)
        #expect(copy.headline(latest.analysis) == "CGPA cutoff around 8.5")
        #expect(copy.basis(latest.analysis, totalStudents: latest.totalStudents).hasPrefix("Estimate from"))
        #expect(!copy.headline(latest.analysis).localizedCaseInsensitiveContains("official"))
    }

    @Test func partialCoverageIsStated() {
        let a = AnalysisSummary(
            status: .ready, matched: 80, coverage: 0.5, cutoff: .noFilter, median: 8.8,
            histogram: [], overRepresented: [])
        #expect(copy.basis(a, totalStudents: 160) == "Estimate from 80 of 160 students")
    }

    @Test func missingDataIsNotCalledThin() throws {
        let notSetUp = try #require(try Fixture.snapshot("not_set_up").drives.first)
        let thin = try #require(try Fixture.snapshot("insufficient").drives.first)
        #expect(copy.headline(notSetUp.analysis) != copy.headline(thin.analysis))
    }

    @Test func everyWholeWidgetStateHasWords() {
        for glance in [Glance.notStarted, .needsRefresh, .needsIdentity, .noShortlists(nil),
                       .noShortlists(.importing(filename: "a.xlsx")), .noShortlists(.failed(filename: "a.xlsx")), .removed] {
            let m = copy.message(for: glance)
            #expect(m != nil)
            #expect(!(m?.title.isEmpty ?? true))
        }
        #expect(copy.message(for: .sample()) == nil)
    }

    @Test func numbersUseTheLocale() {
        #expect(copy.count(1815) == "1,815")
        #expect(Copy(locale: Locale(identifier: "de_DE")).cgpa(8.5, digits: 1) == "8,5")
    }
}

@Suite("Dates")
struct FreshnessTests {
    let freshness: Freshness = {
        var c = Calendar(identifier: .gregorian)
        c.timeZone = TimeZone(identifier: "Asia/Kolkata")!
        return Freshness(calendar: c, locale: Locale(identifier: "en_GB"))
    }()
    // 20:30 in Kolkata.
    let now = Fixture.now

    @Test func today() {
        #expect(freshness.label(for: now.addingTimeInterval(-3 * 3600), now: now) == "Today, 17:30")
        #expect(freshness.compactLabel(for: now.addingTimeInterval(-3 * 3600), now: now) == "17:30")
    }

    @Test func aLittleClockSkewIsStillToday() {
        #expect(freshness.label(for: now.addingTimeInterval(30), now: now).hasPrefix("Today"))
    }

    @Test func yesterdayThenWeekdaysThenDates() {
        #expect(freshness.label(for: now.addingTimeInterval(-86_400), now: now) == "Yesterday")
        #expect(freshness.label(for: now.addingTimeInterval(-3 * 86_400), now: now) == "Monday")
        #expect(freshness.compactLabel(for: now.addingTimeInterval(-3 * 86_400), now: now) == "Mon")
        #expect(freshness.label(for: now.addingTimeInterval(-10 * 86_400), now: now).hasPrefix("7 Sep"), "month names vary a little between ICU versions")
    }

    @Test func anotherYearSaysSo() {
        #expect(freshness.label(for: now.addingTimeInterval(-300 * 86_400), now: now).hasSuffix("2025"))
    }

    @Test func labelsOnlyChangeAtMidnight() {
        let imported = now.addingTimeInterval(-3 * 3600)
        let beforeMidnight = freshness.calendar.date(bySettingHour: 23, minute: 59, second: 0, of: now)!
        let afterMidnight = beforeMidnight.addingTimeInterval(120)
        #expect(freshness.label(for: imported, now: beforeMidnight).hasPrefix("Today"))
        #expect(freshness.label(for: imported, now: afterMidnight) == "Yesterday")
    }
}
