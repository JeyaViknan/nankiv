// What the widget shows, and the words it uses.
//
// The same decisions as Glance.swift and Copy.swift on macOS, in the same
// order, with the same rules: no identity means no answer to show; a transient
// state expires at the moment the snapshot says it does; an estimate is always
// called an estimate. widgets/windows/cards/examples/*.json are the expected
// results, and Nankiv.Widgets.Tests holds this file to them.

using System.Globalization;

namespace Nankiv.Widgets;

public enum GlanceState { NotStarted, NeedsRefresh, NeedsIdentity, NoShortlists, Removed, Ready }

public sealed record Notice(bool IsFailure, string Filename);

public sealed record Glance(
    GlanceState State,
    DrivePreview? Drive,
    bool IsPinned,
    Notice? Notice,
    Season? Season,
    IReadOnlyList<DrivePreview> Others)
{
    public static Glance Make(SnapshotLoad load, long? pinned, DateTimeOffset now)
    {
        if (load.Kind == SnapshotStateKind.Missing) return Empty(GlanceState.NotStarted);
        if (load.Kind == SnapshotStateKind.Stale || load.Snapshot is null) return Empty(GlanceState.NeedsRefresh);

        var snapshot = load.Snapshot;
        if (!snapshot.IdentityConfigured) return Empty(GlanceState.NeedsIdentity);

        var notice = NoticeFor(snapshot.Activity, now);

        DrivePreview? shown;
        if (pinned is { } id)
        {
            shown = snapshot.Drives.FirstOrDefault(d => d.Id == id);
            if (shown is null) return Empty(GlanceState.Removed);
        }
        else
        {
            shown = snapshot.Drives.FirstOrDefault();
            if (shown is null)
            {
                return new Glance(GlanceState.NoShortlists, null, false, notice, snapshot.Season, []);
            }
        }

        return new Glance(
            GlanceState.Ready,
            shown,
            pinned is not null,
            notice,
            snapshot.Season,
            snapshot.Drives.Where(d => d.Id != shown.Id).ToList());
    }

    private static Glance Empty(GlanceState state) => new(state, null, false, null, null, []);

    private static Notice? NoticeFor(Activity activity, DateTimeOffset now) => activity switch
    {
        Activity.Importing i when now < i.Until => new Notice(false, i.Filename),
        Activity.Failed f when now < f.Until => new Notice(true, f.Filename),
        _ => null,
    };
}

/// <summary>Every sentence the Windows widget shows. Word for word with macOS.</summary>
public static class Copy
{
    public static string Title(Verdict verdict) => verdict.Status switch
    {
        VerdictStatus.Shortlisted => "You're in",
        VerdictStatus.NotShortlisted => "Not this time",
        _ => "Can't tell",
    };

    public static string Status(Verdict verdict) => verdict.Status switch
    {
        VerdictStatus.Shortlisted => "In",
        VerdictStatus.NotShortlisted => "Not in",
        _ => "Unknown",
    };

    public static string Detail(Verdict verdict, int totalStudents) => verdict.Status switch
    {
        VerdictStatus.Shortlisted => $"{Count(totalStudents)} students shortlisted",
        VerdictStatus.NotShortlisted => $"Not among the {Count(totalStudents)} shortlisted",
        _ => verdict.Reason switch
        {
            UndeterminedReason.NeedsRegNo => "This list uses registration numbers. Add yours in nankiv.",
            UndeterminedReason.NeedsNeoId => "This list uses Neo IDs. Add yours in nankiv.",
            UndeterminedReason.FileNotUnderstood => "This file has no IDs to match you against.",
            UndeterminedReason.NoIdentityConfigured => "Add your Neo ID in nankiv.",
            _ => "Open nankiv for details.",
        },
    };

    public static string Headline(AnalysisSummary analysis) => analysis.Status switch
    {
        AnalysisStatus.Insufficient => "Not enough data to analyse",
        AnalysisStatus.NotSetUp => "Analysis isn't set up",
        _ => analysis.Cutoff?.Kind switch
        {
            CutoffKind.Around => $"CGPA cutoff around {Cgpa(analysis.Cutoff.Threshold ?? 0, 1)}",
            CutoffKind.SkewsHigh => "Skews high, no hard cutoff",
            _ => "No CGPA filter detected",
        },
    };

