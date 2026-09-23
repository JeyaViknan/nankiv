// The provider, measured against the shared files.
//
// widgets/fixtures/*.json are written by src-tauri/tests/widget_fixtures.rs.
// widgets/windows/cards/examples/*.json are what a card should say for each
// one. Between them, these tests catch the two ways this code can go wrong:
// misreading the contract, and drifting from what the app itself says.

using System.Globalization;
using System.Runtime.CompilerServices;
using System.Text.Json;
using System.Text.Json.Nodes;
using Xunit;

namespace Nankiv.Widgets.Tests;

internal static class Fixed
{
    /// <summary>
    /// The examples were written in one language, so the numbers and words in
    /// them are compared in that language. Dates are excluded from the
    /// comparison instead, because their format follows the machine's own
    /// culture and time zone — see <see cref="CardDataTests.DatesReadAsDates"/>.
    /// </summary>
    [ModuleInitializer]
    internal static void Culture()
    {
        CultureInfo.DefaultThreadCurrentCulture = new CultureInfo("en-IN");
        CultureInfo.DefaultThreadCurrentUICulture = new CultureInfo("en-IN");
    }
}

public class CardDataTests
{
    /// <summary>The moment the fixtures were generated, plus a minute.</summary>
    private static readonly DateTimeOffset Now = DateTimeOffset.Parse("2026-09-17T15:01:00Z");

    private static string Root =>
        Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", "..", ".."));

    private static string Fixture(string name) => Path.Combine(Root, "fixtures", $"{name}.json");

    private static string Example(string name) =>
        Path.Combine(Root, "windows", "cards", "examples", $"{name}.json");

    private static SnapshotLoad Load(string name) =>
        SnapshotReader.Decode(File.ReadAllText(Fixture(name)));

    /// <summary>Every fixture in widgets/fixtures that is a snapshot.</summary>
    public static readonly string[] SnapshotNames =
    [
        "ready", "importing", "failed", "not_shortlisted", "insufficient",
        "soft_preference", "undetermined", "not_set_up", "no_identity", "empty",
    ];

    public static TheoryData<string> Snapshots()
    {
        var data = new TheoryData<string>();
        foreach (var name in SnapshotNames) data.Add(name);
        return data;
    }

    [Theory]
    [MemberData(nameof(Snapshots))]
    public void EveryFixtureDecodes(string name)
    {
        var load = Load(name);
        Assert.Equal(SnapshotStateKind.Loaded, load.Kind);
        Assert.Equal(WidgetSnapshot.SupportedSchema, load.Snapshot!.Schema);
    }

    [Theory]
    [MemberData(nameof(Snapshots))]
    public void CardDataMatchesTheCommittedExample(string name)
    {
        var glance = Glance.Make(Load(name), pinned: null, Now);
        AssertMatchesExample(name, glance);
    }

    [Fact]
    public void APinnedShortlistAndOneThatWasDeleted()
    {
        var snapshot = Load("ready").Snapshot!;
        var cedar = snapshot.Drives.First(d => d.Company == "Cedar Labs");

        var pinned = Glance.Make(Load("ready"), cedar.Id, Now);
        Assert.True(pinned.IsPinned);
        Assert.Equal(cedar.Id, pinned.Drive!.Id);
        AssertMatchesExample("pinned", pinned);

        var removed = Glance.Make(Load("ready"), 999_999, Now);
        Assert.Equal(GlanceState.Removed, removed.State);
        AssertMatchesExample("removed", removed);
    }

    [Fact]
    public void AResultLeadsForATurnThenTheWidgetRests()
    {
        var snapshot = Load("ready").Snapshot!;
        Assert.NotNull(snapshot.LeadsUntil);
        var leadsUntil = snapshot.LeadsUntil!.Value;
        Assert.Equal(TimeSpan.FromHours(12), leadsUntil - snapshot.Drives[0].ImportedAt);

        var leading = Glance.Make(Load("ready"), null, leadsUntil.AddSeconds(-1));
        Assert.Equal(GlanceState.Ready, leading.State);

        var resting = Glance.Make(Load("ready"), null, leadsUntil.AddHours(1));
        Assert.Equal(GlanceState.Resting, resting.State);
        // Nothing is hidden: the result that was leading heads the list now.
        Assert.Equal("Aurora Systems", resting.Others[0].Company);
        AssertMatchesExample("resting", resting);
    }

    [Fact]
    public void AChosenShortlistNeverStepsBack()
    {
        var snapshot = Load("ready").Snapshot!;
        var cedar = snapshot.Drives.First(d => d.Company == "Cedar Labs");
        var glance = Glance.Make(Load("ready"), cedar.Id, snapshot.LeadsUntil!.Value.AddDays(3));
        Assert.Equal(GlanceState.Ready, glance.State);
        Assert.Equal(cedar.Id, glance.Drive!.Id);
    }

