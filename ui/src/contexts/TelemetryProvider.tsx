import React, { createContext, useContext, useState, useEffect, useRef, ReactNode } from "react";
import { TelemetryLogEntry, SystemStats, OperatingMode, BackendMessage } from "../lib/types";

interface TelemetryContextType {
  isSimMode: boolean;
  toggleSimMode: () => void;
  currentPacket: TelemetryLogEntry | null;
  history: TelemetryLogEntry[];
  stats: SystemStats | null;
  sendCommand: (cmd: string, param: string) => void;
  setModeCmd: (mode: OperatingMode) => void;
  lastResponse: string;
  isLockedToFront: boolean;
  setIsLockedToFront: (val: boolean) => void;
  visibleData: TelemetryLogEntry[];
  panHistory: (delta: number) => void;
  dismissFault: (index: number) => void;
  resetSystem: () => void;
}

const TelemetryContext = createContext<TelemetryContextType | undefined>(undefined);

const DEFAULT_STATS: SystemStats = {
  packetRateHz: 0,
  uptimeSeconds: 0,
  linkStatus: "DISCONNECTED",
  crcErrors: 0,
  syncErrors: 0,
  lengthErrors: 0,
  skippedErrors: 0,
  duplicateErrors: 0,
  outOfOrderErrors: 0,
  packetLossPercent: 0,
  activeFaults: ["None"],
  ok: 0,
  failed: 0
};

const FAULT_FLAG_LABELS: Array<[number, string]> = [
  [1 << 0, "TEMP_SENSOR_FAULT"],
  [1 << 1, "LIGHT_SENSOR_FAULT"],
  [1 << 2, "VOLTAGE_MONITOR_FAULT"],
  [1 << 3, "OVER_TEMPERATURE"],
  [1 << 4, "LOW_VOLTAGE"],
  [1 << 5, "SENSOR_INIT_FAULT"],
  [1 << 6, "UART_RX_OVERRUN"],
  [1 << 7, "RX_BUFFER_OVERFLOW"],
  [1 << 8, "INTERNAL_TIMEOUT"],
  [1 << 9, "WATCHDOG_RESET_DETECTED"],
  [1 << 10, "PLATFORM_INIT_FAULT"],
  [1 << 11, "UNDER_TEMPERATURE"],
  [1 << 12, "HIGH_VOLTAGE"]
];

const EVENT_LABELS: Record<number, string> = {
  0x01: "MALFORMED_PACKET",
  0x02: "CRC_FAILURE",
  0x03: "UNSUPPORTED_COMMAND",
  0x04: "COMMAND_INVALID_IN_MODE",
  0x05: "TEMP_SENSOR_READ_FAIL",
  0x06: "LIGHT_SENSOR_READ_FAIL",
  0x07: "VOLTAGE_MONITOR_READ_FAIL",
  0x08: "SAFE_MODE_ENTERED",
  0x09: "SAFE_MODE_EXITED",
  0x0A: "INTERNAL_TIMEOUT",
  0x0B: "UART_RX_OVERRUN",
  0x0C: "WATCHDOG_RESET_DETECTED",
  0x0D: "UNDER_TEMPERATURE",
  0x0E: "HIGH_VOLTAGE",
  0x0F: "LOW_VOLTAGE",
  0x10: "OVER_TEMPERATURE"
};

const COMMAND_RESPONSE_LABELS: Record<number, string> = {
  0x00: "COMPLETED",
  0x01: "REJECTED",
  0x02: "INVALID_PARAMETER",
  0x03: "INVALID_MODE",
  0x04: "FAULT_ACTIVE",
  0x05: "EXECUTION_ERROR",
  0x06: "NOT_SUPPORTED",
  0x07: "DEFERRED"
};

function trimTrailingSlashes(value: string): string {
  return value.replace(/\/+$/, "");
}

