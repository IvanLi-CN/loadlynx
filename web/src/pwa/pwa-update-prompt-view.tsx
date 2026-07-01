import { CheckCircle2, RefreshCw, Wifi, XCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button.tsx";

export type PwaUpdatePromptState =
  | "hidden"
  | "offline-ready"
  | "update-ready"
  | "registration-error";

export interface PwaUpdatePromptViewProps {
  state: PwaUpdatePromptState;
  errorMessage?: string | null;
  onClose: () => void;
  onUpdate: () => void;
}

export function PwaUpdatePromptView({
  errorMessage,
  onClose,
  onUpdate,
  state,
}: PwaUpdatePromptViewProps) {
  const { t } = useTranslation();

  if (state === "hidden") return null;

  const isUpdate = state === "update-ready";
  const isError = state === "registration-error";
  const Icon = isUpdate ? RefreshCw : isError ? XCircle : CheckCircle2;
  const title = isUpdate
    ? t("pwa.updateReadyTitle")
    : isError
      ? t("pwa.registrationErrorTitle")
      : t("pwa.offlineReadyTitle");
  const body = isUpdate
    ? t("pwa.updateReadyBody")
    : isError
      ? (errorMessage ?? t("pwa.registrationErrorBody"))
      : t("pwa.offlineReadyBody");

  return (
    <section
      aria-live="polite"
      className="ll-pwa-toast"
      role={isError ? "alert" : "status"}
    >
      <div
        className={[
          "ll-alert ll-pwa-toast-alert",
          isUpdate
            ? "ll-alert-info"
            : isError
              ? "ll-alert-warning"
              : "ll-alert-success",
        ].join(" ")}
      >
        <div className="ll-pwa-toast-icon" aria-hidden="true">
          <Icon size={18} />
        </div>
        <div className="min-w-0 flex-1">
          <div className="ll-pwa-toast-title">{title}</div>
          <div className="ll-pwa-toast-body">
            {state === "offline-ready" ? (
              <span className="inline-flex items-center gap-1.5">
                <Wifi size={14} aria-hidden="true" />
                {body}
              </span>
            ) : (
              body
            )}
          </div>
        </div>
        <div className="ll-pwa-toast-actions">
          {isUpdate ? (
            <Button
              type="button"
              variant="primary"
              size="xs"
              onClick={onUpdate}
            >
              {t("pwa.updateAction")}
            </Button>
          ) : null}
          <Button type="button" variant="ghost" size="xs" onClick={onClose}>
            {t("pwa.dismissAction")}
          </Button>
        </div>
      </div>
    </section>
  );
}
