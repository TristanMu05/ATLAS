import React from "react";
import { useTelemetry } from "../contexts/TelemetryProvider";

export const TelemetryPanel: React.FC = () => {
  const { currentPacket, stats } = useTelemetry();

  if (!currentPacket || !stats) {
    return <div className="text-gray-500">Waiting for telemetry...</div>;
  }

  // Helper for mode styles
  const getModeColor = (mode: string) => {
    switch(mode) {
      case "NOMINAL": return "text-brand-green border-brand-green";
      case "SAFE": return "text-gray-400 border-gray-400";
      case "WARNING": return "text-brand-yellow border-brand-yellow";
      case "DIAGNOSTIC": return "text-blue-400 border-blue-400";
      default: return "text-brand-red border-brand-red";
    }
  };

  const ModeDisplay = () => (
    <div className="flex items-center space-x-2 border-r border-dark-700 pr-6">
      <span className="text-gray-400 text-sm uppercase tracking-wider">Mode:</span>
      <span className={`font-bold tracking-widest pl-2 border-l-2 ${getModeColor(currentPacket.mode)}`}>
        {currentPacket.mode}
      </span>
    </div>
  );

  const StatItem = ({ label, value, colorClass = "text-white" }: { label: string, value: string | number, colorClass?: string }) => (
    <div className="flex flex-col border border-dark-700 bg-dark-800 rounded px-4 py-2 w-full">
      <span className="text-xs text-gray-400 mb-1">{label}</span>
      <span className={`text-2xl font-semibold tracking-wider ${colorClass}`}>{value}</span>
    </div>
  );

  // Status indicator
  const tempColor = currentPacket.temperature > 50 ? "text-brand-red" : currentPacket.temperature > 40 ? "text-brand-yellow" : "text-white";
  const voltColor = currentPacket.voltage < 3.1 ? "text-brand-red" : "text-white";

  return (
    <div className="flex flex-col space-y-4 w-full">
      {/* Top Header Bar */}
      <div className="flex flex-row items-center justify-between border-b border-dark-700 pb-4">
        <ModeDisplay />
        <div className="flex space-x-6">
          <div className="flex items-center space-x-2">
            <span className="text-gray-400 text-sm">Link:</span>
            <span className={`text-sm font-bold ${stats.linkStatus === 'CONNECTED' ? 'text-brand-green' : 'text-brand-red'}`}>
              {stats.linkStatus}
            </span>
          </div>
          <div className="flex items-center space-x-2">
            <span className="text-gray-400 text-sm">Packets:</span>
            <span className="text-sm text-white">{stats.packetRateHz} Hz</span>
          </div>
          <div className="flex items-center space-x-2">
            <span className="text-gray-400 text-sm">Seq:</span>
            <span className="text-sm font-mono text-white">{currentPacket.sequence}</span>
          </div>
          <div className="flex items-center space-x-2">
            <span className="text-gray-400 text-sm">Uptime:</span>
            <span className="text-sm font-mono text-white">{new Date(stats.uptimeSeconds * 1000).toISOString().substr(11, 8)}</span>
          </div>
        </div>
      </div>

      {/* Main Metric Cards */}
      <div className="grid grid-cols-4 gap-4">
        <StatItem label="Temperature" value={`${currentPacket.temperature.toFixed(1)} °C`} colorClass={tempColor} />
        <StatItem label="Voltage" value={`${currentPacket.voltage.toFixed(2)} V`} colorClass={voltColor} />
        <StatItem label="Packet Rate" value={`${stats.packetRateHz} Hz`} />
        <StatItem label="Sequence" value={`# ${currentPacket.sequence}`} />
      </div>
    </div>
  );
};
