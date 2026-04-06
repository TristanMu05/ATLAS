import React from "react";
import { useTelemetry } from "../contexts/TelemetryProvider";
import { Database } from "lucide-react";

export const ReplaySystem: React.FC = () => {
  const { isSimMode, toggleSimMode, visibleData, panHistory } = useTelemetry();

  // Show the logs for the current visible window, reversed so newest is at the top
  const windowLogs = [...visibleData].reverse();

  const handleWheel = (e: React.WheelEvent<HTMLDivElement>) => {
    const delta = e.deltaY !== 0 ? e.deltaY : e.deltaX;
    panHistory(delta);
  };

  return (
    <div className="bg-dark-800 border border-dark-700 rounded h-full flex flex-col min-h-0">
      {/* Header */}
      <div className="bg-dark-900 border-b border-dark-700 p-3 flex justify-between items-center">
        <div className="flex items-center space-x-2">
          <Database size={16} className="text-gray-400" />
          <h2 className="text-sm font-semibold tracking-wider text-gray-200 uppercase">Telemetry Log</h2>
        </div>
        
        <button 
          onClick={toggleSimMode}
          className={`text-xs px-3 py-1 rounded font-bold uppercase transition-colors ${isSimMode ? 'bg-brand-yellow text-dark-900' : 'bg-brand-green text-dark-900'}`}
        >
          {isSimMode ? 'SIMULATOR MODE' : 'LIVE WEBSOCKET'}
        </button>
      </div>

      {/* Log Viewer */}
      <div 
        className="flex-1 p-3 min-h-0 overflow-y-auto custom-scrollbar"
        onWheel={handleWheel}
      >
        <table className="w-full text-left text-sm whitespace-nowrap table-fixed">
          <thead className="text-gray-500 border-b border-dark-700 sticky top-0 bg-dark-800">
            <tr>
              <th className="font-normal py-1.5 w-[25%]">Timestamp</th>
              <th className="font-normal py-1.5 w-[18%]">Temp (°C)</th>
              <th className="font-normal py-1.5 w-[18%]">Volt (V)</th>
              <th className="font-normal py-1.5 w-[18%]">Light (ADC)</th>
              <th className="font-normal py-1.5 w-[21%]">Status</th>
            </tr>
          </thead>
          <tbody className="text-gray-300 font-mono">
            {windowLogs.map((log, i) => (
              <tr key={i} className={`border-b border-dark-700 ${log.has_crc_error ? 'text-brand-red bg-red-900 bg-opacity-20' : ''}`}>
                <td className="py-1.5">{new Date(log.receive_timestamp).toLocaleTimeString()}</td>
                <td className="py-1.5">{log.temperature.toFixed(2)}</td>
                <td className="py-1.5">{log.voltage.toFixed(3)}</td>
                <td className="py-1.5">{log.light}</td>
                <td className={`py-1.5 font-bold ${log.has_crc_error ? 'text-brand-red' : (log.mode === 'NORMAL' ? 'text-brand-green' : 'text-brand-yellow')}`}>
                  {log.has_crc_error ? 'CRC ERROR' : log.mode}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
};
