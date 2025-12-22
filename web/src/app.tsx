import { useEffect, useState } from "react";

type AppVersion = {
  version: string;
  builtAt: string;
};

function isStorybookRuntime(): boolean {
  return globalThis.__LOADLYNX_STORYBOOK__ === true;
}

export function App() {
  const [versionInfo, setVersionInfo] = useState<AppVersion | null>(null);
  const storybookRuntime = isStorybookRuntime();

  useEffect(() => {
    if (storybookRuntime) {
      return;
    }
    void fetch("/version.json")
      .then(async (response) => {
        if (!response.ok) {
          return null;
        }
        return (await response.json()) as AppVersion;
      })
      .then((payload) => {
        if (payload) {
          setVersionInfo(payload);
        }
      })
      .catch(() => {
        // Best-effort only; version is not critical at this stage.
      });
  }, [storybookRuntime]);

  return (
    <main
      style={{
        minHeight: "100vh",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        padding: "2rem",
        boxSizing: "border-box",
      }}
    >
      <section
        style={{
          maxWidth: "640px",
          width: "100%",
          borderRadius: "0.75rem",
          padding: "2rem",
          background:
            "radial-gradient(circle at top, rgba(56,189,248,0.15), transparent 55%), #020617",
          boxShadow:
            "0 24px 60px rgba(15,23,42,0.9), 0 0 0 1px rgba(148,163,184,0.15)",
        }}
      >
        <header style={{ marginBottom: "1.5rem" }}>
          <h1
            style={{
              fontSize: "1.75rem",
              lineHeight: 1.2,
              margin: 0,
              marginBottom: "0.5rem",
            }}
          >
            LoadLynx Web Console{" "}
            <span style={{ opacity: 0.7 }}>(scaffold)</span>
          </h1>
          <p
            style={{
              margin: 0,
              color: "#9ca3af",
              fontSize: "0.95rem",
            }}
          >
            Frontend shell for the LoadLynx network control console. Routing,
            device management, and CC control panels will be added in later
            tasks.
          </p>
        </header>
        <dl
          style={{
            margin: 0,
            display: "grid",
            gridTemplateColumns: "minmax(0, 1fr) minmax(0, 2fr)",
            rowGap: "0.5rem",
            columnGap: "1.5rem",
            fontSize: "0.9rem",
          }}
        >
          <dt style={{ color: "#9ca3af" }}>Project</dt>
          <dd style={{ margin: 0 }}>LoadLynx Web Console</dd>
          <dt style={{ color: "#9ca3af" }}>Build</dt>
          <dd style={{ margin: 0 }}>
            {storybookRuntime
              ? "storybook"
              : versionInfo
                ? versionInfo.version
                : "dev (version.json pending)"}
          </dd>
          {versionInfo ? (
            <>
              <dt style={{ color: "#9ca3af" }}>Built at</dt>
              <dd style={{ margin: 0 }}>
                {new Date(versionInfo.builtAt).toLocaleString()}
              </dd>
            </>
          ) : null}
        </dl>
      </section>
    </main>
  );
}

export default App;
