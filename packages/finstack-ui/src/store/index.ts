export interface EngineStateSnapshot {
  schemaVersion: string;
  marketContext?: unknown;
}

export interface UIStateSnapshot {
  activeView?: unknown;
  panelState?: Record<string, unknown>;
}

export interface RootStateSnapshot {
  engine: EngineStateSnapshot;
  ui: UIStateSnapshot;
}
