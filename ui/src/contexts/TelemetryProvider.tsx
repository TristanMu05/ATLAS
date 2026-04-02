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
}

const TelemetryContext = createContext<TelemetryContextType | undefined>(undefined);

export const TelemetryProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  const [isSimMode, setIsSimMode] = useState(false);
  const [currentPacket, setCurrentPacket] = useState<TelemetryLogEntry | null>(null);
  const [history, setHistory] = useState<TelemetryLogEntry[]>([]);
  const [stats, setStats] = useState<SystemStats | null>({
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
  });
  const [lastResponse, setLastResponse] = useState<string>("NONE");

  const packetsThisSecondRef = useRef(0);
  const connectionStartTimeRef = useRef<number | null>(null);

  useEffect(() => {
    const timer = setInterval(() => {
      setStats(prev => {
        if (!prev || prev.linkStatus === "DISCONNECTED") return prev;
        
        const uptime = connectionStartTimeRef.current 
          ? Math.floor((Date.now() - connectionStartTimeRef.current) / 1000) 
          : prev.uptimeSeconds;
          
        const rate = packetsThisSecondRef.current;
        packetsThisSecondRef.current = 0;

        return {
          ...prev,
          uptimeSeconds: uptime,
          packetRateHz: rate
        };
      });
    }, 1000);
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
    const ws = new WebSocket("ws://localhost:3000/ws");
    
    ws.onopen = () => {
      connectionStartTimeRef.current = Date.now();
      packetsThisSecondRef.current = 0;
      setStats(prev => prev ? { ...prev, linkStatus: "CONNECTED" } : {
        packetRateHz: 0,
        uptimeSeconds: 0,
        linkStatus: "CONNECTED",
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
      });
    };

    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data) as BackendMessage;
        console.log("WS RECV (connectWebSocket):", msg);

        if (msg.type === "FrameReceived") {
          packetsThisSecondRef.current += 1;
          const p = msg.payload || [];
          const packet: TelemetryLogEntry = {
            timestamp: msg.timestamp,
            sequence: msg.seq,
            mode: "NOMINAL",
            temperature: p.length > 0 ? p[0] : 0,
            voltage: p.length > 1 ? p[1] : 0,
            fault_flags: p.length > 2 ? p[2] : 0,
            receive_timestamp: Date.now(),
            has_crc_error: false,
            is_dropped: false
          };

          setCurrentPacket(packet);
          setHistory(prev => {
            const next = [...prev, packet];
            // Store up to 10,000 packets (~16 minutes at 10Hz) before dropping old ones
            if(next.length > 10000) next.shift();
            return next;
          });
        } else if (msg.type === "Stats") {
          const total = msg.ok + msg.dropped;
          setStats(prev => prev ? {
            ...prev,
            packetLossPercent: total > 0 ? (msg.dropped / total) * 100 : 0,
            ok: msg.ok,
            failed: msg.dropped
          } : {
            packetRateHz: 0,
            uptimeSeconds: 0,
            linkStatus: "CONNECTED",
            crcErrors: 0,
            syncErrors: 0,
            lengthErrors: 0,
            skippedErrors: 0,
            duplicateErrors: 0,
            outOfOrderErrors: 0,
            packetLossPercent: total > 0 ? (msg.dropped / total) * 100 : 0,
            activeFaults: ["None"],
            ok: msg.ok,
            failed: msg.dropped
          });
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
        }
      } catch(e) {
        console.error("Failed to parse websocket message", e);
      }
    };

    ws.onerror = () => {
      connectionStartTimeRef.current = null;
      setStats(prev => prev ? { ...prev, linkStatus: "DISCONNECTED", packetRateHz: 0 } : null);
    };
    ws.onclose = () => {
      connectionStartTimeRef.current = null;
      setStats(prev => prev ? { ...prev, linkStatus: "DISCONNECTED", packetRateHz: 0 } : null);
    };

    return ws;
  };

  useEffect(() => {
    const ws = connectWebSocket();
    return () => ws.close();
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
      await fetch(`http://localhost:3000/start/${endpoint}`, { method: "POST" });
      setLastResponse(`STARTED ${endpoint.toUpperCase()}`);
    } catch (e) {
      setLastResponse("FETCH FAILED");
    }
  };

  const sendCommand = async (cmd: string, param: string) => {
    console.log(`Command sent to UI Context: ${cmd} ${param}`);
    try {
      await fetch("http://localhost:3000/command", { 
        method: "POST", 
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ cmd, param }) 
      });
      setLastResponse("SENT VERIFIED");
    } catch(e) {
      setLastResponse("SEND FAILED");
    }
    setTimeout(() => setLastResponse("NONE"), 3000);
  };

  const setModeCmd = (mode: OperatingMode) => {
    sendCommand("SET_MODE", mode);
  };

  return (
    <TelemetryContext.Provider value={{
      isSimMode, toggleSimMode, currentPacket, history, stats, sendCommand, setModeCmd, lastResponse,
      isLockedToFront, setIsLockedToFront, visibleData, panHistory
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