    [Fact]
    public void BeforeTheAppHasEverRun()
    {
        AssertMatchesExample("not_started", Glance.Make(SnapshotLoad.Missing, null, Now));
        AssertMatchesExample("needs_refresh", Glance.Make(SnapshotLoad.Stale, null, Now));
    }

    [Fact]
    public void ASnapshotFromAnotherVersionIsStaleNotMisread()
    {
        var json = File.ReadAllText(Fixture("ready")).Replace("\"schema\": 1", "\"schema\": 2");
        Assert.Equal(SnapshotStateKind.Stale, SnapshotReader.Decode(json).Kind);
        Assert.Equal(SnapshotStateKind.Stale, SnapshotReader.Decode("{\"schema\":1,").Kind);
    }

    [Fact]
    public void AnUnknownVerdictIsNeverARejection()
    {
        var verdict = Verdict.From("waitlisted", null);
        Assert.Equal(VerdictStatus.Undetermined, verdict.Status);
        Assert.Equal("Can't tell", Copy.Title(verdict));
    }

    [Fact]
    public void TransientStatesExpireWhenTheSnapshotSaysSo()
    {
        var during = Glance.Make(Load("importing"), null, Now);
        Assert.NotNull(during.Notice);

        // The app quit mid-import: the widget stops believing it.
        var after = Glance.Make(Load("importing"), null, Now.AddMinutes(5));
        Assert.Null(after.Notice);
    }

    [Fact]
    public void ACutoffIsAlwaysCalledAnEstimate()
    {
        var drive = Load("ready").Snapshot!.Drives[0];
        Assert.Equal("CGPA cutoff around 8.5", Copy.Headline(drive.Analysis));
        Assert.StartsWith("Estimate from", Copy.Basis(drive.Analysis, drive.TotalStudents));
    }

    [Fact]
    public void NoIdentifiersReachTheCard()
    {
        foreach (var name in SnapshotNames)
        {
            var json = CardData.Serialise(CardData.Build(Glance.Make(Load(name), null, Now), Now));
            Assert.DoesNotMatch(@"[A-Z]\d[A-Z]\d[A-Z]\d[A-Z]\d", json);
            Assert.DoesNotMatch(@"\d{2}[A-Z]{3}\d{4}", json);
        }
    }

    [Fact]
    public void DatesReadAsDates()
    {
        var now = new DateTimeOffset(2026, 9, 17, 20, 30, 0, TimeSpan.Zero).ToLocalTime();
        Assert.StartsWith("Today, ", Copy.When(now.AddHours(-3), now));
        Assert.Equal("Yesterday", Copy.When(now.AddDays(-1), now));
        Assert.Equal(
            CultureInfo.CurrentCulture.DateTimeFormat.GetDayName(now.AddDays(-3).DayOfWeek),
            Copy.When(now.AddDays(-3), now));
        Assert.DoesNotContain("Today", Copy.When(now.AddDays(-10), now));
    }

    private static void AssertMatchesExample(string name, Glance glance)
    {
        var expected = JsonNode.Parse(File.ReadAllText(Example(name)))!;
        var actual = CardData.Build(glance, Now);
        Assert.Equal(Canonical(Dateless(expected)), Canonical(Dateless(actual)));
    }

    /// <summary>
    /// Replaces every date label, which depends on the machine's culture and
    /// time zone rather than on the contract.
    /// </summary>
    private static JsonNode Dateless(JsonNode node)
    {
        switch (node)
        {
            case JsonObject obj:
                var copy = new JsonObject();
                foreach (var pair in obj)
                {
                    copy[pair.Key] = pair.Key == "when"
                        ? JsonValue.Create("<date>")
                        : pair.Value is null ? null : Dateless(pair.Value);
                }
                return copy;
            case JsonArray array:
                var items = new JsonArray();
                foreach (var item in array) items.Add(item is null ? null : Dateless(item));
                return items;
            default:
                return JsonNode.Parse(node.ToJsonString())!;
        }
    }

    /// <summary>Key order is not part of the contract; content is.</summary>
    private static string Canonical(JsonNode node)
    {
        if (node is JsonObject obj)
        {
            var sorted = new JsonObject();
            foreach (var pair in obj.OrderBy(p => p.Key, StringComparer.Ordinal))
            {
                sorted[pair.Key] = pair.Value is null ? null : JsonNode.Parse(Canonical(pair.Value));
            }
            return sorted.ToJsonString();
        }
        if (node is JsonArray array)
        {
            var items = new JsonArray();
            foreach (var item in array)
            {
                items.Add(item is null ? null : JsonNode.Parse(Canonical(item)));
            }
            return items.ToJsonString();
        }
        return node.ToJsonString(new JsonSerializerOptions());
    }
}
