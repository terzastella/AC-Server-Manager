import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { JoinWindow } from "./JoinWindow";
import { LauncherHome } from "./LauncherHome";
import { api } from "./api";
import { LanguageProvider } from "./i18n/core";
import "./styles.css";

function Root() {
  const [mode, setMode] = useState<"loading" | "join" | "launcher" | "app">("loading");

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const context = await api.joinContext();
        if (cancelled) return;
        if (context) {
          setMode("join");
          return;
        }
        const info = await api.info();
        if (!cancelled) setMode(info.launcher_mode ? "launcher" : "app");
      } catch {
        if (!cancelled) setMode("app");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  if (mode === "loading") return null;
  if (mode === "join") return <JoinWindow />;
  if (mode === "launcher") return <LauncherHome />;
  return <App />;
}

class Boundary extends React.Component<{ children: React.ReactNode }, { error: string }> {
  state = { error: "" };

  static getDerivedStateFromError(error: unknown) {
    return { error: String(error) };
  }

  render() {
    if (this.state.error) {
      return (
        <div className="join-page">
          <div className="join-card join-error">
            <h1>Qualcosa è andato storto · Something went wrong</h1>
            <p>Chiudi e riapri il launcher. Dettaglio · Close and reopen the launcher. Detail: {this.state.error}</p>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <LanguageProvider>
      <Boundary>
        <Root />
      </Boundary>
    </LanguageProvider>
  </React.StrictMode>,
);
