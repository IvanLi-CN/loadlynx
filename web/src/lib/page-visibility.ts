import { useEffect, useState } from "react";

export interface PageVisibilityDocumentLike {
  visibilityState: string;
  addEventListener(type: "visibilitychange", listener: () => void): void;
  removeEventListener(type: "visibilitychange", listener: () => void): void;
}

export function readPageVisibility(
  doc: Pick<PageVisibilityDocumentLike, "visibilityState"> | null | undefined,
): boolean {
  return !doc || doc.visibilityState === "visible";
}

export function observePageVisibility(
  doc: PageVisibilityDocumentLike | null | undefined,
  onChange: (visible: boolean) => void,
): () => void {
  if (!doc) {
    return () => {};
  }

  const handleVisibility = () => {
    onChange(readPageVisibility(doc));
  };

  doc.addEventListener("visibilitychange", handleVisibility);
  return () => {
    doc.removeEventListener("visibilitychange", handleVisibility);
  };
}

export function usePageVisibility(): boolean {
  const [isPageVisible, setIsPageVisible] = useState(() =>
    readPageVisibility(typeof document === "undefined" ? null : document),
  );

  useEffect(() => {
    return observePageVisibility(
      typeof document === "undefined" ? null : document,
      setIsPageVisible,
    );
  }, []);

  return isPageVisible;
}
