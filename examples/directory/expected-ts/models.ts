import type { Color, Status } from "./types";

export interface Point {
  x: number;
  y: number;
}

export interface Shape {
  x: number;
  y: number;
  color: Color;
  status?: Status | null;
  label: string;
}

export interface ShapePreview {
  x: number;
  y: number;
  color: Color;
  status?: Status | null;
}

