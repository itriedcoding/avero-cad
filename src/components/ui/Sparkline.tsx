import { useId } from "react";

export function Sparkline({
  values,
  height = 56,
  label,
}: {
  values: number[];
  height?: number;
  label?: string;
}) {
  const gradientId = useId();
  if (values.length < 2) {
    return (
      <div
        className="flex items-center justify-center rounded border border-dashed border-line text-[11px] text-faint"
        style={{ height }}
      >
        Waiting for more samples
      </div>
    );
  }

  const width = 320;
  const max = Math.max(...values);
  const min = Math.min(...values);
  const span = Math.max(1, max - min);
  const stepX = width / (values.length - 1);
  const points = values.map((value, index) => {
    const x = index * stepX;
    const y = height - 4 - ((value - min) / span) * (height - 10);
    return [x, y] as const;
  });
  const line = points.map(([x, y]) => `${x.toFixed(1)},${y.toFixed(1)}`).join(" ");
  const area = `0,${height} ${line} ${width},${height}`;

  return (
    <div className="relative">
      <svg
        viewBox={`0 0 ${width} ${height}`}
        preserveAspectRatio="none"
        width="100%"
        height={height}
        role="img"
        aria-label={label ?? `Player count trend between ${min} and ${max}`}
      >
        <defs>
          <linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="rgba(255,255,255,0.16)" />
            <stop offset="100%" stopColor="rgba(255,255,255,0)" />
          </linearGradient>
        </defs>
        <polygon points={area} fill={`url(#${gradientId})`} />
        <polyline
          points={line}
          fill="none"
          stroke="rgba(255,255,255,0.75)"
          strokeWidth="1.25"
          strokeLinejoin="round"
          strokeLinecap="round"
          vectorEffect="non-scaling-stroke"
        />
      </svg>
      <div className="pointer-events-none absolute inset-x-0 top-0 flex justify-between text-[10px] text-faint">
        <span>{max}</span>
        <span>{min}</span>
      </div>
    </div>
  );
}
