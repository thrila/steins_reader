export interface Skill {
  id: string;
  name: string;
  mean: number;
  variance: number;
  process_noise: number;
  position: number;
  created_at: number;
  last_updated: number;
}

export interface Observation {
  id: string;
  skill_id: string;
  score: number;
  measurement_noise: number;
  source: string;
  note: string | null;
  resulting_mean: number;
  resulting_variance: number;
  created_at: number;
}

export type Source = "standardized_test" | "peer_review" | "self_report" | "casual";

export const SOURCE_LABELS: Record<Source, string> = {
  standardized_test: "Standardized Test",
  peer_review: "Peer Review",
  self_report: "Self Report",
  casual: "Casual"
};

export interface ObserveResponse {
  skill: Skill;
  observation: Observation;
  kalman_gain: number;
}
