import { cn } from "../../lib/utils.ts";

export type SliderRadioGroupSize = "sm" | "md" | "lg";
export type SliderRadioGroupWidthMode = "fit" | "fill";

export type SliderRadioGroupOption<T extends string> = {
  value: T;
  label: string;
  disabled?: boolean;
};

export type SliderRadioGroupProps<T extends string> = {
  value: T;
  onValueChange: (value: T) => void;
  options: Array<SliderRadioGroupOption<T>>;
  size?: SliderRadioGroupSize;
  widthMode?: SliderRadioGroupWidthMode;
  ariaLabel?: string;
  className?: string;
};

const shellClassBySize: Record<SliderRadioGroupSize, string> = {
  sm: "ll-slider-radio-group--sm",
  md: "ll-slider-radio-group--md",
  lg: "ll-slider-radio-group--lg",
};

const itemClassBySize: Record<SliderRadioGroupSize, string> = {
  sm: "text-xs",
  md: "text-sm",
  lg: "text-base",
};

const shellClassByWidthMode: Record<SliderRadioGroupWidthMode, string> = {
  fit: "w-fit max-w-full overflow-x-auto",
  fill: "w-full overflow-x-hidden",
};

const trackClassByWidthMode: Record<SliderRadioGroupWidthMode, string> = {
  fit: "w-max",
  fill: "w-full",
};

const itemClassByWidthMode: Record<SliderRadioGroupWidthMode, string> = {
  fit: "shrink-0",
  fill: "min-w-0 flex-1",
};

export function SliderRadioGroup<T extends string>({
  value,
  onValueChange,
  options,
  size = "md",
  widthMode = "fit",
  ariaLabel,
  className,
}: SliderRadioGroupProps<T>) {
  return (
    <div
      role="radiogroup"
      aria-label={ariaLabel}
      data-size={size}
      className={cn(
        "ll-slider-radio-group min-w-0 rounded-[1.1rem] border border-cyan-400/18 bg-black/28 shadow-[inset_0_0_0_1px_rgba(17,33,45,0.6)]",
        shellClassByWidthMode[widthMode],
        shellClassBySize[size],
        className,
      )}
    >
      <div
        className={cn(
          "ll-slider-radio-group__track inline-flex items-center",
          trackClassByWidthMode[widthMode],
        )}
      >
        {options.map((option) => {
          const selected = option.value === value;
          const disabled = Boolean(option.disabled);
          return (
            <label
              key={option.value}
              className={cn(
                "ll-slider-radio-group__item relative inline-flex whitespace-nowrap items-center justify-center border font-semibold tracking-[0.14em] uppercase transition-[border-color,background-color,color,box-shadow] duration-200",
                itemClassBySize[size],
                itemClassByWidthMode[widthMode],
                selected
                  ? "border-cyan-300/34 bg-cyan-400/14 text-slate-50 shadow-[0_0_0_1px_rgba(45,222,255,0.28),0_0_24px_rgba(43,209,255,0.12)]"
                  : "border-slate-400/10 bg-transparent text-slate-200/60",
                disabled
                  ? "cursor-not-allowed opacity-35"
                  : selected
                    ? "cursor-pointer"
                    : "cursor-pointer hover:border-cyan-400/18 hover:text-slate-100",
              )}
            >
              <input
                type="radio"
                name={ariaLabel ?? "slider-radio-group"}
                value={option.value}
                checked={selected}
                disabled={disabled}
                onChange={() => {
                  if (!disabled && option.value !== value) {
                    onValueChange(option.value);
                  }
                }}
                className="ll-slider-radio-group__input"
                aria-label={option.label}
              />
              {option.label}
            </label>
          );
        })}
      </div>
    </div>
  );
}
