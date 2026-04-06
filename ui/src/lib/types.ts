export type OperatingMode = "SAFE" | "IDLE" | "NORMAL" | "DIAGNOSTIC";

export interface TelemetryPacket {
  timestamp: number;
  sequence: number;
  mode: OperatingMode;
  temperature: number;
  voltage: number;
  light: number;
  status_flags: number;
  fault_flags: number;
}

export interface TelemetryLogEntry extends TelemetryPacket {
  receive_timestamp: number;
  has_crc_error: boolean;
  is_dropped: boolean;
}

export interface SystemStats {
  packetRateHz: number;
  uptimeSeconds: number;
  linkStatus: "CONNECTED" | "CONNECTING" | "RECONNECTING" | "DISCONNECTED";
  crcErrors: number;
  syncErrors: number;
  lengthErrors: number;
  skippedErrors: number;
  duplicateErrors: number;
  outOfOrderErrors: number;
  packetLossPercent: number;
  activeFaults: string[];
  ok: number;
  failed: number;
}

export type BackendMessage = 
  | { type: "FrameReceived"; seq: number; timestamp: number, payload: number[] }
  | { type: "Stats"; ok: number; dropped: number }
  | { type: "Error"; kind: string; detail: string }
  | { type: "Ack"; code: number }
  | { type: "CommandResponse"; code: number }
  | { type: "Event"; code: number }
  | { type: "LinkState"; state: SystemStats["linkStatus"]; detail: string };
