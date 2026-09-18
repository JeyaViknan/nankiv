// swift-tools-version: 6.0
//
// The macOS widget's shared code, kept outside the Xcode project so it can be
// built and tested from the command line, in CI, without a signing identity.
//
//   NankivWidgetModel   The snapshot contract, and what to show when. Foundation
//                       only — no views, no WidgetKit — so every decision the
//                       widget makes is unit-tested.
//   NankivWidgetViews   How each size draws a glance. SwiftUI and WidgetKit.
//   nankiv-widget-render
//                       Renders every state at every size and appearance to
//                       PNG, for reviewing layouts without a desktop.
//
// The widget extension itself (widget definition, timeline provider and
// configuration intent) lives in ../Extension, because App Intents metadata is
// extracted from the extension target at build time.

import PackageDescription

let package = Package(
    name: "NankivWidgetCore",
    platforms: [.macOS(.v14)],
    products: [
        .library(name: "NankivWidgetModel", targets: ["NankivWidgetModel"]),
        .library(name: "NankivWidgetViews", targets: ["NankivWidgetViews"]),
        .executable(name: "nankiv-widget-render", targets: ["WidgetRender"]),
    ],
    targets: [
        .target(name: "NankivWidgetModel"),
        .target(name: "NankivWidgetViews", dependencies: ["NankivWidgetModel"]),
        .executableTarget(
            name: "WidgetRender",
            dependencies: ["NankivWidgetModel", "NankivWidgetViews"]
        ),
        .testTarget(
            name: "NankivWidgetModelTests",
            dependencies: ["NankivWidgetModel"]
        ),
    ]
)