function resolveBackendHttpBase(): string {
  const configuredBase = trimTrailingSlashes(import.meta.env.VITE_ATLAS_BACKEND_URL ?? "");
  if (configuredBase.length > 0) {
    return configuredBase;
  }

  const protocol = window.location.protocol === "https:" ? "https:" : "http:";
  const hostname = window.location.hostname || "localhost";
  return `${protocol}//${hostname}:3000`;
}

function resolveBackendWsUrl(httpBase: string): string {
  const wsBase = httpBase.replace(/^http/i, "ws");
  return `${wsBase}/ws`;
}

function formatFetchError(error: unknown): string {
  if (error instanceof Error && error.message.trim().length > 0) {
    return error.message;
  }

  return "network request failed";
}

function isLiveLinkStatus(value: string): value is SystemStats["linkStatus"] {
  return value === "CONNECTED" || value === "CONNECTING" || value === "RECONNECTING" || value === "DISCONNECTED";
}

const BACKEND_HTTP_BASE = resolveBackendHttpBase();
const BACKEND_WS_URL = resolveBackendWsUrl(BACKEND_HTTP_BASE);

function decodeMode(mode: number): OperatingMode {
  switch (mode) {
    case 0x00: return "IDLE";
    case 0x01: return "NORMAL";
    case 0x02: return "SAFE";
    case 0x03: return "DIAGNOSTIC";
    default: return "SAFE";
  }
}

function decodeSignedInt16(msb: number, lsb: number): number {
  const value = (msb << 8) | lsb;
  return (value & 0x8000) !== 0 ? value - 0x10000 : value;
}

function decodeFaultFlags(flags: number): string[] {
  const faults = FAULT_FLAG_LABELS
    .filter(([bit]) => (flags & bit) !== 0)
    .map(([, label]) => label);

  return faults.length > 0 ? faults : ["None"];
}

function parseTelemetryPayload(payload: number[], seq: number, timestamp: number): TelemetryLogEntry {
  if (payload.length >= 10) {
    const faultFlags = (payload[8] << 8) | payload[9];

    return {
      timestamp,
      sequence: seq,
      mode: decodeMode(payload[0]),
      temperature: decodeSignedInt16(payload[1], payload[2]) / 10,
      voltage: ((payload[3] << 8) | payload[4]) / 1000,
      light: (payload[5] << 8) | payload[6],
      status_flags: payload[7],
      fault_flags: faultFlags,
      receive_timestamp: Date.now(),
      has_crc_error: false,
      is_dropped: false
    };
  }

  return {
    timestamp,
    sequence: seq,
    mode: "NORMAL",
    temperature: payload.length > 0 ? payload[0] : 0,
    voltage: payload.length > 1 ? payload[1] : 0,
    light: payload.length > 2 ? payload[2] : 0,
    status_flags: 0,
    fault_flags: payload.length > 3 ? payload[3] : 0,
    receive_timestamp: Date.now(),
    has_crc_error: false,
    is_dropped: false
  };
}

