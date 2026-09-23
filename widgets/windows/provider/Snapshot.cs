// The snapshot the app publishes, as C# sees it.
//
// A mirror of src-tauri/src/widget.rs, like Swift's NankivWidgetModel. The
// files in widgets/fixtures are the shared examples; Nankiv.Widgets.Tests
// decodes every one of them.
//
// Nothing here interprets data. Where a value is unrecognised the choice is
// always "unknown", never an answer — a shortlist result that cannot be read
// must not turn into "not shortlisted".

using System.Text.Json;
using System.Text.Json.Serialization;

namespace Nankiv.Widgets;

public sealed record WidgetSnapshot(
    int Schema,
    DateTimeOffset GeneratedAt,
    bool IdentityConfigured,
    Activity Activity,
    Season Season,
    DateTimeOffset? LeadsUntil,
    IReadOnlyList<DrivePreview> Drives)
{
    public const int SupportedSchema = 1;
}

public abstract record Activity
{
    public sealed record Idle : Activity;

    /// <summary>Shown until <paramref name="Until"/>: past it, the app is assumed to have stopped.</summary>
    public sealed record Importing(string Filename, DateTimeOffset Since, DateTimeOffset Until) : Activity;

    /// <summary>Shown until <paramref name="Until"/>, or until an import succeeds.</summary>
    public sealed record Failed(string Filename, DateTimeOffset At, DateTimeOffset Until) : Activity;
}

public sealed record Season(int Drives, int Shortlisted, int NotShortlisted, int Undetermined);

public sealed record DrivePreview(
    long Id,
    string Company,
    DateTimeOffset ImportedAt,
    int TotalStudents,
    Verdict Verdict,
    CircleSummary Circle,
    AnalysisSummary Analysis);

public enum VerdictStatus { Shortlisted, NotShortlisted, Undetermined }

public enum UndeterminedReason { NoIdentityConfigured, NeedsNeoId, NeedsRegNo, FileNotUnderstood, Other }

public sealed record Verdict(VerdictStatus Status, UndeterminedReason? Reason)
{
    public static Verdict From(string? status, string? reason) => status switch
    {
        "shortlisted" => new Verdict(VerdictStatus.Shortlisted, null),
        "not_shortlisted" => new Verdict(VerdictStatus.NotShortlisted, null),
        // Includes anything this version does not recognise.
        _ => new Verdict(VerdictStatus.Undetermined, reason switch
        {
            "no_identity_configured" => UndeterminedReason.NoIdentityConfigured,
            "needs_neo_id" => UndeterminedReason.NeedsNeoId,
            "needs_reg_no" => UndeterminedReason.NeedsRegNo,
            "file_not_understood" => UndeterminedReason.FileNotUnderstood,
            _ => UndeterminedReason.Other,
        }),
    };
}

public sealed record CircleSummary(
    int Total,
    int Shortlisted,
    int NotShortlisted,
    int Undetermined,
    IReadOnlyList<CircleMember> Members);

public sealed record CircleMember(string Label, Verdict Verdict);

public enum AnalysisStatus { Ready, Insufficient, NotSetUp }

public sealed record AnalysisSummary(
    AnalysisStatus Status,
    int Matched,
    double Coverage,
    Cutoff? Cutoff,
    double? Median,
    IReadOnlyList<Bucket> Histogram,
    IReadOnlyList<string> OverRepresented);

public enum CutoffKind { Around, SkewsHigh, NoFilter }

/// <summary>Always an estimate, and always presented as one.</summary>
public sealed record Cutoff(CutoffKind Kind, double? Threshold, double? ObservedFloor);

public sealed record Bucket(double Lower, double Upper, int Count);

public enum SnapshotStateKind { Missing, Stale, Loaded }

public sealed record SnapshotLoad(SnapshotStateKind Kind, WidgetSnapshot? Snapshot)
{
    public static readonly SnapshotLoad Missing = new(SnapshotStateKind.Missing, null);
    public static readonly SnapshotLoad Stale = new(SnapshotStateKind.Stale, null);
    public static SnapshotLoad Loaded(WidgetSnapshot s) => new(SnapshotStateKind.Loaded, s);
}

