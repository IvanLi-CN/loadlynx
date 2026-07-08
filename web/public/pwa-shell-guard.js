(async () => {
  const currentScript = document.currentScript;
  if (!(currentScript instanceof HTMLScriptElement)) {
    return;
  }

  const shellVersionRaw = currentScript.dataset.shellVersion || "";
  const appEntry = currentScript.dataset.appEntry || "";
  const shellVersion = shellVersionRaw.startsWith("%")
    ? ""
    : shellVersionRaw.trim();
  const recoveryParam = "__ll_sw_recover";
  const recoveryNonceParam = "__ll_shell_reload";
  const recoverySessionKey = "loadlynx.shell.recovery";

  function renderRecoveryNotice(mode, remoteVersion) {
    const root = document.getElementById("root");
    if (!root) return;

    root.innerHTML = `
      <main style="min-height:100vh;display:flex;align-items:center;justify-content:center;padding:2rem;box-sizing:border-box;">
        <section style="max-width:38rem;width:100%;border-radius:1rem;padding:1.5rem 1.75rem;background:radial-gradient(circle at top, rgba(56,189,248,0.16), transparent 55%), #020617;box-shadow:0 24px 60px rgba(15,23,42,0.9), 0 0 0 1px rgba(148,163,184,0.15);color:#e2e8f0;font-family:ui-sans-serif,system-ui,sans-serif;">
          <h1 style="margin:0 0 0.75rem;font-size:1.25rem;line-height:1.2;">LoadLynx Web Console</h1>
          <p style="margin:0;color:#cbd5e1;line-height:1.6;">
            ${
              mode === "recovering"
                ? `检测到较新的 Web 版本 ${remoteVersion}，正在切换到最新控制台……`
                : `检测到旧缓存壳仍在接管页面。请关闭这个标签页后重新打开控制台；最新版本是 ${remoteVersion}。`
            }
          </p>
        </section>
      </main>
    `;
  }

  async function clearServiceWorkersAndCaches() {
    if ("serviceWorker" in navigator) {
      const registrations = await navigator.serviceWorker
        .getRegistrations()
        .catch(() => []);
      await Promise.all(
        registrations.map((registration) =>
          registration.unregister().catch(() => false),
        ),
      );
    }

    if ("caches" in window) {
      const keys = await caches.keys().catch(() => []);
      await Promise.all(
        keys.map((key) => caches.delete(key).catch(() => false)),
      );
    }
  }

  async function recoverIfShellIsStale() {
    if (!shellVersion || typeof fetch !== "function") {
      return false;
    }

    try {
      const currentUrl = new URL(window.location.href);
      const response = await fetch(
        `/version.json?ll-shell-check=${encodeURIComponent(shellVersion)}`,
        {
          cache: "no-store",
        },
      );
      if (!response.ok) {
        return false;
      }

      const payload = await response.json();
      const remoteVersion =
        typeof payload?.version === "string" ? payload.version.trim() : "";

      if (!remoteVersion || remoteVersion === shellVersion) {
        sessionStorage.removeItem(recoverySessionKey);
        return false;
      }

      const attemptKey = `${shellVersion}->${remoteVersion}@${currentUrl.pathname}`;
      const alreadyRecovering =
        sessionStorage.getItem(recoverySessionKey) === attemptKey &&
        currentUrl.searchParams.get(recoveryParam) === remoteVersion;

      if (alreadyRecovering) {
        renderRecoveryNotice("stalled", remoteVersion);
        return true;
      }

      sessionStorage.setItem(recoverySessionKey, attemptKey);
      renderRecoveryNotice("recovering", remoteVersion);
      await clearServiceWorkersAndCaches();
      currentUrl.searchParams.set(recoveryParam, remoteVersion);
      currentUrl.searchParams.set(recoveryNonceParam, Date.now().toString(36));
      window.location.replace(currentUrl.toString());
      return true;
    } catch {
      return false;
    }
  }

  function loadAppEntry() {
    if (!appEntry || appEntry === "__LOADLYNX_APP_ENTRY__") {
      return;
    }

    const script = document.createElement("script");
    script.type = "module";
    script.src = appEntry;
    document.body.appendChild(script);
  }

  if (!(await recoverIfShellIsStale())) {
    loadAppEntry();
  }
})();