export const TelemetryProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  const [isSimMode, setIsSimMode] = useState(false);
  const [currentPacket, setCurrentPacket] = useState<TelemetryLogEntry | null>(null);
  const [history, setHistory] = useState<TelemetryLogEntry[]>([]);
  const [stats, setStats] = useState<SystemStats | null>(DEFAULT_STATS);
  const [lastResponse, setLastResponse] = useState<string>("NONE");

  const packetArrivalTimesRef = useRef<number[]>([]);
  const connectionStartTimeRef = useRef<number | null>(null);
  const hasStartedInitialSessionRef = useRef(false);
  const pendingCommandLabelRef = useRef<string | null>(null);
  const statsOffsetRef = useRef({ ok: 0, dropped: 0 });

  useEffect(() => {
    const rateWindowMs = 2000;
    const timer = setInterval(() => {
      setStats(prev => {
        if (!prev || prev.linkStatus === "DISCONNECTED") return prev;
        
        const now = Date.now();
        const uptime = connectionStartTimeRef.current 
          ? Math.floor((now - connectionStartTimeRef.current) / 1000) 
          : prev.uptimeSeconds;
        const cutoff = now - rateWindowMs;
        packetArrivalTimesRef.current = packetArrivalTimesRef.current.filter(ts => ts >= cutoff);
        const rate = Math.round((packetArrivalTimesRef.current.length * 1000) / rateWindowMs);

        return {
          ...prev,
          uptimeSeconds: uptime,
          packetRateHz: rate
        };
      });
    }, 250);
    return () => clearInterval(timer);
  }, []);

  // --- Scroll State for Graph & Logs ---
  const [isLockedToFront, setIsLockedToFront] = useState(true);
  const [scrollStartIndex, setScrollStartIndex] = useState<number>(0);
  const [scrollEndIndex, setScrollEndIndex] = useState<number>(0);

  const WINDOW_SIZE = 100;

  useEffect(() => {
    if (isLockedToFront && history.length > 0) {
      setScrollEndIndex(history.length - 1);
      setScrollStartIndex(Math.max(0, history.length - WINDOW_SIZE));
    }
  }, [history, isLockedToFront]);

  const panHistory = (delta: number) => {
    const step = Math.sign(delta) * 5; 
    if (delta !== 0) {
      setIsLockedToFront(false);
      setScrollStartIndex(prev => {
        const next = Math.max(0, Math.min(history.length - WINDOW_SIZE, prev + step));
        setScrollEndIndex(next + WINDOW_SIZE - 1);
        
        if (next >= history.length - WINDOW_SIZE) {
          setIsLockedToFront(true);
        }
        return next;
      });
    }
  };

  const visibleData = React.useMemo(() => {
    if (isLockedToFront) {
      return history.slice(Math.max(0, history.length - WINDOW_SIZE));
    }
    return history.slice(scrollStartIndex, scrollEndIndex + 1);
  }, [history, isLockedToFront, scrollStartIndex, scrollEndIndex]);
  // ------------------------------------



  const connectWebSocket = () => {
    const ws = new WebSocket(BACKEND_WS_URL);
    
    ws.onopen = () => {
      packetArrivalTimesRef.current = [];
      setStats(prev => prev ?? DEFAULT_STATS);
    };

    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data) as BackendMessage;
        console.log("WS RECV (connectWebSocket):", msg);

        if (msg.type === "FrameReceived") {
          if (connectionStartTimeRef.current === null) {
            connectionStartTimeRef.current = Date.now();
          }
          packetArrivalTimesRef.current.push(Date.now());
          const packet = parseTelemetryPayload(msg.payload || [], msg.seq, msg.timestamp);

          setCurrentPacket(packet);
          setStats(prev => {
            const base = prev ?? { ...DEFAULT_STATS };
            const newFaults = decodeFaultFlags(packet.fault_flags);
            let merged = base.activeFaults;

            // Only merge if the packet actually reports faults
            if (newFaults.length > 0 && newFaults[0] !== 'None') {
              // Prepend new faults that aren't already listed
              const existing = new Set(merged);
              const toAdd = newFaults.filter(f => !existing.has(f));
              if (toAdd.length > 0) {
                merged = [...toAdd, ...merged.filter(f => f !== 'None')];
              }
            }

            return {
              ...base,
              linkStatus: "CONNECTED",
              activeFaults: merged
            };
          });
          setHistory(prev => {
            const next = [...prev, packet];
            // Store up to 10,000 packets (~16 minutes at 10Hz) before dropping old ones
            if(next.length > 10000) next.shift();
            return next;
          });
        } else if (msg.type === "Stats") {
          const adjOk = msg.ok - statsOffsetRef.current.ok;
          const adjDropped = msg.dropped - statsOffsetRef.current.dropped;
          const total = adjOk + adjDropped;
          setStats(prev => prev ? {
            ...prev,
            linkStatus: prev.linkStatus === "DISCONNECTED" ? "CONNECTED" : prev.linkStatus,
            packetLossPercent: total > 0 ? (adjDropped / total) * 100 : 0,
            ok: adjOk,
            failed: adjDropped
          } : {
            ...DEFAULT_STATS,
            linkStatus: "CONNECTED",
            packetLossPercent: total > 0 ? (adjDropped / total) * 100 : 0,
            ok: adjOk,
            failed: adjDropped
          });
        } else if (msg.type === "LinkState") {
          const nextStatus = isLiveLinkStatus(msg.state) ? msg.state : "DISCONNECTED";

          if (nextStatus === "CONNECTED") {
            connectionStartTimeRef.current ??= Date.now();
          } else {
            connectionStartTimeRef.current = null;
            packetArrivalTimesRef.current = [];
          }

          setStats(prev => prev ? {
            ...prev,
            linkStatus: nextStatus,
            packetRateHz: nextStatus === "CONNECTED" ? prev.packetRateHz : 0
          } : {
            ...DEFAULT_STATS,
            linkStatus: nextStatus
          });

          if (nextStatus === "CONNECTED") {
            setLastResponse("LINK CONNECTED");
          } else if (nextStatus === "RECONNECTING") {
            setLastResponse("LINK RECONNECTING");
          }
        } else if (msg.type === "Error") {
          setStats(prev => {
            if (!prev) return null;
            let { crcErrors, syncErrors, lengthErrors, skippedErrors, duplicateErrors, outOfOrderErrors } = prev;
            
            const detailLower = msg.detail.toLowerCase();
            if (detailLower.includes("crcmismatch")) crcErrors++;
            else if (detailLower.includes("invalidsync")) syncErrors++;
            else if (detailLower.includes("lengthmismatch") || detailLower.includes("packettooshort")) lengthErrors++;
            else if (detailLower.includes("skipped") || detailLower.includes("dropped")) skippedErrors++;
            else if (detailLower.includes("duplicate")) duplicateErrors++;
            else if (detailLower.includes("out of order") || detailLower.includes("out-of-order") || detailLower.includes("outoforder")) outOfOrderErrors++;

            return {
              ...prev,
              crcErrors,
              syncErrors,
              lengthErrors,
              skippedErrors,
              duplicateErrors,
              outOfOrderErrors,
              activeFaults: [msg.detail, ...prev.activeFaults.filter(f => f !== 'None')].slice(0, 100)
            };
          });
        } else if (msg.type === "Ack") {
          setLastResponse(msg.code === 0x06 ? "ACK" : "NAK");
        } else if (msg.type === "CommandResponse") {
          setLastResponse(COMMAND_RESPONSE_LABELS[msg.code] ?? `RESPONSE 0x${msg.code.toString(16).toUpperCase()}`);
        } else if (msg.type === "Event") {
          const eventLabel = EVENT_LABELS[msg.code] ?? `EVENT 0x${msg.code.toString(16).toUpperCase()}`;
          setLastResponse(eventLabel);
          setStats(prev => prev ? {
            ...prev,
            activeFaults: [eventLabel, ...prev.activeFaults.filter(f => f !== 'None')].slice(0, 100)
          } : prev);
        }
      } catch(e) {
        console.error("Failed to parse websocket message", e);
      }
    };

    ws.onerror = () => {
      connectionStartTimeRef.current = null;
      packetArrivalTimesRef.current = [];
      setStats(prev => prev ? { ...prev, linkStatus: "DISCONNECTED", packetRateHz: 0 } : null);
    };
    ws.onclose = () => {
      connectionStartTimeRef.current = null;
      packetArrivalTimesRef.current = [];
      setStats(prev => prev ? { ...prev, linkStatus: "DISCONNECTED", packetRateHz: 0 } : null);
    };

    return ws;
  };

  useEffect(() => {
    const ws = connectWebSocket();
    return () => ws.close();
  }, []);

  const startSession = async (endpoint: "live" | "sim") => {
    const response = await fetch(`${BACKEND_HTTP_BASE}/start/${endpoint}`, { method: "POST" });
    const detail = await response.text();
    setLastResponse(response.ok ? detail.toUpperCase() : `START FAILED: ${detail}`);
  };

  useEffect(() => {
    if (hasStartedInitialSessionRef.current) {
      return;
    }

    hasStartedInitialSessionRef.current = true;
    startSession("live").catch((error) => {
      setLastResponse(`START FAILED: ${formatFetchError(error)}`);
    });
  }, []);

  const toggleSimMode = async () => {
    const newMode = !isSimMode;
    setIsSimMode(newMode);
    
    // Explicitly reconnect if we are disconnected so it works without refreshing UI
    if (stats?.linkStatus === "DISCONNECTED") {
      connectWebSocket();
    }

    try {
      const endpoint = newMode ? "sim" : "live";
      await startSession(endpoint);
    } catch (error) {
      setLastResponse(`START FAILED: ${formatFetchError(error)}`);
    }
  };

  const sendCommand = async (cmd: string, param: string) => {
    console.log(`Command sent to UI Context: ${cmd} ${param}`);
    const pendingLabel = `SENDING ${cmd.trim().toUpperCase()}`;
    pendingCommandLabelRef.current = pendingLabel;
    setLastResponse(pendingLabel);

    try {
      const response = await fetch(`${BACKEND_HTTP_BASE}/command`, {
        method: "POST", 
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ cmd, param }) 
      });
      const detail = await response.text();

      if (!response.ok) {
        pendingCommandLabelRef.current = null;
        setLastResponse(`SEND FAILED: ${detail}`);
        return;
      }

      const upperDetail = detail.toUpperCase();
      if (upperDetail.includes("RECONNECTING")) {
        setLastResponse(upperDetail);
        return;
      }

      window.setTimeout(() => {
        setLastResponse((previous) => {
          if (pendingCommandLabelRef.current == null) {
            return previous;
          }

          if (previous === pendingLabel) {
            return upperDetail;
          }

          pendingCommandLabelRef.current = null;
          return previous;
        });
      }, 150);
    } catch(error) {
      pendingCommandLabelRef.current = null;
      setLastResponse(`SEND FAILED: ${formatFetchError(error)}`);
    }
  };

  const setModeCmd = (mode: OperatingMode) => {
    sendCommand("SET_MODE", mode);
  };

  const dismissFault = (index: number) => {
    setStats(prev => {
      if (!prev) return prev;
      const updated = prev.activeFaults.filter((_, i) => i !== index);
      return {
        ...prev,
        activeFaults: updated.length > 0 ? updated : ["None"]
      };
    });
  };

  const resetSystem = () => {
    // Snapshot current backend counts so post-reset Stats are relative to now
    if (stats) {
      statsOffsetRef.current = {
        ok: statsOffsetRef.current.ok + stats.ok,
        dropped: statsOffsetRef.current.dropped + stats.failed
      };
    }

    // Clear all UI state
    setCurrentPacket(null);
    setHistory([]);
    setStats({ ...DEFAULT_STATS });
    setLastResponse("SYSTEM RESET");
    packetArrivalTimesRef.current = [];
    connectionStartTimeRef.current = null;
    pendingCommandLabelRef.current = null;
    setIsLockedToFront(true);

    // Send SET_MODE IDLE to the STM
    sendCommand("SET_MODE", "IDLE");
  };

  return (
    <TelemetryContext.Provider value={{
      isSimMode, toggleSimMode, currentPacket, history, stats, sendCommand, setModeCmd, lastResponse,
      isLockedToFront, setIsLockedToFront, visibleData, panHistory, dismissFault, resetSystem
    }}>
      {children}
    </TelemetryContext.Provider>
  );
};

export const useTelemetry = () => {
  const context = useContext(TelemetryContext);
  if (!context) throw new Error("useTelemetry must be used within TelemetryProvider");
  return context;
};
