import React, { useState } from "react";
import { useTelemetry } from "../contexts/TelemetryProvider";
import { OperatingMode } from "../lib/types";
import { Terminal, Send, ShieldAlert, Cpu } from "lucide-react";

export const CommandConsole: React.FC = () => {
  const { setModeCmd, sendCommand, lastResponse } = useTelemetry();
  const [cmdInput, setCmdInput] = useState("");
  const [paramInput, setParamInput] = useState("");

  const handleModeChange = (mode: OperatingMode) => {
    if (confirm(`Are you sure you want to transition to ${mode} mode?`)) {
      setModeCmd(mode);
    }
  };

  const handleSend = () => {
    if (cmdInput.trim()) {
      sendCommand(cmdInput.trim(), paramInput.trim());
      setCmdInput("");
      setParamInput("");
    }
  };

  return (
    <div className="flex flex-col bg-dark-800 border border-dark-700 rounded h-full">
      <div className="bg-dark-900 border-b border-dark-700 p-3 flex items-center space-x-2">
        <Terminal size={16} className="text-gray-400" />
        <h2 className="text-sm font-semibold tracking-wider text-gray-200 uppercase">Command Console</h2>
      </div>

      <div className="p-4 flex flex-col space-y-6 flex-1">
        
        {/* Mode Switches */}
        <div>
          <h3 className="text-xs text-gray-400 mb-3 uppercase tracking-wider">Flight Mode Transition</h3>
          <div className="grid grid-cols-2 gap-2">
            <button onClick={() => handleModeChange("SAFE")} className="bg-dark-700 hover:bg-gray-600 text-gray-300 py-2 rounded text-sm font-semibold transition-colors flex items-center justify-center space-x-2 border border-gray-600">
              <ShieldAlert size={14} /><span>SAFE</span>
            </button>
            <button onClick={() => handleModeChange("IDLE")} className="bg-dark-700 hover:bg-dark-600 text-gray-300 py-2 rounded text-sm font-semibold transition-colors border border-dark-600">
              IDLE
            </button>
            <button onClick={() => handleModeChange("NOMINAL")} className="bg-brand-green bg-opacity-10 hover:bg-opacity-20 text-brand-green py-2 rounded text-sm font-semibold transition-colors border border-brand-green border-opacity-30 flex items-center justify-center space-x-2">
              <Cpu size={14} /><span>NOMINAL</span>
            </button>
            <button onClick={() => handleModeChange("DIAGNOSTIC")} className="bg-blue-500 bg-opacity-10 hover:bg-opacity-20 text-blue-400 py-2 rounded text-sm font-semibold transition-colors border border-blue-500 border-opacity-30">
              DIAGNOSTIC
            </button>
          </div>
        </div>

        <div className="border-t border-dark-700 pt-4">
          <h3 className="text-xs text-gray-400 mb-3 uppercase tracking-wider">Direct Terminal</h3>
          <div className="flex flex-col space-y-2">
            <input 
              type="text" 
              placeholder="COMMAND (e.g. SET_RATE)"
              className="bg-dark-900 border border-dark-700 rounded px-3 py-2 text-sm text-white focus:outline-none focus:border-brand-green uppercase"
              value={cmdInput}
              onChange={(e) => setCmdInput(e.target.value)}
            />
            <input 
              type="text" 
              placeholder="PARAM (e.g. 100)"
              className="bg-dark-900 border border-dark-700 rounded px-3 py-2 text-sm text-white focus:outline-none focus:border-brand-green uppercase"
              value={paramInput}
              onChange={(e) => setParamInput(e.target.value)}
            />
            <button 
              onClick={handleSend}
              className="mt-2 bg-brand-green hover:bg-green-400 text-dark-900 font-bold py-2 rounded flex items-center justify-center space-x-2 transition-colors"
            >
              <span>SEND</span>
              <Send size={14} />
            </button>
          </div>
        </div>

        <div className="mt-auto pt-4 border-t border-dark-700">
          <div className="text-xs text-gray-500 mb-1">Last Response:</div>
          <div className={`text-sm font-mono p-2 rounded bg-dark-900 ${lastResponse !== 'NONE' ? 'text-brand-green border border-brand-green border-opacity-30' : 'text-gray-500 border border-dark-700'}`}>
            {lastResponse}
          </div>
        </div>

      </div>
    </div>
  );
};