    public static string Basis(AnalysisSummary analysis, int totalStudents) => analysis.Status switch
    {
        AnalysisStatus.Ready => analysis.Matched >= totalStudents
            ? $"Estimate from all {Count(totalStudents)} students"
            : $"Estimate from {Count(analysis.Matched)} of {Count(totalStudents)} students",
        AnalysisStatus.Insufficient => $"Matched {Count(analysis.Matched)} of {Count(totalStudents)} students",
        _ => "Open nankiv to add CGPA data.",
    };

    /// <summary>"8.5 – 10.0 · median 8.95". The Windows widget has no chart element.</summary>
    public static string? Spread(AnalysisSummary analysis)
    {
        if (analysis.Status != AnalysisStatus.Ready || analysis.Histogram.Count == 0) return null;
        var low = Cgpa(analysis.Histogram[0].Lower, 1);
        var high = Cgpa(analysis.Histogram[^1].Upper, 1);
        return analysis.Median is { } median
            ? $"{low} – {high} · median {Cgpa(median, 2)}"
            : $"{low} – {high}";
    }

    public static string CircleSummaryText(CircleSummary circle) =>
        $"{Count(circle.Shortlisted)} of {Count(circle.Total)} in";

    public static string SeasonSummary(Season season) =>
        season.Drives == 1 ? "1 shortlist" : $"{Count(season.Drives)} shortlists";

    public static string SeasonTally(Season season)
    {
        var parts = new List<string> { $"{Count(season.Shortlisted)} in", $"{Count(season.NotShortlisted)} not in" };
        if (season.Undetermined > 0) parts.Add($"{Count(season.Undetermined)} unknown");
        return string.Join(" · ", parts);
    }

    public static string NoticeFull(Notice notice) => notice.IsFailure
        ? $"Couldn't import {notice.Filename}"
        : $"Importing {notice.Filename}…";

    public static string NoticeShort(Notice notice) => notice.IsFailure ? "Last import failed" : "Importing…";

    public static (string Title, string Body)? Message(Glance glance) => glance.State switch
    {
        GlanceState.NotStarted => ("Open nankiv", "Your latest shortlist result will appear here."),
        GlanceState.NeedsRefresh => ("Open nankiv", "Open the app to bring this widget up to date."),
        GlanceState.NeedsIdentity => ("Finish setting up", "Add your Neo ID in nankiv to see your results here."),
        GlanceState.Removed => ("Shortlist removed", "Edit this widget to choose another."),
        GlanceState.NoShortlists => glance.Notice switch
        {
            { IsFailure: true } n => ("Couldn't import", $"{n.Filename} — open nankiv to see why."),
            { IsFailure: false } n => ("Importing…", n.Filename),
            _ => ("No shortlists yet", "Drop a shortlist into nankiv and your result appears here."),
        },
        _ => null,
    };

    /// <summary>"Today, 17:30", "Yesterday", "Monday", "7 Sep" — absolute, so it stays true.</summary>
    public static string When(DateTimeOffset date, DateTimeOffset now)
    {
        var culture = CultureInfo.CurrentCulture;
        var days = (now.Date - date.Date).Days;
        return days switch
        {
            <= 0 => $"Today, {date.ToLocalTime().ToString("t", culture)}",
            1 => "Yesterday",
            <= 6 => culture.DateTimeFormat.GetDayName(date.ToLocalTime().DayOfWeek),
            _ => date.ToLocalTime().ToString(date.Year == now.Year ? "d MMM" : "d MMM yyyy", culture),
        };
    }

    public static string Count(int n) => n.ToString("N0", CultureInfo.CurrentCulture);

    public static string Cgpa(double value, int digits) =>
        value.ToString("F" + digits.ToString(CultureInfo.InvariantCulture), CultureInfo.CurrentCulture);
}
