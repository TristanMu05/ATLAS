import React from "react";
import { useTelemetry } from "../contexts/TelemetryProvider";
import { AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from "recharts";

export const GraphView: React.FC = () => {
  const { isLockedToFront, setIsLockedToFront, visibleData, panHistory } = useTelemetry();

  const handleWheel = (e: React.WheelEvent<HTMLDivElement>) => {
    // A standard mouse wheel up (scroll up) gives negative deltaY. Let's map scrolling up/left to going back in time.
    const delta = e.deltaY !== 0 ? e.deltaY : e.deltaX;
    panHistory(delta);
  };


  // Create a formatter for the XAxis (time elapsed in seconds)
  const formatTime = (ms: number) => `${Math.floor(ms / 1000)}s`;

  return (
    <div className="flex space-x-4 w-full h-[300px] mt-4">
      {/* Temperature Graph */}
      <div className="flex-1 bg-dark-800 border border-dark-700 rounded p-4 flex flex-col relative">
        <div className="flex justify-between items-center mb-2">
          <h3 className="text-gray-400 text-sm font-semibold tracking-wider">Temperature History (°C)</h3>
          {!isLockedToFront && (
            <button 
              onClick={() => setIsLockedToFront(true)}
              className="px-2 py-1 text-xs bg-brand-blue/20 text-brand-blue rounded hover:bg-brand-blue hover:text-white transition-colors"
            >
              Resume Live
            </button>
          )}
        </div>
        
        <div className="flex-1 w-full min-h-0" onWheel={handleWheel}>
          <ResponsiveContainer width="100%" height="100%">
            <AreaChart data={visibleData} margin={{ top: 5, right: 0, left: -20, bottom: 0 }}>
              <defs>
                <linearGradient id="colorTemp" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#eab308" stopOpacity={0.3}/>
                  <stop offset="95%" stopColor="#eab308" stopOpacity={0}/>
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="#374151" vertical={false} />
              <XAxis dataKey="timestamp" tickFormatter={formatTime} stroke="#9ca3af" fontSize={12} minTickGap={30} />
              <YAxis domain={['auto', 'auto']} stroke="#9ca3af" fontSize={12} />
              <Tooltip 
                contentStyle={{ backgroundColor: '#1f2937', borderColor: '#374151' }}
                itemStyle={{ color: '#eab308' }}
                labelFormatter={(v) => `Time: ${formatTime(v as number)}`}
              />
              <Area type="monotone" dataKey="temperature" stroke="#eab308" fillOpacity={1} fill="url(#colorTemp)" isAnimationActive={false} />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      </div>

      {/* Voltage Graph */}
      <div className="flex-1 bg-dark-800 border border-dark-700 rounded p-4 flex flex-col relative">
        <h3 className="text-gray-400 text-sm mb-2 font-semibold tracking-wider">Voltage History (V)</h3>
        <div className="flex-1 w-full min-h-0" onWheel={handleWheel}>
          <ResponsiveContainer width="100%" height="100%">
            <AreaChart data={visibleData} margin={{ top: 5, right: 0, left: -20, bottom: 0 }}>
              <defs>
                <linearGradient id="colorVolt" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#3b82f6" stopOpacity={0.3}/>
                  <stop offset="95%" stopColor="#3b82f6" stopOpacity={0}/>
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="#374151" vertical={false} />
              <XAxis dataKey="timestamp" tickFormatter={formatTime} stroke="#9ca3af" fontSize={12} minTickGap={30} />
              <YAxis domain={[3.0, 3.5]} stroke="#9ca3af" fontSize={12} />
              <Tooltip 
                contentStyle={{ backgroundColor: '#1f2937', borderColor: '#374151' }}
                itemStyle={{ color: '#3b82f6' }}
                labelFormatter={(v) => `Time: ${formatTime(v as number)}`}
              />
              <Area type="monotone" dataKey="voltage" stroke="#3b82f6" fillOpacity={1} fill="url(#colorVolt)" isAnimationActive={false} />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      </div>
    </div>
  );
};
