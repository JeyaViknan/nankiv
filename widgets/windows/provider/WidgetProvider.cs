// The Windows widget provider.
//
// Windows widgets are driven by a COM server the Widgets host activates: it
// asks for content, and the provider answers with an Adaptive Card template
// plus its data. nankiv answers from the snapshot the app publishes, and
// watches that file so a new shortlist appears without anyone polling.
//
// NOT BUILT OR RUN YET. This is written against Microsoft.Windows.Widgets
// (Windows App SDK 1.6+) and needs Windows and an MSIX package to compile and
// register — see widgets/windows/README.md for what remains.

using System.Text.Json.Nodes;
using Microsoft.Windows.Widgets;
using Microsoft.Windows.Widgets.Providers;

namespace Nankiv.Widgets;

public sealed class NankivWidgetProvider : IWidgetProvider, IDisposable
{
    /// <summary>Must match the widget id in Package.appxmanifest.</summary>
    public const string DefinitionId = "Shortlist";

    private readonly Dictionary<string, PinnedWidget> _widgets = new();
    private readonly FileSystemWatcher? _watcher;
    private readonly object _lock = new();

    public NankivWidgetProvider()
    {
        var directory = Path.GetDirectoryName(SnapshotReader.DefaultPath);
        if (directory is null) return;
        Directory.CreateDirectory(directory);

        // The app rewrites the snapshot by renaming a temporary file into
        // place, so a rename is the signal — not a timer.
        _watcher = new FileSystemWatcher(directory, Path.GetFileName(SnapshotReader.DefaultPath))
        {
            NotifyFilter = NotifyFilters.FileName | NotifyFilters.LastWrite | NotifyFilters.Size,
            EnableRaisingEvents = true,
        };
        _watcher.Changed += (_, _) => UpdateAll();
        _watcher.Created += (_, _) => UpdateAll();
        _watcher.Renamed += (_, _) => UpdateAll();
        _watcher.Deleted += (_, _) => UpdateAll();
    }

    public void CreateWidget(WidgetContext context)
    {
        lock (_lock)
        {
            _widgets[context.Id] = new PinnedWidget(context.Id, context.Size, active: true);
        }
        Update(context.Id);
    }

    public void DeleteWidget(string widgetId, string customState)
    {
        lock (_lock) { _widgets.Remove(widgetId); }
    }

    public void OnWidgetContextChanged(WidgetContextChangedArgs args)
    {
        lock (_lock)
        {
            if (_widgets.TryGetValue(args.WidgetContext.Id, out var widget))
            {
                _widgets[args.WidgetContext.Id] = widget with { Size = args.WidgetContext.Size };
            }
        }
        Update(args.WidgetContext.Id);
    }

    public void Activate(WidgetContext context)
    {
        lock (_lock)
        {
            _widgets[context.Id] = _widgets.TryGetValue(context.Id, out var widget)
                ? widget with { Size = context.Size, Active = true }
                : new PinnedWidget(context.Id, context.Size, active: true);
        }
        Update(context.Id);
    }

    public void Deactivate(string widgetId)
    {
        lock (_lock)
        {
            if (_widgets.TryGetValue(widgetId, out var widget))
            {
                _widgets[widgetId] = widget with { Active = false };
            }
        }
    }

    /// <summary>
    /// Nothing on the card sends a verb back: every tap is a link into the app.
    /// A widget that cannot be told to do anything cannot be told to do
    /// something unintended.
    /// </summary>
    public void OnActionInvoked(WidgetActionInvokedArgs args)
    {
    }

    private void UpdateAll()
    {
        string[] ids;
        lock (_lock) { ids = _widgets.Where(w => w.Value.Active).Select(w => w.Key).ToArray(); }
        foreach (var id in ids) Update(id);
    }

    private void Update(string widgetId)
    {
        PinnedWidget widget;
        lock (_lock)
        {
            if (!_widgets.TryGetValue(widgetId, out widget!)) return;
        }

        var glance = Glance.Make(SnapshotReader.Load(), pinned: null, DateTimeOffset.Now);
        JsonObject data;
        try
        {
            data = CardData.Build(glance, DateTimeOffset.Now);
        }
        catch (Exception)
        {
            // A widget that cannot be built is left as it was rather than
            // replaced with an error.
            return;
        }

        var options = new WidgetUpdateRequestOptions(widgetId)
        {
            Template = Cards.Template(widget.Size),
            Data = CardData.Serialise(data),
            CustomState = string.Empty,
        };
        WidgetManager.GetDefault().UpdateWidget(options);
    }

    public void Dispose() => _watcher?.Dispose();

    private sealed record PinnedWidget(string Id, WidgetSize Size, bool Active)
    {
        public bool Active { get; init; } = Active;
        public WidgetSize Size { get; init; } = Size;
    }
}

/// <summary>The Adaptive Card templates, shipped beside the provider.</summary>
public static class Cards
{
    private static readonly Dictionary<string, string> Cache = new();

    public static string Template(WidgetSize size) => Load(size switch
    {
        WidgetSize.Small => "small",
        WidgetSize.Large => "large",
        _ => "medium",
    });

    private static string Load(string name)
    {
        lock (Cache)
        {
            if (Cache.TryGetValue(name, out var cached)) return cached;
            var path = Path.Combine(AppContext.BaseDirectory, "cards", $"{name}.json");
            var json = File.ReadAllText(path);
            Cache[name] = json;
            return json;
        }
    }
}
