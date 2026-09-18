// Renders every widget state at every size, in each appearance, to PNG.
//
//   swift run nankiv-widget-render [output-directory]
//
// A widget can only be seen for real on a desktop, which a build machine does
// not have. This draws the same views with the same fixtures into contact
// sheets — one per state — so a layout change can be reviewed as an image.
//
// It approximates the system's treatment rather than reproducing it: vibrant
// and accented rendering are done by the window server, so here they are
// simulated by setting the rendering mode (which the views respond to) and
// desaturating. Final judgement belongs to Xcode previews and a real desktop.

import AppKit
import NankivWidgetModel
import NankivWidgetViews
import SwiftUI
import WidgetKit

let fixtures = URL(fileURLWithPath: #filePath)
    .deletingLastPathComponent()  // main.swift
    .deletingLastPathComponent()  // WidgetRender
    .deletingLastPathComponent()  // Sources
    .deletingLastPathComponent()  // Core
    .deletingLastPathComponent()  // macos
    .appendingPathComponent("fixtures")

let output = URL(
    fileURLWithPath: CommandLine.arguments.dropFirst().first ?? "widget-renders", isDirectory: true)

let now = ISO8601DateFormatter().date(from: "2026-09-17T15:01:00Z")!

func load(_ name: String) -> SnapshotLoad {
    guard let data = try? Data(contentsOf: fixtures.appendingPathComponent("\(name).json")) else {
        fatalError("missing fixture \(name)")
    }
    return SnapshotSource.decode(data)
}

func pinnedID(_ name: String, company: String) -> Int64? {
    guard case .loaded(let s) = load(name) else { return nil }
    return s.drives.first { $0.company == company }?.id
}

let states: [(String, Glance)] = [
    ("ready", Glance.make(from: load("ready"), pinned: nil, at: now)),
    ("importing", Glance.make(from: load("importing"), pinned: nil, at: now)),
    ("failed", Glance.make(from: load("failed"), pinned: nil, at: now)),
    ("not-shortlisted", Glance.make(from: load("not_shortlisted"), pinned: nil, at: now)),
    ("insufficient", Glance.make(from: load("insufficient"), pinned: nil, at: now)),
    ("soft-preference", Glance.make(from: load("soft_preference"), pinned: nil, at: now)),
    ("undetermined", Glance.make(from: load("undetermined"), pinned: nil, at: now)),
    ("not-set-up", Glance.make(from: load("not_set_up"), pinned: nil, at: now)),
    ("pinned", Glance.make(from: load("ready"), pinned: pinnedID("ready", company: "Cedar Labs"), at: now)),
    ("no-identity", Glance.make(from: load("no_identity"), pinned: nil, at: now)),
    ("empty", Glance.make(from: load("empty"), pinned: nil, at: now)),
    ("empty-importing", .noShortlists(.importing(filename: "Fjord Robotics shortlist.xlsx"))),
    ("removed", Glance.make(from: load("ready"), pinned: 999_999, at: now)),
    ("not-started", Glance.make(from: .missing, pinned: nil, at: now)),
    ("needs-refresh", Glance.make(from: .stale, pinned: nil, at: now)),
    ("resting", Glance.make(from: load("ready"), pinned: nil, at: now.addingTimeInterval(13 * 3600))),
    ("resting-importing", Glance.make(from: load("importing"), pinned: nil, at: now.addingTimeInterval(13 * 3600))),
    ("placeholder", .sample(now: now)),
]

/// Approximate Mac desktop sizes. Margins are the generous 16pt, so a layout
/// that fits here fits the desktop's tighter margins too.
let families: [(WidgetFamily, CGSize)] = [
    (.systemSmall, CGSize(width: 170, height: 170)),
    (.systemMedium, CGSize(width: 364, height: 170)),
    (.systemLarge, CGSize(width: 364, height: 382)),
    (.systemExtraLarge, CGSize(width: 764, height: 382)),
]
let margin: CGFloat = 16

/// `APPEARANCES=light,dark` renders a subset, for a closer look.
let appearances: [Appearance] = {
    guard let raw = ProcessInfo.processInfo.environment["APPEARANCES"] else { return Appearance.allCases }
    return raw.split(separator: ",").compactMap { Appearance(rawValue: String($0)) }
}()

enum Appearance: String, CaseIterable {
    case light, dark, vibrant, accented, largeText = "large text"
}

struct Tile: View {
    let glance: Glance
    let family: WidgetFamily
    let size: CGSize
    let appearance: Appearance
    let redacted: Bool

    var body: some View {
        GlanceView(glance: glance, family: family, now: now)
            // Links are live controls that only the widget host can draw.
            .environment(\.glanceDrawsLinksAsLabels, true)
            .padding(margin)
            .frame(width: size.width, height: size.height)
            .environment(\.widgetRenderingMode, mode)
            .environment(\.dynamicTypeSize, appearance == .largeText ? .xxxLarge : .large)
            .redacted(reason: redacted ? .placeholder : [])
            .background {
                switch appearance {
                case .light, .dark, .largeText:
                    GlanceBackground()
                case .vibrant, .accented:
                    Color.clear
                }
            }
            .grayscale(appearance == .vibrant || appearance == .accented ? 1 : 0)
            .clipShape(RoundedRectangle(cornerRadius: 22, style: .continuous))
            .background {
                if appearance == .accented {
                    // Standing in for tinted glass.
                    RoundedRectangle(cornerRadius: 22, style: .continuous)
                        .fill(Color(red: 0.23, green: 0.34, blue: 0.52).opacity(0.85))
                        .overlay(
                            RoundedRectangle(cornerRadius: 22, style: .continuous)
                                .strokeBorder(.white.opacity(0.35), lineWidth: 1))
                }
            }
            .environment(\.colorScheme, appearance == .light || appearance == .largeText ? .light : .dark)
            .environment(\.locale, Locale(identifier: "en_IN"))
            .environment(\.calendar, {
                var c = Calendar(identifier: .gregorian)
                c.timeZone = TimeZone(identifier: "Asia/Kolkata")!
                c.locale = Locale(identifier: "en_IN")
                return c
            }())
    }

    private var mode: WidgetRenderingMode {
        switch appearance {
        case .vibrant: .vibrant
        case .accented: .accented
        default: .fullColor
        }
    }
}

struct Sheet: View {
    let name: String
    let glance: Glance

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            Text(name).font(.system(size: 20, weight: .bold)).foregroundStyle(.white)
            ForEach(appearances, id: \.self) { appearance in
                VStack(alignment: .leading, spacing: 6) {
                    Text(appearance.rawValue).font(.system(size: 12, weight: .medium)).foregroundStyle(.white.opacity(0.7))
                    HStack(alignment: .top, spacing: 18) {
                        ForEach(families.indices, id: \.self) { i in
                            let (family, size) = families[i]
                            Tile(
                                glance: glance, family: family, size: size, appearance: appearance,
                                redacted: name == "placeholder")
                        }
                    }
                }
            }
        }
        .padding(24)
        .background(
            LinearGradient(
                colors: [Color(red: 0.36, green: 0.42, blue: 0.55), Color(red: 0.15, green: 0.17, blue: 0.24)],
                startPoint: .topLeading, endPoint: .bottomTrailing)
        )
    }
}

@MainActor
func render() throws {
    try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
    let only = ProcessInfo.processInfo.environment["ONLY"]
    for (name, glance) in states where only == nil || only == name {
        let renderer = ImageRenderer(content: Sheet(name: name, glance: glance))
        renderer.scale = 2
        guard let image = renderer.nsImage,
            let tiff = image.tiffRepresentation,
            let png = NSBitmapImageRep(data: tiff)?.representation(using: .png, properties: [:])
        else {
            throw CocoaError(.fileWriteUnknown)
        }
        let url = output.appendingPathComponent("\(name).png")
        try png.write(to: url)
        print(url.path)
    }
}

MainActor.assumeIsolated {
    _ = NSApplication.shared
    do {
        try render()
    } catch {
        FileHandle.standardError.write(Data("render failed: \(error)\n".utf8))
        exit(1)
    }
}
