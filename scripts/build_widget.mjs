// Builds the desktop widget before the app is bundled.
//
// Runs as part of `tauri build`. Only macOS has a widget to build today, and
// only Xcode can build it, so everywhere else this exits quietly rather than
// failing a release.
import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";

if (process.platform !== "darwin") {
  console.log("widget: nothing to build on " + process.platform);
  process.exit(0);
}

try {
  execFileSync("xcodebuild", ["-version"], { stdio: "ignore" });
} catch {
  console.error("widget: Xcode is required to build the macOS widget");
  process.exit(1);
}

const script = new URL("../widgets/macos/build.sh", import.meta.url).pathname;
if (!existsSync(script)) {
  console.error("widget: " + script + " is missing");
  process.exit(1);
}
execFileSync(script, [], { stdio: "inherit" });