/// <summary>
/// Reads the snapshot the app writes. One small file; no database, no analysis.
/// </summary>
public static class SnapshotReader
{
    /// <summary>Must match the directory <c>lib.rs</c> publishes into.</summary>
    public static string DefaultPath => Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
        "app.nankiv.desktop", "widget", "snapshot.json");

    public static SnapshotLoad Load(string? path = null)
    {
        path ??= DefaultPath;
        if (!File.Exists(path)) return SnapshotLoad.Missing;
        try
        {
            return Decode(File.ReadAllText(path));
        }
        catch (IOException)
        {
            // Being rewritten as we read: the writer renames into place, so the
            // next update will bring a whole file.
            return SnapshotLoad.Stale;
        }
    }

    public static SnapshotLoad Decode(string json)
    {
        try
        {
            using var document = JsonDocument.Parse(json);
            if (!document.RootElement.TryGetProperty("schema", out var schema)
                || schema.GetInt32() != WidgetSnapshot.SupportedSchema)
            {
                // Written by another version of nankiv. Opening the app rewrites it.
                return SnapshotLoad.Stale;
            }
            var snapshot = JsonSerializer.Deserialize<WidgetSnapshot>(json, Options);
            return snapshot is null ? SnapshotLoad.Stale : SnapshotLoad.Loaded(snapshot);
        }
        catch (JsonException)
        {
            return SnapshotLoad.Stale;
        }
    }

    public static readonly JsonSerializerOptions Options = Build();

    private static JsonSerializerOptions Build()
    {
        var options = new JsonSerializerOptions
        {
            PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
            PropertyNameCaseInsensitive = true,
        };
        options.Converters.Add(new ActivityConverter());
        options.Converters.Add(new VerdictConverter());
        options.Converters.Add(new CircleMemberConverter());
        options.Converters.Add(new AnalysisConverter());
        return options;
    }
}

internal sealed class ActivityConverter : JsonConverter<Activity>
{
    public override Activity Read(ref Utf8JsonReader reader, Type type, JsonSerializerOptions options)
    {
        using var document = JsonDocument.ParseValue(ref reader);
        var root = document.RootElement;
        var state = root.TryGetProperty("state", out var s) ? s.GetString() : null;
        switch (state)
        {
            case "importing":
                return new Activity.Importing(
                    root.GetProperty("filename").GetString() ?? "",
                    root.GetProperty("since").GetDateTimeOffset(),
                    root.GetProperty("until").GetDateTimeOffset());
            case "failed":
                return new Activity.Failed(
                    root.GetProperty("filename").GetString() ?? "",
                    root.GetProperty("at").GetDateTimeOffset(),
                    root.GetProperty("until").GetDateTimeOffset());
            default:
                // Something newer than this widget knows about: show the data
                // without a status line rather than nothing at all.
                return new Activity.Idle();
        }
    }

    public override void Write(Utf8JsonWriter writer, Activity value, JsonSerializerOptions options) =>
        throw new NotSupportedException("the widget never writes snapshots");
}

internal sealed class VerdictConverter : JsonConverter<Verdict>
{
    public override Verdict Read(ref Utf8JsonReader reader, Type type, JsonSerializerOptions options)
    {
        using var document = JsonDocument.ParseValue(ref reader);
        var root = document.RootElement;
        return Verdict.From(
            root.TryGetProperty("status", out var s) ? s.GetString() : null,
            root.TryGetProperty("reason", out var r) ? r.GetString() : null);
    }

    public override void Write(Utf8JsonWriter writer, Verdict value, JsonSerializerOptions options) =>
        throw new NotSupportedException("the widget never writes snapshots");
}

