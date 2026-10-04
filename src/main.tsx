import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { initTheme } from "./lib/theme";
import { IS_MAC } from "./lib/keys";
import "./styles/app.css";

// The inline script in index.html already stamped an explicit choice; this
// reconciles the full three-state preference once the module graph is live.
initTheme();

// Layout that differs by platform — the space a Mac keeps for its traffic
// lights — keys off this, rather than every component asking.
document.documentElement.dataset.platform = IS_MAC ? "mac" : "other";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
