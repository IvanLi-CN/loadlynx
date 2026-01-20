import { formatFixed } from "./format.ts";

export type FixedNumberProps = {
  value: number | null | undefined;
  digits: number;
};

export function FixedNumber({ value, digits }: FixedNumberProps) {
  const text = formatFixed(value, digits);
  if (text === "—") {
    return <span>—</span>;
  }

  const [intPart, fracPart] = text.split(".");
  if (fracPart === undefined) {
    return <span>{text}</span>;
  }

  return (
    <span className="instrument-number">
      <span>{intPart}</span>
      <span className="instrument-number-dot">.</span>
      <span>{fracPart}</span>
    </span>
  );
}

