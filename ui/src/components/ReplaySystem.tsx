import React from "react";
import { useTelemetry } from "../contexts/TelemetryProvider";
import { Play, Pause, SkipForward, Rewind, Database } from "lucide-react";

export const ReplaySystem: React.FC = () => {
  const { isSimMode, toggleSimMode, visibleData, panHistory } = useTelemetry();

  // Show the logs for the current visible window, reversed so newest is at the top
  const windowLogs = [...visibleData].reverse();

  const handleWheel = (e: React.WheelEvent<HTMLDivElement>) => {
    // Only pan if they are scrolling the mouse wheel (to match graph behavior)
    const delta = e.deltaY !== 0 ? e.deltaY : e.deltaX;
    panHistory(delta);
  };

  return (
    <div className="bg-dark-800 border border-dark-700 rounded h-full flex flex-col min-h-0">
      {/* Header */}
      <div className="bg-dark-900 border-b border-dark-700 p-3 flex justify-between items-center">
        <div className="flex items-center space-x-2">
          <Database size={16} className="text-gray-400" />
          <h2 className="text-sm font-semibold tracking-wider text-gray-200 uppercase">Telemetry Log & Replay</h2>
        </div>
        
        <button 
          onClick={toggleSimMode}
          className={`text-xs px-3 py-1 rounded font-bold uppercase transition-colors ${isSimMode ? 'bg-brand-yellow text-dark-900' : 'bg-brand-green text-dark-900'}`}
        >
          {isSimMode ? 'SIMULATOR MODE' : 'LIVE WEBSOCKET'}
        </button>
      </div>

      <div className="p-4 flex flex-col lg:flex-row flex-1 min-h-0">
        {/* Log Viewer */}
        <div 
          className="flex-1 border-r border-dark-700 pr-4 flex flex-col min-h-0"
          onWheel={handleWheel}
        >
          <div className="overflow-y-auto flex-1 custom-scrollbar pr-2">
          <table className="w-full text-left text-sm whitespace-nowrap">
            <thead className="text-gray-500 pb-2 border-b border-dark-700">
              <tr>
                <th className="font-normal py-2">Timestamp</th>
                <th className="font-normal py-2">Temp (°C)</th>
                <th className="font-normal py-2">Volt (V)</th>
                <th className="font-normal py-2">Status</th>
              </tr>
            </thead>
            <tbody className="text-gray-300 font-mono">
              {windowLogs.map((log, i) => (
                <tr key={i} className={`border-b border-dark-700 ${log.has_crc_error ? 'text-brand-red bg-red-900 bg-opacity-20' : ''}`}>
                  <td className="py-2">{new Date(log.receive_timestamp).toLocaleTimeString()}</td>
                  <td className="py-2">{log.temperature.toFixed(2)}</td>
                  <td className="py-2">{log.voltage.toFixed(2)}</td>
                  <td className={`py-2 font-bold ${log.has_crc_error ? 'text-brand-red' : (log.mode === 'NOMINAL' ? 'text-brand-green' : 'text-brand-yellow')}`}>
                    {log.has_crc_error ? 'CRC ERROR' : log.mode}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          </div>
        </div>

        {/* Replay Controls Placeholder */}
        <div className="w-1/3 flex flex-col justify-center items-center pl-4 space-y-4">
          <h3 className="text-xs text-gray-500 uppercase tracking-widest text-center">Replay Controls</h3>
          
          <div className="flex items-center space-x-3 bg-dark-900 px-4 py-2 rounded border border-dark-700">
             <button className="text-gray-400 hover:text-white transition-colors" title="Rewind">
              <Rewind size={18} />
             </button>
             <button className="text-white hover:text-brand-green transition-colors" title="Play">
              <Play size={20} fill="currentColor" />
             </button>
             <button className="text-gray-400 hover:text-white transition-colors" title="Pause">
              <Pause size={18} />
             </button>
             <button className="text-gray-400 hover:text-white transition-colors" title="Step Forward">
              <SkipForward size={18} />
             </button>
          </div>

          <div className="w-full flex items-center space-x-2">
            <span className="text-xs text-gray-500 font-mono">00:00</span>
            <input type="range" className="flex-1 accent-brand-green" min={0} max={100} defaultValue={0} disabled />
            <select className="bg-dark-900 border border-dark-700 text-xs text-white rounded px-2 py-1 outline-none">
              <option>1x Speed</option>
              <option>2x</option>
              <option>0.5x</option>
            </select>
          </div>
        </div>
      </div>
    </div>
  );
};
