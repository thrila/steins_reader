import { useMemo, useState } from "react";
import type { Skill } from "../types";

interface Props {
  skills: Skill[];
  size?: number;
  onSelect: (skill: Skill) => void;
  selectedId?: string | null;
}

const RINGS = 1;

function polarToCartesian(cx: number, cy: number, radius: number, angleDeg: number) {
  const angleRad = ((angleDeg - 90) * Math.PI) / 180;
  return { x: cx + radius * Math.cos(angleRad), y: cy + radius * Math.sin(angleRad) };
}

export default function RadarChart({ skills, size = 480, onSelect, selectedId }: Props) {
  const [hoverId, setHoverId] = useState<string | null>(null);
  const cx = size / 2;
  const cy = size / 2;
  const maxR = size * 0.32; // was 0.36 — leaves more margin for labels
  const n = skills.length;

  const angleStep = n > 0 ? 360 / n : 0;

  const meanPoints = useMemo(
    () =>
      skills.map((s, i) => {
        const r = (s.mean / 100) * maxR;
        return polarToCartesian(cx, cy, r, i * angleStep);
      }),
    [skills, maxR, cx, cy, angleStep]
  );

  // Confidence band: mean +/- 1 stddev, clamped to [0,100], shown as a soft halo
  const confidenceOuter = useMemo(
    () =>
      skills.map((s, i) => {
        const sd = Math.sqrt(s.variance);
        const upper = Math.min(100, s.mean + sd);
        const r = (upper / 100) * maxR;
        return polarToCartesian(cx, cy, r, i * angleStep);
      }),
    [skills, maxR, cx, cy, angleStep]
  );
  const confidenceInner = useMemo(
    () =>
      skills.map((s, i) => {
        const sd = Math.sqrt(s.variance);
        const lower = Math.max(0, s.mean - sd);
        const r = (lower / 100) * maxR;
        return polarToCartesian(cx, cy, r, i * angleStep);
      }),
    [skills, maxR, cx, cy, angleStep]
  );

  const pointsToPath = (pts: { x: number; y: number }[]) => pts.map((p) => `${p.x},${p.y}`).join(" ");

  if (n === 0) {
    return (
      <div className="radar-empty" style={{ width: size, height: size }}>
        <p>No vertices yet. Add a skill to begin the dossier.</p>
      </div>
    );
  }

  return (
    <svg viewBox={`0 0 ${size} ${size}`} width={size} height={size} className="radar-svg">
      {/* grid rings */}
      {Array.from({ length: RINGS }).map((_, ringIdx) => {
        const r = (maxR * (ringIdx + 1)) / RINGS;
        const ringPts = skills.map((_, i) => polarToCartesian(cx, cy, r, i * angleStep));
        return (
          <polygon
            key={`ring-${ringIdx}`}
            points={pointsToPath(ringPts)}
            className="radar-grid-ring"
            style={{ strokeDasharray: ringIdx === RINGS - 1 ? "0" : "3 4" }}
          />
        );
      })}

      {/* axes */}
      {skills.map((s, i) => {
        const edge = polarToCartesian(cx, cy, maxR, i * angleStep);
        return <line key={`axis-${s.id}`} x1={cx} y1={cy} x2={edge.x} y2={edge.y} className="radar-axis" />;
      })}

      {/* confidence band */}
      <polygon points={pointsToPath(confidenceOuter)} className="radar-confidence" />
      <polygon points={pointsToPath(confidenceInner)} className="radar-confidence-inner" />

      {/* mean polygon */}
      <polygon points={pointsToPath(meanPoints)} className="radar-mean-fill" />
      <polyline points={pointsToPath([...meanPoints, meanPoints[0]])} className="radar-mean-stroke" />

      {/* vertices */}
      {meanPoints.map((p, i) => (
        <circle
          key={`vertex-${skills[i].id}`}
          cx={p.x}
          cy={p.y}
          r={hoverId === skills[i].id || selectedId === skills[i].id ? 6 : 4}
          className="radar-vertex"
          onMouseEnter={() => setHoverId(skills[i].id)}
          onMouseLeave={() => setHoverId(null)}
          onClick={() => onSelect(skills[i])}
        />
      ))}

      {/* labels */}
      {skills.map((s, i) => {
        const labelR = maxR + 24;
        const p = polarToCartesian(cx, cy, labelR, i * angleStep);
        const anchor = Math.abs(p.x - cx) < 4 ? "middle" : p.x > cx ? "start" : "end";
        return (
          <g
            key={`label-${s.id}`}
            className={`radar-label-group ${selectedId === s.id ? "active" : ""}`}
            onClick={() => onSelect(s)}
            onMouseEnter={() => setHoverId(s.id)}
            onMouseLeave={() => setHoverId(null)}
          >
            <text x={p.x} y={p.y} textAnchor={anchor} className="radar-label">
              {s.name.toUpperCase()}
            </text>
            <text x={p.x} y={p.y + 14} textAnchor={anchor} className="radar-label-value">
              {Math.round(s.mean)}
            </text>
          </g>
        );
      })}
    </svg>
  );
}
