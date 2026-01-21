export type AppVersionLinkProps = {
  version: string | null;
  repo: string | null;
  sha: string | null;
  tag: string | null;
};

function isStableReleaseTag(tag: string | null): boolean {
  return tag?.startsWith("v") ?? false;
}

export function AppVersionLink({
  version,
  repo,
  sha,
  tag,
}: AppVersionLinkProps) {
  const normalizedVersion = version?.trim() || null;
  if (!normalizedVersion) return null;

  const normalizedRepo = repo?.trim() || null;
  const normalizedSha = sha?.trim() || null;
  const normalizedTag = tag?.trim() || null;

  const baseUrl = normalizedRepo
    ? `https://github.com/${normalizedRepo}`
    : null;

  const href =
    baseUrl && isStableReleaseTag(normalizedTag)
      ? `${baseUrl}/tree/${normalizedTag}`
      : baseUrl && normalizedSha
        ? `${baseUrl}/commit/${normalizedSha}`
        : null;

  const className = [
    "text-xs font-mono",
    href ? "link link-hover" : "opacity-70",
  ]
    .filter(Boolean)
    .join(" ");

  if (!href) {
    return (
      <span className={className} title={normalizedVersion}>
        {normalizedVersion}
      </span>
    );
  }

  return (
    <a
      className={className}
      href={href}
      target="_blank"
      rel="noreferrer"
      title={href}
      aria-label={`Open ${normalizedVersion} on GitHub`}
    >
      {normalizedVersion}
    </a>
  );
}
