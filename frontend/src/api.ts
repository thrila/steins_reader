import type { Skill, Observation, ObserveResponse, Source } from "./types";

const BASE = "/api";

async function handle<T>(res: Response): Promise<T> {
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new Error(`Request failed (${res.status}): ${text || res.statusText}`);
  }
  if (res.status === 204) return undefined as unknown as T;
  return res.json() as Promise<T>;
}

export const api = {
  listSkills: () => fetch(`${BASE}/skills`).then((r) => handle<Skill[]>(r)),

  createSkill: (name: string, initial_mean?: number) =>
    fetch(`${BASE}/skills`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name, initial_mean })
    }).then((r) => handle<Skill>(r)),

  updateSkill: (id: string, body: Partial<Pick<Skill, "name" | "mean" | "variance" | "process_noise" | "position">>) =>
    fetch(`${BASE}/skills/${id}`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body)
    }).then((r) => handle<Skill>(r)),

  deleteSkill: (id: string) => fetch(`${BASE}/skills/${id}`, { method: "DELETE" }).then((r) => handle<void>(r)),

  observe: (id: string, score: number, source: Source, note?: string, measurement_noise?: number) =>
    fetch(`${BASE}/skills/${id}/observe`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ score, source, note, measurement_noise })
    }).then((r) => handle<ObserveResponse>(r)),

  history: (id: string) => fetch(`${BASE}/skills/${id}/history`).then((r) => handle<Observation[]>(r))
};