/// <summary>
/// A circle member carries its answer as a plain `status`, not as the nested
/// verdict object a drive has — there is no reason for it, since a friend's
/// result has no explanation attached. Without this the property decodes as
/// null and the first card built from it throws.
/// </summary>
internal sealed class CircleMemberConverter : JsonConverter<CircleMember>
{
    public override CircleMember Read(ref Utf8JsonReader reader, Type type, JsonSerializerOptions options)
    {
        using var document = JsonDocument.ParseValue(ref reader);
        var root = document.RootElement;
        return new CircleMember(
            root.TryGetProperty("label", out var label) ? label.GetString() ?? "" : "",
            Verdict.From(root.TryGetProperty("status", out var status) ? status.GetString() : null, null));
    }

    public override void Write(Utf8JsonWriter writer, CircleMember value, JsonSerializerOptions options) =>
        throw new NotSupportedException("the widget never writes snapshots");
}

internal sealed class AnalysisConverter : JsonConverter<AnalysisSummary>
{
    public override AnalysisSummary Read(ref Utf8JsonReader reader, Type type, JsonSerializerOptions options)
    {
        using var document = JsonDocument.ParseValue(ref reader);
        var root = document.RootElement;
        var status = root.TryGetProperty("status", out var s) ? s.GetString() : null;
        var parsed = status switch
        {
            "ready" => AnalysisStatus.Ready,
            "not_set_up" => AnalysisStatus.NotSetUp,
            // An unfamiliar status shows no statistics rather than guessed ones.
            _ => AnalysisStatus.Insufficient,
        };

        Cutoff? cutoff = null;
        if (parsed == AnalysisStatus.Ready
            && root.TryGetProperty("cutoff", out var c)
            && c.ValueKind == JsonValueKind.Object)
        {
            var kind = c.TryGetProperty("kind", out var k) ? k.GetString() : null;
            double? threshold = c.TryGetProperty("threshold", out var t) && t.ValueKind == JsonValueKind.Number
                ? t.GetDouble() : null;
            double? floor = c.TryGetProperty("observed_floor", out var f) && f.ValueKind == JsonValueKind.Number
                ? f.GetDouble() : null;
            cutoff = (kind, threshold) switch
            {
                ("hard_cutoff", not null) => new Cutoff(CutoffKind.Around, threshold, floor),
                ("soft_preference", _) => new Cutoff(CutoffKind.SkewsHigh, null, floor),
                ("no_cgpa_filter", _) => new Cutoff(CutoffKind.NoFilter, null, null),
                // A cutoff this version cannot read is left out, not guessed at.
                _ => null,
            };
        }

        var histogram = new List<Bucket>();
        if (parsed == AnalysisStatus.Ready
            && root.TryGetProperty("histogram", out var h)
            && h.ValueKind == JsonValueKind.Array)
        {
            foreach (var bucket in h.EnumerateArray())
            {
                histogram.Add(new Bucket(
                    bucket.GetProperty("lower").GetDouble(),
                    bucket.GetProperty("upper").GetDouble(),
                    bucket.GetProperty("count").GetInt32()));
            }
        }

        var branches = new List<string>();
        if (parsed == AnalysisStatus.Ready
            && root.TryGetProperty("over_represented", out var o)
            && o.ValueKind == JsonValueKind.Array)
        {
            foreach (var branch in o.EnumerateArray())
            {
                if (branch.GetString() is { } name) branches.Add(name);
            }
        }

        return new AnalysisSummary(
            parsed,
            root.TryGetProperty("matched", out var m) ? m.GetInt32() : 0,
            root.TryGetProperty("coverage", out var cov) ? cov.GetDouble() : 0,
            cutoff,
            parsed == AnalysisStatus.Ready && root.TryGetProperty("median", out var med)
                && med.ValueKind == JsonValueKind.Number ? med.GetDouble() : null,
            histogram,
            branches);
    }

    public override void Write(Utf8JsonWriter writer, AnalysisSummary value, JsonSerializerOptions options) =>
        throw new NotSupportedException("the widget never writes snapshots");
}
