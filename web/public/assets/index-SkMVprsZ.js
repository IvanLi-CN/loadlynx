const legacyRecoverySessionKey = "loadlynx.legacy.entry.recovery";
const legacyRecoveryParam = "__ll_legacy_entry_recover";
const legacyRecoveryNonceParam = "__ll_legacy_entry_nonce";
const legacyEntryId = "index-SkMVprsZ";

function renderLegacyRecoveryNotice(message) {
  const root = document.getElementById("root");
  if (!root) return;

  root.innerHTML = `
    <main style="min-height:100vh;display:flex;align-items:center;justify-content:center;padding:2rem;box-sizing:border-box;">
      <section style="max-width:38rem;width:100%;border-radius:1rem;padding:1.5rem 1.75rem;background:radial-gradient(circle at top, rgba(56,189,248,0.16), transparent 55%), #020617;box-shadow:0 24px 60px rgba(15,23,42,0.9), 0 0 0 1px rgba(148,163,184,0.15);color:#e2e8f0;font-family:ui-sans-serif,system-ui,sans-serif;">
        <h1 style="margin:0 0 0.75rem;font-size:1.25rem;line-height:1.2;">LoadLynx Web Console</h1>
        <p style="margin:0;color:#cbd5e1;line-height:1.6;">${message}</p>
      </section>
    </main>
  `;
}

async function clearLegacyCaches() {
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
    await Promise.all(keys.map((key) => caches.delete(key).catch(() => false)));
  }
}

(async () => {
  const currentUrl = new URL(window.location.href);
  const recoveryAttempt = `${legacyEntryId}@${currentUrl.pathname}`;
  const alreadyRecovering =
    sessionStorage.getItem(legacyRecoverySessionKey) === recoveryAttempt &&
    currentUrl.searchParams.has(legacyRecoveryParam);

  if (alreadyRecovering) {
    renderLegacyRecoveryNotice(
      "旧缓存资源仍在接管页面。请关闭所有 LoadLynx 标签页后重新打开控制台。",
    );
    return;
  }

  sessionStorage.setItem(legacyRecoverySessionKey, recoveryAttempt);
  renderLegacyRecoveryNotice("检测到旧版离线缓存，正在恢复最新控制台……");
  await clearLegacyCaches();
  currentUrl.searchParams.set(legacyRecoveryParam, legacyEntryId);
  currentUrl.searchParams.set(
    legacyRecoveryNonceParam,
    Date.now().toString(36),
  );
  window.location.replace(currentUrl.toString());
})();
