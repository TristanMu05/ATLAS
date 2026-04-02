import React, { useState } from "react";
import { useTelemetry } from "../contexts/TelemetryProvider";

export const FaultPanel: React.FC = () => {
  const { stats, currentPacket } = useTelemetry();

  const [showHistory, setShowHistory] = useState(false);

  if (!stats || !currentPacket) return null;

  return (
    <>
      <div className="bg-dark-800 border-l-2 border-brand-red rounded p-4 h-full flex flex-col relative">
        <h3 className="text-gray-400 text-sm font-semibold tracking-wider uppercase mb-4 flex items-center">
          <span className="w-2 h-2 rounded-full bg-brand-red mr-2 animate-pulse"></span>
          Fault Status
        </h3>
        
        <div className="flex-1 space-y-4">
          <div>
            <div className="flex justify-between items-end mb-1">
              <span className="text-xs text-gray-500 uppercase tracking-widest block">Active Faults</span>
              {stats.activeFaults.length > 0 && stats.activeFaults[0] !== 'None' && (
                <button 
                  onClick={() => setShowHistory(true)}
                  className="text-[10px] text-brand-blue hover:text-white uppercase tracking-widest"
                >
                  View History
                </button>
              )}
            </div>
            <div className="bg-dark-900 border border-dark-700 rounded p-2 min-h-[60px] flex flex-col justify-center">
              {stats.activeFaults.slice(0, 1).map((f, i) => (
                <div key={i} className={`text-sm ${f === 'None' ? 'text-brand-green' : 'text-brand-red'}`}>{f}</div>
              ))}
            </div>
          </div>

        <div>
          <span className="text-xs text-gray-500 uppercase tracking-widest block mb-1">Telemetry Integrity</span>
          <div className="grid grid-cols-2 gap-2 mt-2">
            <div className="bg-dark-900 border border-dark-700 rounded p-2 flex flex-col items-center justify-center">
              <span className="text-gray-400 text-xs">CRC Errors</span>
              <span className={`text-xl font-bold ${stats.crcErrors > 0 ? 'text-brand-yellow' : 'text-brand-green'}`}>
                {stats.crcErrors}
              </span>
              <span className="text-gray-400 text-xs">Sync Errors</span>
              <span className={`text-xl font-bold ${stats.syncErrors > 0 ? 'text-brand-yellow' : 'text-brand-green'}`}>
                {stats.syncErrors}
              </span>
              <span className="text-gray-400 text-xs">Length Errors</span>
              <span className={`text-xl font-bold ${stats.lengthErrors > 0 ? 'text-brand-yellow' : 'text-brand-green'}`}>
                {stats.lengthErrors}
              </span>
              <span className="text-gray-400 text-xs">Sequence Errors</span>
              <span className={`text-xl font-bold ${stats.skippedErrors+ stats.outOfOrderErrors + stats.duplicateErrors > 0 ? 'text-brand-yellow' : 'text-brand-green'}`}>
                {stats.skippedErrors + stats.outOfOrderErrors + stats.duplicateErrors}
              </span>
            </div>
            <div className="bg-dark-900 border border-dark-700 rounded p-2 flex flex-col items-center justify-center">
              <span className="text-gray-400 text-xs">Packet Loss</span>
              <span className={`text-xl font-bold ${(stats.packetLossPercent > 1) ? 'text-brand-red' : 'text-gray-200'}`}>
                {stats.packetLossPercent.toFixed(1)}%
              </span>
              <span className="text-gray-400 text-xs">Successful Packets</span>
              <span className={`text-xl font-bold ${(stats.ok > 0) ? 'text-brand-green' : 'text-brand-red'}`}>
                {stats.ok}
              </span>
              <span className="text-gray-400 text-xs">Failed Packets</span>
              <span className={`text-xl font-bold ${(stats.failed > 0) ? 'text-brand-red' : 'text-brand-green'}`}>
                {stats.failed}
              </span>
            </div>
          </div>
        </div>
        </div>
      </div>

      {showHistory && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm p-4">
          <div className="bg-dark-800 border border-dark-700 rounded-lg shadow-2xl w-full max-w-lg overflow-hidden flex flex-col max-h-[80vh]">
            <div className="p-4 border-b border-dark-700 flex justify-between items-center bg-dark-900">
              <h3 className="text-white font-bold tracking-widest uppercase flex items-center">
                <span className="w-2 h-2 rounded-full bg-brand-red mr-2 animate-pulse"></span>
                Fault History
              </h3>
              <button 
                onClick={() => setShowHistory(false)}
                className="text-gray-400 hover:text-white text-xl"
              >
                &times;
              </button>
            </div>
            <div className="p-4 overflow-y-auto flex-1 space-y-2">
              {stats.activeFaults.map((f, i) => (
                <div key={i} className={`text-sm p-2 rounded border ${f === 'None' ? 'bg-dark-900 border-dark-700 text-brand-green' : 'bg-red-900/20 border-brand-red text-brand-red'}`}>
                  {f}
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
    </>
  );
};
