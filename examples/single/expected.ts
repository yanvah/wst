export enum Priority {
  Low = "Low",
  Medium = "Medium",
  High = "High",
  /** @deprecated */
  Critical = "Critical",
  /** @banned */
  Blocked = "Blocked",
}

export type Payload =
  | { Text: string }
  | { Count: number }
  | { Data: string[] }
  // @deprecated
  | { Legacy: string }
;

export interface Record {
  id: number;
  ref_code?: number | null;
  quantity?: number | null;
  score: number | null;
  active: boolean;
  label: string;
  note?: string | null;
  lookup?: Record<string, string> | null;
  flags?: boolean[] | null;
  priority: Priority;
  payload?: Payload | null;
}

export interface RecordSummary {
  id: number;
  ref_code?: number | null;
  quantity?: number | null;
  score: number | null;
  active: boolean;
  label: string;
  priority: Priority;
}

export interface ExtendedRecord {
  id: number;
  ref_code?: number | null;
  quantity?: number | null;
  score: number | null;
  active: boolean;
  label: string;
  note?: string | null;
  lookup?: Record<string, string> | null;
  flags?: boolean[] | null;
  priority: Priority;
  payload?: Payload | null;
  created_by: string;
}

export enum ApiError {
  NotFound = "NotFound",
  Forbidden = "Forbidden",
}

export interface InternalAudit {
  checksum: string;
  processed: boolean;
}

export const DEFAULT_ERROR: ApiError = ApiError.NotFound;

export const DEFAULT_RECORD: Record = {
  id: 1,
  score: 0,
  active: true,
  label: "default",
  priority: Priority.Low,
};

