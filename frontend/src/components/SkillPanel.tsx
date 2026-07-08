import { useEffect, useState } from "react";
import { X, Trash2, Check, Clock } from "react-feather";
import { api } from "../api";
import type { Observation, Skill, Source } from "../types";
import { SOURCE_LABELS } from "../types";

interface Props {
  skill: Skill;
  onClose: () => void;
  onUpdated: (skill: Skill) => void;
  onDeleted: (id: string) => void;
}

const SOURCES: Source[] = ["standardized_test", "peer_review", "self_report", "casual"];

export default function SkillPanel({ skill, onClose, onUpdated, onDeleted }: Props) {
  const [name, setName] = useState(skill.name);
  const [score, setScore] = useState(Math.round(skill.mean));
  const [source, setSource] = useState<Source>("self_report");
  const [note, setNote] = useState("");
  const [history, setHistory] = useState<Observation[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);

  useEffect(() => {
    setName(skill.name);
    setScore(Math.round(skill.mean));
    setConfirmDelete(false);
    api
      .history(skill.id)
      .then(setHistory)
      .catch(() => setHistory([]));
  }, [skill.id]);

  async function handleRename() {
    if (name.trim() === "" || name === skill.name) return;
    try {
      const updated = await api.updateSkill(skill.id, { name: name.trim() });
      onUpdated(updated);
    } catch (e) {
      setError((e as Error).message);
    }
  }

  async function handleObserve() {
    setBusy(true);
    setError(null);
    try {
      const res = await api.observe(skill.id, score, source, note || undefined);
      onUpdated(res.skill);
      setHistory((h) => [...h, res.observation]);
      setNote("");
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  async function handleDelete() {
    if (!confirmDelete) {
      setConfirmDelete(true);
      return;
    }
    try {
      await api.deleteSkill(skill.id);
      onDeleted(skill.id);
    } catch (e) {
      setError((e as Error).message);
    }
  }

  const stddev = Math.sqrt(skill.variance);
  const confidencePct = Math.max(0, Math.min(100, 100 - stddev));

  return (
    <div className="panel">
      <div className="panel-header">
        <input
          className="panel-title-input"
          value={name}
          onChange={(e) => setName(e.target.value)}
          onBlur={handleRename}
          onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
        />
        <button className="icon-btn" onClick={onClose} aria-label="Close">
          <X size={18} />
        </button>
      </div>

      <div className="panel-stats">
        <div className="stat">
          <span className="stat-label">Mean</span>
          <span className="stat-value">{skill.mean.toFixed(1)}</span>
        </div>
        <div className="stat">
          <span className="stat-label">Confidence</span>
          <span className="stat-value">{confidencePct.toFixed(0)}%</span>
        </div>
        <div className="stat">
          <span className="stat-label">±1σ</span>
          <span className="stat-value">{stddev.toFixed(1)}</span>
        </div>
      </div>

      <div className="panel-section">
        <label className="field-label">New observation</label>
        <div className="score-row">
          <input
            type="range"
            min={0}
            max={100}
            value={score}
            onChange={(e) => setScore(Number(e.target.value))}
            className="score-slider"
          />
          <input
            type="number"
            min={0}
            max={100}
            value={score}
            onChange={(e) => setScore(Number(e.target.value))}
            className="score-number"
          />
        </div>

        <label className="field-label">Source</label>
        <div className="source-row">
          {SOURCES.map((s) => (
            <button
              key={s}
              className={`source-chip ${source === s ? "active" : ""}`}
              onClick={() => setSource(s)}
              type="button"
            >
              {SOURCE_LABELS[s]}
            </button>
          ))}
        </div>

        <label className="field-label">Note (optional)</label>
        <input
          className="text-input"
          placeholder="e.g. Codewars kata streak, mock interview..."
          value={note}
          onChange={(e) => setNote(e.target.value)}
        />

        <button className="primary-btn" onClick={handleObserve} disabled={busy}>
          <Check size={16} /> {busy ? "Updating..." : "Feed into Kalman filter"}
        </button>
        {error && <p className="error-text">{error}</p>}
      </div>

      {history.length > 0 && (
        <div className="panel-section">
          <label className="field-label">
            <Clock size={13} /> History
          </label>
          <ul className="history-list">
            {history
              .slice()
              .reverse()
              .slice(0, 8)
              .map((o) => (
                <li key={o.id} className="history-item">
                  <span className="history-source">{SOURCE_LABELS[o.source as Source] ?? o.source}</span>
                  <span className="history-score">{o.score.toFixed(0)}</span>
                  <span className="history-arrow">→</span>
                  <span className="history-result">{o.resulting_mean.toFixed(1)}</span>
                </li>
              ))}
          </ul>
        </div>
      )}

      <div className="panel-footer">
        <button className={`danger-btn ${confirmDelete ? "confirm" : ""}`} onClick={handleDelete}>
          <Trash2 size={15} /> {confirmDelete ? "Confirm delete vertex?" : "Delete vertex"}
        </button>
      </div>
    </div>
  );
}
