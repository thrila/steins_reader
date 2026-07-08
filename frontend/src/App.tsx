import { useEffect, useState } from "react";
import { Plus } from "react-feather";
import RadarChart from "./components/RadarChart";
import SkillPanel from "./components/SkillPanel";
import { api } from "./api";
import type { Skill } from "./types";

export default function App() {
  const [skills, setSkills] = useState<Skill[]>([]);
  const [selected, setSelected] = useState<Skill | null>(null);
  const [newSkillName, setNewSkillName] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .listSkills()
      .then(setSkills)
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, []);

  function handleUpdated(skill: Skill) {
    setSkills((prev) => prev.map((s) => (s.id === skill.id ? skill : s)));
    setSelected(skill);
  }

  function handleDeleted(id: string) {
    setSkills((prev) => prev.filter((s) => s.id !== id));
    setSelected(null);
  }

  async function handleAdd() {
    const name = newSkillName.trim();
    if (!name) return;
    try {
      const skill = await api.createSkill(name);
      setSkills((prev) => [...prev, skill]);
      setNewSkillName("");
      setSelected(skill);
    } catch (e) {
      setError((e as Error).message);
    }
  }

  return (
    <div className="app-shell">
      <header className="dossier-header">
        <h1>Steins Reader</h1>
        <p className="dossier-subtitle">Faithful tracking. Visible results.</p>
      </header>

      <main className="main-grid">
        <section className="chart-column">
          {loading ? (
            <p className="muted">Loading dossier…</p>
          ) : (
            <RadarChart skills={skills} onSelect={setSelected} selectedId={selected?.id ?? null} />
          )}

          <div className="add-skill-row">
            <input
              className="text-input"
              placeholder="New vertex name, e.g. Systems Programming"
              value={newSkillName}
              onChange={(e) => setNewSkillName(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleAdd()}
            />
            <button className="primary-btn" onClick={handleAdd}>
              <Plus size={16} /> Add vertex
            </button>
          </div>
          {error && <p className="error-text">{error}</p>}
        </section>

        <section className="side-column">
          {selected ? (
            <SkillPanel
              key={selected.id}
              skill={selected}
              onClose={() => setSelected(null)}
              onUpdated={handleUpdated}
              onDeleted={handleDeleted}
            />
          ) : (
            <div className="panel panel-placeholder">
              <p>Select a vertex on the chart to update, rename, or delete it.</p>
            </div>
          )}
        </section>
      </main>
    </div>
  );
}
