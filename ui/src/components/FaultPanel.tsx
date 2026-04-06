import React from "react";
import { useTelemetry } from "../contexts/TelemetryProvider";

export const FaultPanel: React.FC = () => {
  const { stats, currentPacket, dismissFault } = useTelemetry();

  if (!stats || !currentPacket) return null;

  const hasFaults = stats.activeFaults.length > 0 && stats.activeFaults[0] !== 'None';

  return (
    <div className="bg-dark-800 border-l-2 border-brand-red rounded p-4 h-full flex flex-col relative">
      <h3 className="text-gray-400 text-sm font-semibold tracking-wider uppercase mb-4 flex items-center">
        <span className={`w-2 h-2 rounded-full mr-2 ${hasFaults ? 'bg-brand-red animate-pulse' : 'bg-brand-green'}`}></span>
        Fault Status
      </h3>
      
      <div className="flex-1 space-y-4 overflow-hidden flex flex-col">
        {/* Active Faults List */}
        <div className="flex-1 flex flex-col min-h-0">
          <span className="text-xs text-gray-500 uppercase tracking-widest block mb-1">Active Faults</span>
          <div className="bg-dark-900 border border-dark-700 rounded p-2 min-h-[60px] flex-1 overflow-y-auto space-y-1">
            {!hasFaults ? (
              <div className="text-sm text-brand-green flex items-center justify-center h-full">
                <svg className="w-4 h-4 mr-1.5 opacity-60" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                No Active Faults
              </div>
            ) : (
              stats.activeFaults.map((f, i) => (
                <div
                  key={`${f}-${i}`}
                  className="group flex items-center justify-between text-sm px-2 py-1.5 rounded border border-brand-red/30 bg-red-950/30 text-brand-red transition-colors hover:bg-red-950/50"
                >
                  <div className="flex items-center gap-2 min-w-0">
                    <span className="w-1.5 h-1.5 rounded-full bg-brand-red flex-shrink-0 animate-pulse"></span>
                    <span className="truncate">{f}</span>
                  </div>
                  <button
                    onClick={() => dismissFault(i)}
                    className="flex-shrink-0 ml-2 w-5 h-5 flex items-center justify-center rounded text-gray-500 hover:text-white hover:bg-red-900/60 transition-colors opacity-0 group-hover:opacity-100"
                    title="Dismiss fault"
                    aria-label={`Dismiss fault: ${f}`}
                  >
                    <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2.5}>
                      <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                  </button>
                </div>
              ))
            )}
          </div>
        </div>

        {/* Telemetry Integrity */}
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
  );
};
