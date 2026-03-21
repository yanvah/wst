export enum Color {
  Red = "Red",
  Green = "Green",
  Blue = "Blue",
}

export enum Status {
  Active = "Active",
  /** @deprecated */
  Inactive = "Inactive",
}

export enum InternalCode {
  Ok = "Ok",
  Fail = "Fail",
}

export interface Point {
  x: number;
  y: number;
}

