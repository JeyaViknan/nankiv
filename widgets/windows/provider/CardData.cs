// Turns a glance into the data the Adaptive Card templates bind to.
//
// The templates in widgets/windows/cards decide layout; this decides content.
// widgets/windows/cards/examples/*.json are the expected output for every
// state — the Windows tests hold this code to them, and a check on any machine
// expands the templates against them (see widgets/windows/cards.test.ts).

using System.Text.Json;
using System.Text.Json.Nodes;

namespace Nankiv.Widgets;

public static class CardData
{
    /// <summary>Members shown in a card; the rest are counted in the summary.</summary>
    public const int MembersShown = 4;

    /// <summary>Recent shortlists listed on the large card.</summary>
    public const int RecentShown = 4;

    public static JsonObject Build(Glance glance, DateTimeOffset now)
    {
        var data = new JsonObject
        {
            ["state"] = glance.State == GlanceState.Ready ? "ready" : Name(glance.State),
            ["link"] = glance.Drive is { } target
                ? $"nankiv://drive/{target.Id}"
                : "nankiv://shortlists",
        };

        if (Copy.Message(glance) is { } message)
        {
            data["message"] = new JsonObject { ["title"] = message.Title, ["body"] = message.Body };
            return data;
        }

        var drive = glance.Drive!;
        data["company"] = drive.Company;
        data["title"] = Copy.Title(drive.Verdict);
        data["detail"] = Copy.Detail(drive.Verdict, drive.TotalStudents);
        data["tone"] = Tone(drive.Verdict);
        data["when"] = Copy.When(drive.ImportedAt, now);
        data["pinned"] = glance.IsPinned;
        data["notice"] = glance.Notice is { } notice
            ? new JsonObject
            {
                ["full"] = Copy.NoticeFull(notice),
                ["short"] = Copy.NoticeShort(notice),
                ["tone"] = notice.IsFailure ? "Warning" : "Default",
            }
            : null;

        data["analysis"] = new JsonObject
        {
            ["headline"] = Copy.Headline(drive.Analysis),
            ["basis"] = Copy.Basis(drive.Analysis, drive.TotalStudents),
            ["spread"] = Copy.Spread(drive.Analysis),
        };

        // One insight beside the answer, as on macOS: the analysis if there is
        // one, otherwise the circle, otherwise the season.
        data["insight"] = drive.Analysis.Status == AnalysisStatus.Ready
            ? "analysis"
            : drive.Circle.Total > 0 ? "circle" : "season";

        data["circle"] = drive.Circle.Total > 0
            ? new JsonObject
            {
                ["summary"] = Copy.CircleSummaryText(drive.Circle),
                ["members"] = new JsonArray(drive.Circle.Members.Take(MembersShown).Select(m =>
                    (JsonNode)new JsonObject
                    {
                        ["label"] = m.Label,
                        ["status"] = Copy.Status(m.Verdict),
                    }).ToArray()),
            }
            : null;

        data["recent"] = glance.Others.Count > 0
            ? new JsonArray(glance.Others.Take(RecentShown).Select(d =>
                (JsonNode)new JsonObject
                {
                    ["company"] = d.Company,
                    ["status"] = Copy.Status(d.Verdict),
                    ["tone"] = Tone(d.Verdict),
                    ["when"] = Copy.When(d.ImportedAt, now),
                    ["link"] = $"nankiv://drive/{d.Id}",
                }).ToArray())
            : null;

        data["season"] = glance.Season is { } season
            ? new JsonObject
            {
                ["summary"] = Copy.SeasonSummary(season),
                ["tally"] = Copy.SeasonTally(season),
            }
            : null;

        return data;
    }

    public static string Serialise(JsonObject data) =>
        data.ToJsonString(new JsonSerializerOptions { WriteIndented = false });

    /// <summary>
    /// Adaptive Cards' own colour names. Colour is never the only signal: the
    /// status word sits beside it everywhere it is used.
    /// </summary>
    private static string Tone(Verdict verdict) => verdict.Status switch
    {
        VerdictStatus.Shortlisted => "Good",
        VerdictStatus.NotShortlisted => "Default",
        _ => "Warning",
    };

    private static string Name(GlanceState state) => state switch
    {
        GlanceState.NotStarted => "notStarted",
        GlanceState.NeedsRefresh => "needsRefresh",
        GlanceState.NeedsIdentity => "needsIdentity",
        GlanceState.NoShortlists => "noShortlists",
        GlanceState.Removed => "removed",
        _ => "ready",
    };
}
